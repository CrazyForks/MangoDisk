use std::{
    collections::HashSet,
    path::Path,
    time::{Duration, Instant},
};

use sha2::{Digest, Sha256};

use crate::{
    cleanup::{
        CleanupActionKind, CleanupActionResult, CleanupExecutionProgress,
        CleanupExecutionRuleResult, CleanupExecutionStage, CleanupRequest, CleanupResult,
    },
    history::{summarize_deep_cleanup, CleanupOperationDetails, DeepCleanupOperationDetails},
};

use crate::{
    applications::catalog::{ProcessSnapshot, ScanContext},
    cleanup::applicability::{evaluate_rule, rule_requires_process, Applicability},
    cleanup::cleaners,
    cleanup::rule_execution::{
        cancelled_action, execute_rule, measure_owned_rule, DeleteStats, RuleExecutionContext,
    },
    cleanup::rules::{compile_scan_plan, registry},
    cleanup::source_selection::SourceSelectionPolicy,
    filesystem::metadata::now_ms,
    history::HistoryService,
    shared::{
        operation::{CoordinatedOperationKind, OperationCancellationToken, OperationGuard},
        CoreError, CoreResult,
    },
};

#[cfg(test)]
use std::fs;

#[cfg(test)]
use crate::cleanup::{
    rule_execution::{
        delete_root_contents, delete_root_contents_with_progress, validate_rule_root,
    },
    rules::{CompiledRule, MatcherSpec},
};

pub struct CleanupService;

const ITEM_PROGRESS_INTERVAL: Duration = Duration::from_millis(80);

struct ExecutionProgressReporter<F> {
    handler: F,
    started: Instant,
    planned_rule_ids: Vec<String>,
    total_rule_count: u64,
    validated_rule_count: u64,
    completed_rule_count: u64,
    checked_item_count: u64,
    checked_bytes: u64,
    affected_item_count: u64,
    released_bytes: u64,
    current_item_path: Option<String>,
    current_rule_affected_item_count: u64,
    current_rule_released_bytes: u64,
    completed_rule_results: Vec<CleanupExecutionRuleResult>,
    last_item_emit: Option<Instant>,
}

impl<F> ExecutionProgressReporter<F>
where
    F: FnMut(CleanupExecutionProgress),
{
    fn new(planned_rule_ids: Vec<String>, handler: F) -> Self {
        let total_rule_count = planned_rule_ids.len();
        Self {
            handler,
            started: Instant::now(),
            planned_rule_ids,
            total_rule_count: total_rule_count as u64,
            validated_rule_count: 0,
            completed_rule_count: 0,
            checked_item_count: 0,
            checked_bytes: 0,
            affected_item_count: 0,
            released_bytes: 0,
            current_item_path: None,
            current_rule_affected_item_count: 0,
            current_rule_released_bytes: 0,
            completed_rule_results: Vec::with_capacity(total_rule_count),
            last_item_emit: None,
        }
    }

    fn emit(&mut self, stage: CleanupExecutionStage, current_rule_id: Option<&str>) {
        (self.handler)(CleanupExecutionProgress {
            stage,
            planned_rule_ids: self.planned_rule_ids.clone(),
            current_rule_id: current_rule_id.map(str::to_owned),
            current_item_path: self.current_item_path.clone(),
            current_rule_affected_item_count: self.current_rule_affected_item_count,
            current_rule_released_bytes: self.current_rule_released_bytes,
            completed_rule_results: self.completed_rule_results.clone(),
            validated_rule_count: self.validated_rule_count,
            completed_rule_count: self.completed_rule_count,
            total_rule_count: self.total_rule_count,
            checked_item_count: self.checked_item_count,
            checked_bytes: self.checked_bytes,
            affected_item_count: self
                .affected_item_count
                .saturating_add(self.current_rule_affected_item_count),
            released_bytes: self
                .released_bytes
                .saturating_add(self.current_rule_released_bytes),
            elapsed_ms: self.started.elapsed().as_millis() as u64,
        });
    }

    fn record_validation(&mut self, item_count: u64, bytes: u64) {
        self.validated_rule_count = self
            .validated_rule_count
            .saturating_add(1)
            .min(self.total_rule_count);
        self.checked_item_count = self.checked_item_count.saturating_add(item_count);
        self.checked_bytes = self.checked_bytes.saturating_add(bytes);
    }

    fn finish_validation(&mut self) {
        // When a measurement stage exists, rules without a generic filesystem
        // measurement are already ready to execute or own their specialized
        // validation. Complete the stage without presenting those rules as
        // missing file checks.
        self.validated_rule_count = self.total_rule_count;
    }

    fn begin_rule(&mut self) {
        self.current_item_path = None;
        self.current_rule_affected_item_count = 0;
        self.current_rule_released_bytes = 0;
        self.last_item_emit = None;
    }

    fn record_item(&mut self, rule_id: &str, path: &Path, stats: &DeleteStats) {
        self.current_rule_affected_item_count = stats.affected_item_count;
        self.current_rule_released_bytes = stats.deleted_bytes;
        let now = Instant::now();
        if self
            .last_item_emit
            .is_some_and(|last_emit| now.duration_since(last_emit) < ITEM_PROGRESS_INTERVAL)
        {
            return;
        }
        // Path conversion allocates on both Windows and macOS. Do it only for
        // snapshots that will actually cross the adapter boundary; deletion
        // may otherwise pay this cost tens of thousands of times per rule.
        self.current_item_path = Some(path.to_string_lossy().into_owned());
        self.last_item_emit = Some(now);
        self.emit(CleanupExecutionStage::Cleaning, Some(rule_id));
    }

    fn record_action(&mut self, action: &CleanupActionResult) {
        self.current_item_path = None;
        self.current_rule_affected_item_count = 0;
        self.current_rule_released_bytes = 0;
        self.last_item_emit = None;
        self.completed_rule_count = self
            .completed_rule_count
            .saturating_add(1)
            .min(self.total_rule_count);
        self.affected_item_count = self
            .affected_item_count
            .saturating_add(action.affected_item_count);
        self.released_bytes = self.released_bytes.saturating_add(action.released_bytes);
        self.completed_rule_results
            .push(CleanupExecutionRuleResult {
                rule_id: action.rule_id.clone(),
                status: action.status,
                affected_item_count: action.affected_item_count,
                released_bytes: action.released_bytes,
            });
    }
}

impl CleanupService {
    /// Requests cooperative cancellation of the active cleanup execution.
    ///
    /// Files that were already removed remain reflected in the result and
    /// history. Long-running platform commands may finish their current native
    /// step before observing the token, but no later cleanup rule is started.
    pub fn cancel() {
        OperationCancellationToken::cleanup().cancel();
    }

    pub fn execute(request: CleanupRequest) -> CoreResult<CleanupResult> {
        Self::execute_with_progress(request, |_| {})
    }

    pub fn execute_with_progress<F>(
        request: CleanupRequest,
        progress: F,
    ) -> CoreResult<CleanupResult>
    where
        F: FnMut(CleanupExecutionProgress),
    {
        let operation_id = format!("deep-cleanup-{}", now_ms());
        Self::execute_deep_cleanup_step_with_progress(request, operation_id, progress)
    }

    pub fn execute_deep_cleanup_step(
        request: CleanupRequest,
        deep_cleanup_operation_id: String,
    ) -> CoreResult<CleanupResult> {
        Self::execute_deep_cleanup_step_with_progress(request, deep_cleanup_operation_id, |_| {})
    }

    pub fn execute_deep_cleanup_step_with_progress<F>(
        request: CleanupRequest,
        deep_cleanup_operation_id: String,
        progress: F,
    ) -> CoreResult<CleanupResult>
    where
        F: FnMut(CleanupExecutionProgress),
    {
        let operation = OperationGuard::start(CoordinatedOperationKind::Cleanup)?;
        if request.rule_ids.is_empty() {
            return Err(CoreError::invalid_input(
                "at least one cleanup rule must be selected",
            ));
        }
        if deep_cleanup_operation_id.trim().is_empty() {
            return Err(CoreError::invalid_input(
                "deep cleanup operation id must not be empty",
            ));
        }
        let started_at_ms = now_ms();
        let started = Instant::now();
        let selected = request.rule_ids.iter().cloned().collect::<HashSet<_>>();
        if selected.len() != request.rule_ids.len() {
            return Err(CoreError::invalid_input(
                "the cleanup plan contains duplicate rules",
            ));
        }
        let source_selection_policy =
            SourceSelectionPolicy::from_request(&selected, &request.source_selections)?;
        let rules = registry()?;
        if selected
            .iter()
            .any(|id| !rules.iter().any(|rule| rule.id == id.as_str()) && !cleaners::contains(id))
        {
            return Err(CoreError::invalid_input(
                "the cleanup plan contains an unknown rule",
            ));
        }
        let cleaner_rule_ids = request
            .rule_ids
            .iter()
            .filter(|id| cleaners::contains(id))
            .cloned()
            .collect::<Vec<_>>();
        // The execution pipeline validates and runs declarative filesystem
        // rules first, then specialized cleaners. Publish that deterministic
        // queue so adapters never mistake selection order for execution order.
        let mut planned_rule_ids = rules
            .iter()
            .filter(|rule| selected.contains(rule.id.as_str()))
            .map(|rule| rule.id.clone())
            .collect::<Vec<_>>();
        planned_rule_ids.extend(cleaners::execution_rule_ids(&cleaner_rule_ids));
        if planned_rule_ids.len() != request.rule_ids.len() {
            return Err(CoreError::operation_failed(
                "cleanup execution planning did not preserve every selected rule",
            ));
        }
        let mut progress = ExecutionProgressReporter::new(planned_rule_ids, progress);
        let validation_started = Instant::now();
        let applicability_context = ScanContext::capture();
        let applicability_process_snapshot = if rules.iter().any(rule_requires_process) {
            match ProcessSnapshot::capture() {
                Ok(snapshot) => Some(snapshot),
                Err(error) => {
                    log::warn!(
                        "cleanup_applicability_process_snapshot_failed error_digest={}",
                        blake3::hash(error.as_bytes()).to_hex()
                    );
                    None
                }
            }
        } else {
            None
        };
        let availability = rules
            .iter()
            .map(|rule| {
                evaluate_rule(
                    &applicability_context.inventory,
                    rule,
                    applicability_process_snapshot.as_ref(),
                ) != Applicability::NotApplicable
            })
            .collect::<Vec<_>>();
        // Recompile ownership for every applicable rule, not only selected
        // rules. An unselected child rule still protects its files from a
        // selected parent rule.
        let ownership_plan = compile_scan_plan(rules, &availability, &[])?;
        let selected_rule_indices = ownership_plan
            .rules
            .iter()
            .enumerate()
            .filter(|(_, rule)| selected.contains(rule.id.as_str()))
            .map(|(rule_index, _)| rule_index)
            .collect::<Vec<_>>();
        // A dry run has no deletion traversal from which to derive an estimate.
        // Source-scoped execution must also prove every untrusted UI path is a
        // live source before mutation. Ordinary whole-rule cleanup can instead
        // validate and account for each candidate during its deletion pass.
        let measured_rule_count = selected_rule_indices
            .iter()
            .filter(|rule_index| {
                requires_preflight_measurement(
                    request.dry_run,
                    source_selection_policy
                        .scope(&ownership_plan.rules[**rule_index].id)
                        .is_some(),
                )
            })
            .count();
        progress.emit(preparation_stage(measured_rule_count), None);
        let mut measured_rules = Vec::with_capacity(selected_rule_indices.len());
        for rule_index in &selected_rule_indices {
            operation.ensure_not_cancelled()?;
            let rule = &ownership_plan.rules[*rule_index];
            let requires_measurement = requires_preflight_measurement(
                request.dry_run,
                source_selection_policy.scope(&rule.id).is_some(),
            );
            if !requires_measurement {
                measured_rules.push(None);
                continue;
            }
            progress.emit(CleanupExecutionStage::Validating, Some(&rule.id));
            let measured = measure_owned_rule(
                &ownership_plan,
                *rule_index,
                source_selection_policy.scope(&rule.id),
            )?;
            progress.record_validation(measured.file_count, measured.bytes);
            progress.emit(CleanupExecutionStage::Validating, Some(&rule.id));
            measured_rules.push(Some(measured));
        }
        // No filesystem mutation has happened yet. Even executions that skip
        // measurement may still be cancelled cleanly before the first rule.
        operation.ensure_not_cancelled()?;
        // Capture one process snapshot immediately before deletion. Rules
        // share the system query instead of trusting process state from the
        // earlier result screen or a potentially long scoped measurement.
        let process_snapshot = if selected_rule_indices
            .iter()
            .any(|rule_index| ownership_plan.rules[*rule_index].requires_app_close())
        {
            ProcessSnapshot::capture()
                .map_err(|error| format!("failed to verify running applications: {error}"))?
        } else {
            ProcessSnapshot::default()
        };
        let validation_elapsed_ms = validation_started.elapsed().as_millis() as u64;
        log::info!(
            "cleanup_started operation_id={} ownership_plan_id={} rule_count={} filesystem_rule_count={} cleaner_rule_count={} measured_rule_count={} validation_elapsed_ms={} rule_ids={:?} dry_run={}",
            operation.id(),
            ownership_plan.plan_id,
            request.rule_ids.len(),
            selected_rule_indices.len(),
            cleaner_rule_ids.len(),
            measured_rule_count,
            validation_elapsed_ms,
            request.rule_ids,
            request.dry_run
        );

        let plan_id = format!("plan-{started_at_ms}");
        let plan_hash = plan_hash(
            &plan_id,
            &request.rule_ids,
            &request.project_roots,
            &request.source_selections,
            request.dry_run,
        );
        let mut actions = Vec::new();
        if measured_rule_count > 0 {
            progress.finish_validation();
        }
        for (rule_index, measured) in selected_rule_indices.into_iter().zip(measured_rules) {
            let rule = &ownership_plan.rules[rule_index];
            progress.begin_rule();
            progress.emit(CleanupExecutionStage::Cleaning, Some(&rule.id));
            let action = if operation.ensure_not_cancelled().is_err() {
                cancelled_action(
                    &rule.id,
                    CleanupActionKind::Delete,
                    measured.as_ref().map_or(0, |measurement| measurement.bytes),
                )
            } else {
                let mut report_item = |path: &Path, stats: &DeleteStats| {
                    progress.record_item(&rule.id, path, stats);
                };
                execute_rule(
                    rule,
                    rule_index,
                    measured,
                    &RuleExecutionContext {
                        ownership_plan: &ownership_plan,
                        process_snapshot: &process_snapshot,
                        source_scope: source_selection_policy.scope(&rule.id),
                        operation: &operation,
                        dry_run: request.dry_run,
                    },
                    &mut report_item,
                )
            };
            progress.record_action(&action);
            progress.emit(CleanupExecutionStage::Cleaning, Some(&rule.id));
            actions.push(action);
        }
        let active_rule_roots = ownership_plan.active_rule_roots();
        actions.extend(cleaners::execute_selected_with_progress(
            cleaners::CleanerExecutionRequest {
                rule_ids: &cleaner_rule_ids,
                inventory: &applicability_context.inventory,
                declared_roots: &active_rule_roots,
                project_roots: &request.project_roots,
                source_selections: &source_selection_policy,
                dry_run: request.dry_run,
                operation: &operation,
            },
            |rule_id, action| {
                if let Some(action) = action {
                    progress.record_action(action);
                } else {
                    progress.begin_rule();
                }
                progress.emit(CleanupExecutionStage::Cleaning, Some(rule_id));
            },
        ));
        let expected_bytes = actions.iter().map(|action| action.bytes_expected).sum();
        let released_bytes = actions.iter().map(|action| action.released_bytes).sum();
        let affected_item_count = actions
            .iter()
            .map(|action| action.affected_item_count)
            .sum();
        let failed_item_count = actions.iter().map(|action| action.failed_item_count).sum();
        progress.emit(CleanupExecutionStage::Finalizing, None);
        let record = summarize_deep_cleanup(
            deep_cleanup_operation_id,
            started_at_ms,
            now_ms(),
            request.dry_run,
            DeepCleanupOperationDetails {
                cleanup: Some(CleanupOperationDetails {
                    selected_rule_ids: request.rule_ids,
                    expected_bytes,
                    actions: actions.clone(),
                }),
                application_leftovers: None,
            },
        );
        // History is an auxiliary audit capability after deletion has occurred.
        // A persistence failure must not report an irreversible successful
        // operation as failed; structured state and logs preserve the failure.
        let history_saved = match HistoryService::upsert_deep_cleanup(record.clone()) {
            Ok(()) => true,
            Err(error) => {
                log::warn!(
                    "cleanup_history_save_failed operation_id={} run_id={} error_digest={}",
                    operation.id(),
                    record.operation_id,
                    blake3::hash(error.diagnostic().as_bytes()).to_hex()
                );
                false
            }
        };
        log::info!(
            "cleanup_finished operation_id={} status={} expected_bytes={} released_bytes={} affected_item_count={} failed_item_count={} history_saved={} elapsed_ms={}",
            operation.id(),
            record.outcome.as_str(),
            expected_bytes,
            released_bytes,
            affected_item_count,
            failed_item_count,
            history_saved,
            started.elapsed().as_millis()
        );
        operation.complete();
        Ok(CleanupResult {
            plan_id,
            plan_hash,
            expected_bytes,
            released_bytes,
            affected_item_count,
            failed_item_count,
            dry_run: request.dry_run,
            actions,
            record,
            history_saved,
        })
    }
}

/// Limits the expensive read-only traversal to cases that cannot safely derive
/// their result from the destructive pass. A preview has no destructive pass,
/// while a source-scoped request must authenticate untrusted UI paths before
/// any selected file is removed.
fn requires_preflight_measurement(dry_run: bool, has_source_scope: bool) -> bool {
    dry_run || has_source_scope
}

fn preparation_stage(measured_rule_count: usize) -> CleanupExecutionStage {
    if measured_rule_count == 0 {
        CleanupExecutionStage::Cleaning
    } else {
        CleanupExecutionStage::Validating
    }
}

fn plan_hash(
    plan_id: &str,
    rule_ids: &[String],
    project_roots: &[String],
    source_selections: &[crate::cleanup::CleanupSourceSelection],
    dry_run: bool,
) -> String {
    let mut normalized = rule_ids.to_vec();
    normalized.sort();
    let mut normalized_roots = project_roots.to_vec();
    normalized_roots.sort();
    let mut hasher = Sha256::new();
    update_hash_field(&mut hasher, plan_id.as_bytes());
    update_hash_field(&mut hasher, if dry_run { b"dry-run" } else { b"apply" });
    for id in normalized {
        update_hash_field(&mut hasher, id.as_bytes());
    }
    for root in normalized_roots {
        update_hash_field(&mut hasher, root.as_bytes());
    }
    let mut normalized_sources = source_selections.to_vec();
    normalized_sources.sort_by(|left, right| left.rule_id.cmp(&right.rule_id));
    for selection in normalized_sources {
        update_hash_field(&mut hasher, selection.rule_id.as_bytes());
        update_hash_field(
            &mut hasher,
            match selection.mode {
                crate::cleanup::CleanupSourceSelectionMode::Include => b"include",
                crate::cleanup::CleanupSourceSelectionMode::Exclude => b"exclude",
            },
        );
        let mut paths = selection.paths;
        paths.sort();
        for path in paths {
            update_hash_field(&mut hasher, path.as_bytes());
        }
    }
    hasher
        .finalize()
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}

fn update_hash_field(hasher: &mut Sha256, value: &[u8]) {
    hasher.update((value.len() as u64).to_le_bytes());
    hasher.update(value);
}

#[cfg(test)]
mod cleanup_matcher_tests {
    use std::path::PathBuf;

    use super::*;

    #[test]
    fn preflight_measurement_is_limited_to_preview_and_source_scoped_requests() {
        assert!(!requires_preflight_measurement(false, false));
        assert!(requires_preflight_measurement(true, false));
        assert!(requires_preflight_measurement(false, true));
        assert!(requires_preflight_measurement(true, true));
        assert_eq!(preparation_stage(0), CleanupExecutionStage::Cleaning);
        assert_eq!(preparation_stage(1), CleanupExecutionStage::Validating);
    }

    #[test]
    fn cleanup_service_cancels_the_active_cleanup_operation() {
        let _operation_lock = crate::shared::operation::test_operation_lock();
        let operation = OperationGuard::start(CoordinatedOperationKind::Cleanup)
            .expect("the isolated cleanup operation must start");

        CleanupService::cancel();

        assert!(
            operation.ensure_not_cancelled().is_err(),
            "the public cleanup cancellation contract must reach the active operation"
        );
    }

    #[test]
    fn execution_progress_preserves_stage_order_and_final_totals() {
        let mut snapshots = Vec::new();
        let action = CleanupActionResult {
            rule_id: "system.fixture".to_string(),
            action_kind: CleanupActionKind::Delete,
            status: crate::cleanup::CleanupActionStatus::Completed,
            reason_code: None,
            bytes_expected: 120,
            released_bytes: 96,
            affected_item_count: 2,
            failed_item_count: 0,
            running_processes: Vec::new(),
        };
        {
            let mut reporter =
                ExecutionProgressReporter::new(vec!["system.fixture".to_string()], |progress| {
                    snapshots.push(progress)
                });
            reporter.emit(CleanupExecutionStage::Validating, None);
            reporter.record_validation(3, 120);
            reporter.emit(CleanupExecutionStage::Validating, Some("system.fixture"));
            reporter.finish_validation();
            reporter.emit(CleanupExecutionStage::Cleaning, Some(&action.rule_id));
            reporter.record_action(&action);
            reporter.emit(CleanupExecutionStage::Cleaning, Some(&action.rule_id));
            reporter.emit(CleanupExecutionStage::Finalizing, None);
        }

        let stages = snapshots
            .iter()
            .map(|progress| progress.stage)
            .collect::<Vec<_>>();
        assert_eq!(
            stages,
            vec![
                CleanupExecutionStage::Validating,
                CleanupExecutionStage::Validating,
                CleanupExecutionStage::Cleaning,
                CleanupExecutionStage::Cleaning,
                CleanupExecutionStage::Finalizing,
            ]
        );
        assert!(snapshots
            .iter()
            .all(|progress| progress.completed_rule_count <= progress.total_rule_count));
        assert!(snapshots
            .iter()
            .all(|progress| progress.planned_rule_ids.as_slice() == ["system.fixture"]));
        let final_snapshot = snapshots.last().expect("final progress must be emitted");
        let serialized = serde_json::to_value(final_snapshot)
            .expect("cleanup execution progress must serialize for desktop events");
        assert_eq!(serialized["plannedRuleIds"][0], "system.fixture");
        assert_eq!(
            final_snapshot.affected_item_count,
            action.affected_item_count
        );
        assert_eq!(final_snapshot.released_bytes, action.released_bytes);
    }

    #[test]
    fn execution_progress_reports_live_items_and_completed_rule_results() {
        let mut snapshots = Vec::new();
        let action = CleanupActionResult {
            rule_id: "system.fixture".to_string(),
            action_kind: CleanupActionKind::Delete,
            status: crate::cleanup::CleanupActionStatus::Completed,
            reason_code: None,
            bytes_expected: 120,
            released_bytes: 64,
            affected_item_count: 1,
            failed_item_count: 0,
            running_processes: Vec::new(),
        };
        {
            let mut reporter =
                ExecutionProgressReporter::new(vec![action.rule_id.clone()], |progress| {
                    snapshots.push(progress)
                });
            reporter.begin_rule();
            reporter.record_item(
                &action.rule_id,
                Path::new("fixture/cache.tmp"),
                &DeleteStats {
                    matched_bytes: 64,
                    deleted_bytes: 64,
                    affected_item_count: 1,
                    failed_item_count: 0,
                },
            );
            reporter.record_action(&action);
            reporter.emit(CleanupExecutionStage::Cleaning, Some(&action.rule_id));
        }

        let live_snapshot = snapshots
            .first()
            .expect("the first deleted item must produce live progress");
        assert_eq!(
            live_snapshot.current_item_path.as_deref(),
            Some("fixture/cache.tmp")
        );
        assert_eq!(live_snapshot.current_rule_affected_item_count, 1);
        assert_eq!(live_snapshot.current_rule_released_bytes, 64);
        assert_eq!(live_snapshot.affected_item_count, 1);
        assert_eq!(live_snapshot.released_bytes, 64);

        let completed_snapshot = snapshots
            .last()
            .expect("the completed rule must produce a summary");
        assert_eq!(completed_snapshot.current_item_path, None);
        assert_eq!(completed_snapshot.completed_rule_count, 1);
        assert_eq!(completed_snapshot.completed_rule_results.len(), 1);
        assert_eq!(
            completed_snapshot.completed_rule_results[0].rule_id,
            action.rule_id
        );
        assert_eq!(
            completed_snapshot.completed_rule_results[0].status,
            action.status
        );
        assert_eq!(
            completed_snapshot.completed_rule_results[0].affected_item_count,
            action.affected_item_count
        );
        assert_eq!(
            completed_snapshot.completed_rule_results[0].released_bytes,
            action.released_bytes
        );
    }

    #[test]
    fn whole_rule_cleanup_derives_expected_bytes_during_single_pass() {
        let _operation_lock = crate::shared::operation::test_operation_lock();
        let sandbox = std::env::temp_dir().join(format!(
            "mangodisk-cleanup-single-pass-test-{}-{}",
            std::process::id(),
            now_ms()
        ));
        let _sandbox_cleanup = DirectoryCleanup(sandbox.clone());
        let cleanup_root = sandbox.join("cache");
        let cache_file = cleanup_root.join("generated.tmp");
        let cache_bytes = b"single-pass cleanup fixture";
        fs::create_dir_all(&cleanup_root).expect("the isolated cleanup root must be created");
        fs::write(&cache_file, cache_bytes).expect("the cleanup fixture must be written");
        let plan = compile_scan_plan(
            vec![CompiledRule::fixture(
                "system.single-pass-fixture",
                cleanup_root,
                crate::cleanup::CleanupCategory::System,
                MatcherSpec::ExtensionIn(vec!["tmp".to_string()]),
            )],
            &[true],
            &[],
        )
        .expect("the isolated rule must compile");
        let process_snapshot = ProcessSnapshot::default();
        let operation = OperationGuard::start(CoordinatedOperationKind::Cleanup)
            .expect("the isolated cleanup operation must start");

        let action = execute_rule(
            &plan.rules[0],
            0,
            None,
            &RuleExecutionContext {
                ownership_plan: &plan,
                process_snapshot: &process_snapshot,
                source_scope: None,
                operation: &operation,
                dry_run: false,
            },
            &mut |_, _| {},
        );

        operation.complete();
        assert_eq!(
            action.status,
            crate::cleanup::CleanupActionStatus::Completed
        );
        assert_eq!(action.bytes_expected, cache_bytes.len() as u64);
        assert_eq!(action.released_bytes, cache_bytes.len() as u64);
        assert_eq!(action.affected_item_count, 1);
        assert!(!cache_file.exists());
    }

    #[test]
    fn whole_root_cleanup_reduces_per_file_deletion_transactions() {
        let _operation_lock = crate::shared::operation::test_operation_lock();
        let sandbox = std::env::temp_dir().join(format!(
            "mangodisk-cleanup-whole-root-test-{}-{}",
            std::process::id(),
            now_ms()
        ));
        let _sandbox_cleanup = DirectoryCleanup(sandbox.clone());
        let cleanup_root = sandbox.join("cache");
        let nested = cleanup_root.join("many-small-files");
        let generic_root = sandbox.join("generic-cache");
        let generic_nested = generic_root.join("many-small-files");
        fs::create_dir_all(&nested).expect("the isolated cleanup root must be created");
        fs::create_dir_all(&generic_nested).expect("the generic comparison root must be created");
        let file_count = 128_u64;
        for index in 0..file_count {
            fs::write(nested.join(format!("{index}.cache")), b"cache")
                .expect("the small cache fixture must be written");
            fs::write(generic_nested.join(format!("{index}.cache")), b"cache")
                .expect("the comparison cache fixture must be written");
        }
        let whole_root_plan = compile_scan_plan(
            vec![CompiledRule::whole_root_fixture(
                "development.whole-root-fixture",
                cleanup_root.clone(),
                crate::cleanup::CleanupCategory::Development,
            )],
            &[true],
            &[],
        )
        .expect("the isolated whole-root rule must compile");
        let generic_plan = compile_scan_plan(
            vec![CompiledRule::fixture(
                "development.generic-fixture",
                generic_root,
                crate::cleanup::CleanupCategory::Development,
                MatcherSpec::All,
            )],
            &[true],
            &[],
        )
        .expect("the generic comparison rule must compile");
        let process_snapshot = ProcessSnapshot::default();
        let operation = OperationGuard::start(CoordinatedOperationKind::Cleanup)
            .expect("the isolated cleanup operation must start");
        let mut reported_paths = Vec::new();
        let mut generic_report_count = 0_u64;

        let generic_action = execute_rule(
            &generic_plan.rules[0],
            0,
            None,
            &RuleExecutionContext {
                ownership_plan: &generic_plan,
                process_snapshot: &process_snapshot,
                source_scope: None,
                operation: &operation,
                dry_run: false,
            },
            &mut |_, _| generic_report_count = generic_report_count.saturating_add(1),
        );

        let action = execute_rule(
            &whole_root_plan.rules[0],
            0,
            None,
            &RuleExecutionContext {
                ownership_plan: &whole_root_plan,
                process_snapshot: &process_snapshot,
                source_scope: None,
                operation: &operation,
                dry_run: false,
            },
            &mut |path, _| reported_paths.push(path.to_path_buf()),
        );

        operation.complete();
        assert_eq!(
            generic_action.status,
            crate::cleanup::CleanupActionStatus::Completed
        );
        assert_eq!(generic_report_count, file_count);
        assert_eq!(
            action.status,
            crate::cleanup::CleanupActionStatus::Completed
        );
        assert_eq!(action.bytes_expected, file_count * 5);
        assert_eq!(action.released_bytes, file_count * 5);
        assert_eq!(action.affected_item_count, file_count);
        assert_eq!(reported_paths, vec![cleanup_root.clone()]);
        assert!(
            !cleanup_root.exists(),
            "the complete cache root must be removed"
        );
    }

    /// Compares per-file and whole-root strategies through production boundaries.
    ///
    /// The benchmark is ignored by default to keep normal tests independent of
    /// disk variance. `MANGODISK_CLEANUP_BENCHMARK_FILE_COUNT` controls the
    /// workload; output contains only counts and timings, never private paths.
    #[test]
    #[ignore = "filesystem performance benchmark"]
    fn benchmark_whole_root_cleanup_against_per_file_cleanup() {
        let _operation_lock = crate::shared::operation::test_operation_lock();
        let file_count = std::env::var("MANGODISK_CLEANUP_BENCHMARK_FILE_COUNT")
            .ok()
            .and_then(|value| value.parse::<u64>().ok())
            .filter(|count| *count > 0)
            .unwrap_or(5_000);
        let sandbox = std::env::temp_dir().join(format!(
            "mangodisk-cleanup-whole-root-benchmark-{}-{}",
            std::process::id(),
            now_ms()
        ));
        let _sandbox_cleanup = DirectoryCleanup(sandbox.clone());
        let generic_root = sandbox.join("generic-cache");
        let whole_root = sandbox.join("whole-root-cache");
        fs::create_dir_all(&generic_root).expect("create the generic benchmark root");
        fs::create_dir_all(&whole_root).expect("create the whole-root benchmark root");
        let payload = [b'x'; 64];
        for index in 0..file_count {
            let bucket = format!("{:03}", index % 128);
            let generic_bucket = generic_root.join(&bucket);
            let whole_bucket = whole_root.join(&bucket);
            fs::create_dir_all(&generic_bucket).expect("create the generic benchmark bucket");
            fs::create_dir_all(&whole_bucket).expect("create the whole-root benchmark bucket");
            let name = format!("{index:08}.cache");
            fs::write(generic_bucket.join(&name), payload)
                .expect("write the generic benchmark file");
            fs::write(whole_bucket.join(name), payload)
                .expect("write the whole-root benchmark file");
        }
        let generic_plan = compile_scan_plan(
            vec![CompiledRule::fixture(
                "development.generic-benchmark",
                generic_root,
                crate::cleanup::CleanupCategory::Development,
                MatcherSpec::All,
            )],
            &[true],
            &[],
        )
        .expect("compile the generic benchmark plan");
        let whole_root_plan = compile_scan_plan(
            vec![CompiledRule::whole_root_fixture(
                "development.whole-root-benchmark",
                whole_root,
                crate::cleanup::CleanupCategory::Development,
            )],
            &[true],
            &[],
        )
        .expect("compile the whole-root benchmark plan");
        let process_snapshot = ProcessSnapshot::default();
        let operation = OperationGuard::start(CoordinatedOperationKind::Cleanup)
            .expect("start the benchmark cleanup operation");

        let generic_started = Instant::now();
        let generic_action = execute_rule(
            &generic_plan.rules[0],
            0,
            None,
            &RuleExecutionContext {
                ownership_plan: &generic_plan,
                process_snapshot: &process_snapshot,
                source_scope: None,
                operation: &operation,
                dry_run: false,
            },
            &mut |_, _| {},
        );
        let generic_ms = generic_started.elapsed().as_secs_f64() * 1_000.0;

        let whole_started = Instant::now();
        let whole_action = execute_rule(
            &whole_root_plan.rules[0],
            0,
            None,
            &RuleExecutionContext {
                ownership_plan: &whole_root_plan,
                process_snapshot: &process_snapshot,
                source_scope: None,
                operation: &operation,
                dry_run: false,
            },
            &mut |_, _| {},
        );
        let whole_ms = whole_started.elapsed().as_secs_f64() * 1_000.0;
        operation.complete();

        assert_eq!(
            generic_action.status,
            crate::cleanup::CleanupActionStatus::Completed
        );
        assert_eq!(
            whole_action.status,
            crate::cleanup::CleanupActionStatus::Completed
        );
        assert_eq!(generic_action.affected_item_count, file_count);
        assert_eq!(whole_action.affected_item_count, file_count);
        println!(
            "cleanup_whole_root_benchmark file_count={file_count} generic_ms={generic_ms:.2} whole_root_ms={whole_ms:.2} speedup={:.2}",
            generic_ms / whole_ms.max(0.01)
        );
    }

    #[test]
    fn whole_root_cleanup_falls_back_for_nested_rule_ownership() {
        let _operation_lock = crate::shared::operation::test_operation_lock();
        let sandbox = std::env::temp_dir().join(format!(
            "mangodisk-cleanup-whole-root-fallback-test-{}-{}",
            std::process::id(),
            now_ms()
        ));
        let _sandbox_cleanup = DirectoryCleanup(sandbox.clone());
        let cleanup_root = sandbox.join("cache");
        let child_root = cleanup_root.join("owned-by-child");
        let parent_file = cleanup_root.join("parent.cache");
        let child_file = child_root.join("child.cache");
        fs::create_dir_all(&child_root).expect("the nested rule root must be created");
        fs::write(&parent_file, b"parent cache").expect("the parent fixture must be written");
        fs::write(&child_file, b"child cache").expect("the child fixture must be written");
        let plan = compile_scan_plan(
            vec![
                CompiledRule::whole_root_fixture(
                    "development.parent-fixture",
                    cleanup_root,
                    crate::cleanup::CleanupCategory::Development,
                ),
                CompiledRule::fixture(
                    "development.child-fixture",
                    child_root,
                    crate::cleanup::CleanupCategory::Development,
                    MatcherSpec::All,
                ),
            ],
            &[true, true],
            &[],
        )
        .expect("nested ownership must compile");
        let process_snapshot = ProcessSnapshot::default();
        let operation = OperationGuard::start(CoordinatedOperationKind::Cleanup)
            .expect("the isolated cleanup operation must start");

        let action = execute_rule(
            &plan.rules[0],
            0,
            None,
            &RuleExecutionContext {
                ownership_plan: &plan,
                process_snapshot: &process_snapshot,
                source_scope: None,
                operation: &operation,
                dry_run: false,
            },
            &mut |_, _| {},
        );

        operation.complete();
        assert_eq!(
            action.status,
            crate::cleanup::CleanupActionStatus::Completed
        );
        assert!(
            !parent_file.exists(),
            "the parent-owned cache must be removed"
        );
        assert!(
            child_file.exists(),
            "fallback traversal must preserve a nested rule boundary"
        );
    }

    struct DirectoryCleanup(PathBuf);

    impl Drop for DirectoryCleanup {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }

    #[test]
    fn filtered_cleanup_preserves_unmatched_empty_directories() {
        let _operation_lock = crate::shared::operation::test_operation_lock();
        let sandbox = std::env::temp_dir().join(format!(
            "mangodisk-cleanup-matcher-test-{}-{}",
            std::process::id(),
            now_ms()
        ));
        let _sandbox_cleanup = DirectoryCleanup(sandbox.clone());
        let cleanup_root = sandbox.join("cache");
        let unmatched_empty = cleanup_root.join("user-created-empty");
        let matched_directory = cleanup_root.join("generated");
        let matched_file = matched_directory.join("cache.tmp");
        fs::create_dir_all(&unmatched_empty).expect("the unmatched directory must be created");
        fs::create_dir_all(&matched_directory).expect("the matched directory must be created");
        fs::write(&matched_file, b"temporary cache").expect("the matched file must be written");
        let canonical_root = validate_rule_root(&cleanup_root, &MatcherSpec::All)
            .expect("the isolated root must be safe");
        let mut stats = DeleteStats {
            matched_bytes: 0,
            deleted_bytes: 0,
            affected_item_count: 0,
            failed_item_count: 0,
        };
        let mut item_progress = Vec::new();

        delete_root_contents_with_progress(
            &cleanup_root,
            &canonical_root,
            &MatcherSpec::ExtensionIn(vec!["tmp".to_string()]),
            &|_, _| true,
            &|| false,
            &mut stats,
            &mut |path, stats| {
                item_progress.push((
                    path.to_path_buf(),
                    stats.affected_item_count,
                    stats.deleted_bytes,
                ));
            },
        );

        assert!(
            !matched_file.exists(),
            "the matched cache file must be deleted"
        );
        assert!(
            !matched_directory.exists(),
            "a directory emptied by this operation may be pruned"
        );
        assert!(
            unmatched_empty.exists(),
            "a pre-existing empty directory is outside the matcher scope"
        );
        assert_eq!(stats.affected_item_count, 1);
        assert_eq!(stats.matched_bytes, b"temporary cache".len() as u64);
        assert_eq!(stats.failed_item_count, 0);
        assert_eq!(item_progress.len(), 1);
        assert_eq!(item_progress[0].0, matched_file);
        assert_eq!(item_progress[0].1, 1);
        assert_eq!(item_progress[0].2, b"temporary cache".len() as u64);
    }

    #[test]
    fn cancelled_cleanup_stops_before_removing_the_next_entry() {
        let _operation_lock = crate::shared::operation::test_operation_lock();
        let sandbox = std::env::temp_dir().join(format!(
            "mangodisk-cleanup-cancel-test-{}-{}",
            std::process::id(),
            now_ms()
        ));
        let _sandbox_cleanup = DirectoryCleanup(sandbox.clone());
        let cleanup_root = sandbox.join("cache");
        let cache_file = cleanup_root.join("cache.tmp");
        fs::create_dir_all(&cleanup_root).expect("the isolated cleanup root must be created");
        fs::write(&cache_file, b"temporary cache").expect("the cache fixture must be written");
        let canonical_root = validate_rule_root(&cleanup_root, &MatcherSpec::All)
            .expect("the isolated root must be safe");
        let mut stats = DeleteStats {
            matched_bytes: 0,
            deleted_bytes: 0,
            affected_item_count: 0,
            failed_item_count: 0,
        };

        delete_root_contents(
            &cleanup_root,
            &canonical_root,
            &MatcherSpec::All,
            &|_, _| true,
            &|| true,
            &mut stats,
        );

        assert!(
            cache_file.exists(),
            "a cancellation observed before traversal must preserve the file"
        );
        assert_eq!(stats.affected_item_count, 0);
        assert_eq!(stats.failed_item_count, 1);
    }

    #[test]
    fn overlapping_cleanup_respects_unselected_child_rule_ownership() {
        let _operation_lock = crate::shared::operation::test_operation_lock();
        let sandbox = std::env::temp_dir().join(format!(
            "mangodisk-cleanup-ownership-test-{}-{}",
            std::process::id(),
            now_ms()
        ));
        let _sandbox_cleanup = DirectoryCleanup(sandbox.clone());
        let parent_root = sandbox.join("cache");
        let child_root = parent_root.join("specialized");
        let parent_file = parent_root.join("general.bin");
        let child_file = child_root.join("owned.tmp");
        fs::create_dir_all(&child_root).expect("the overlapping roots must be created");
        fs::write(&parent_file, b"general cache").expect("the parent-owned file must be written");
        fs::write(&child_file, b"specialized cache").expect("the child-owned file must be written");

        let plan = compile_scan_plan(
            vec![
                CompiledRule::fixture(
                    "system.parent",
                    parent_root.clone(),
                    crate::cleanup::CleanupCategory::System,
                    MatcherSpec::All,
                ),
                CompiledRule::fixture(
                    "application.child",
                    child_root.clone(),
                    crate::cleanup::CleanupCategory::Application,
                    MatcherSpec::ExtensionIn(vec!["tmp".to_string()]),
                ),
            ],
            &[true, true],
            &[],
        )
        .expect("overlapping cleanup rules must produce stable ownership");

        let canonical_parent = validate_rule_root(&parent_root, &MatcherSpec::All)
            .expect("the parent root must be safe");
        let mut parent_stats = DeleteStats {
            matched_bytes: 0,
            deleted_bytes: 0,
            affected_item_count: 0,
            failed_item_count: 0,
        };
        delete_root_contents(
            &parent_root,
            &canonical_parent,
            &MatcherSpec::All,
            &|path, metadata| plan.rule_owns_path(0, path, metadata),
            &|| false,
            &mut parent_stats,
        );

        assert!(
            !parent_file.exists(),
            "the parent-owned file must be deleted"
        );
        assert!(
            child_file.exists(),
            "the unselected child rule must retain ownership of its file"
        );
        assert_eq!(parent_stats.affected_item_count, 1);
        assert_eq!(parent_stats.failed_item_count, 0);

        let canonical_child = validate_rule_root(
            &child_root,
            &MatcherSpec::ExtensionIn(vec!["tmp".to_string()]),
        )
        .expect("the child rule root must be safe");
        let mut child_stats = DeleteStats {
            matched_bytes: 0,
            deleted_bytes: 0,
            affected_item_count: 0,
            failed_item_count: 0,
        };
        delete_root_contents(
            &child_root,
            &canonical_child,
            &MatcherSpec::ExtensionIn(vec!["tmp".to_string()]),
            &|path, metadata| plan.rule_owns_path(1, path, metadata),
            &|| false,
            &mut child_stats,
        );

        assert!(
            !child_file.exists(),
            "the selected child rule may delete its file"
        );
        assert_eq!(child_stats.affected_item_count, 1);
        assert_eq!(child_stats.failed_item_count, 0);
    }
}

#[cfg(all(test, target_os = "macos"))]
mod macos_cleanup_tests {
    use std::{
        ffi::OsString,
        os::unix::fs::symlink,
        path::PathBuf,
        time::{Duration, SystemTime},
    };

    use super::*;
    use crate::cleanup::CleanupRequest;

    struct DirectoryCleanup(PathBuf);

    struct EnvironmentRestore(Vec<(&'static str, Option<OsString>)>);

    impl Drop for DirectoryCleanup {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }

    impl Drop for EnvironmentRestore {
        fn drop(&mut self) {
            for (name, value) in self.0.drain(..) {
                match value {
                    Some(value) => std::env::set_var(name, value),
                    None => std::env::remove_var(name),
                }
            }
        }
    }

    #[test]
    fn cleanup_deletes_regular_files_without_following_external_links() {
        let _operation_lock = crate::shared::operation::test_operation_lock();
        let sandbox = std::env::temp_dir().join(format!(
            "mangodisk-cleanup-boundary-test-{}-{}",
            std::process::id(),
            now_ms()
        ));
        let _sandbox_cleanup = DirectoryCleanup(sandbox.clone());
        let cleanup_root = sandbox.join("cache");
        let external_root = sandbox.join("external");
        let regular_file = cleanup_root.join("regular.tmp");
        let protected_file = external_root.join("protected.txt");
        let external_link = cleanup_root.join("external-link");
        fs::create_dir_all(&cleanup_root).expect("the isolated cleanup root must be created");
        fs::create_dir_all(&external_root).expect("the external fixture must be created");
        fs::write(&regular_file, b"temporary cache")
            .expect("the regular cache file must be written");
        fs::write(&protected_file, b"must remain").expect("the protected file must be written");
        symlink(&external_root, &external_link).expect("the external symlink must be created");
        let canonical_root = validate_rule_root(&cleanup_root, &MatcherSpec::All)
            .expect("the isolated root must be safe");
        let mut stats = DeleteStats {
            matched_bytes: 0,
            deleted_bytes: 0,
            affected_item_count: 0,
            failed_item_count: 0,
        };

        delete_root_contents(
            &cleanup_root,
            &canonical_root,
            &MatcherSpec::All,
            &|_, _| true,
            &|| false,
            &mut stats,
        );

        assert!(
            !regular_file.exists(),
            "the regular cache file must be deleted"
        );
        assert!(
            protected_file.exists(),
            "cleanup must not follow links outside the rule root"
        );
        assert!(
            external_link.symlink_metadata().is_ok(),
            "a rejected link must remain unchanged"
        );
        assert_eq!(stats.affected_item_count, 1);
        assert_eq!(stats.failed_item_count, 1);
    }

    #[test]
    #[ignore = "modifies HOME and executes isolated cleanup; run this test alone"]
    fn communication_cache_rule_preserves_message_container_data() {
        let _operation_lock = crate::shared::operation::test_operation_lock();
        let sandbox = std::env::current_dir()
            .expect("the test process must have a working directory")
            .join("target")
            .join(format!(
                "mangodisk-communication-cache-cleanup-test-{}-{}",
                std::process::id(),
                now_ms()
            ));
        let _sandbox_cleanup = DirectoryCleanup(sandbox.clone());
        let home = sandbox.join("home");
        let cache_file = home
            .join("Library/Caches/net.whatsapp.WhatsApp")
            .join("generated-cache.bin");
        let message_database = home
            .join("Library/Containers/net.whatsapp.WhatsApp/Data/Documents")
            .join("messages.db");
        for fixture in [&cache_file, &message_database] {
            fs::create_dir_all(fixture.parent().expect("the fixture must have a parent"))
                .expect("the isolated application directory must be created");
            fs::write(fixture, b"MangoDisk communication cache fixture")
                .expect("the isolated fixture must be written");
        }

        let _restore = EnvironmentRestore(vec![("HOME", std::env::var_os("HOME"))]);
        std::env::set_var("HOME", &home);

        let preview = CleanupService::execute(CleanupRequest {
            rule_ids: vec!["app.whatsapp-cache".to_string()],
            source_selections: Vec::new(),
            dry_run: true,
            project_roots: Vec::new(),
        })
        .expect("isolated communication cache preview must succeed");
        assert_eq!(preview.failed_item_count, 0);
        assert!(
            cache_file.exists(),
            "dry-run must preserve the cache fixture"
        );
        assert!(
            message_database.exists(),
            "dry-run must preserve message container data"
        );

        let result = CleanupService::execute(CleanupRequest {
            rule_ids: vec!["app.whatsapp-cache".to_string()],
            source_selections: Vec::new(),
            dry_run: false,
            project_roots: Vec::new(),
        })
        .expect("isolated communication cache cleanup must succeed");
        assert_eq!(result.failed_item_count, 0, "{:?}", result.actions);
        assert_eq!(result.affected_item_count, 1);
        assert!(
            !cache_file.exists(),
            "the rebuildable bundle cache must be deleted"
        );
        assert!(
            message_database.exists(),
            "message container data must remain outside the cleanup boundary"
        );
    }

    #[test]
    #[ignore = "modifies HOME and executes isolated cleanup; run this test alone"]
    fn developer_cache_rules_preserve_tools_configuration_and_project_data() {
        let _operation_lock = crate::shared::operation::test_operation_lock();
        let sandbox = std::env::current_dir()
            .expect("the test process must have a working directory")
            .join("target")
            .join(format!(
                "mangodisk-developer-cache-cleanup-test-{}-{}",
                std::process::id(),
                now_ms()
            ));
        let _sandbox_cleanup = DirectoryCleanup(sandbox.clone());
        let home = sandbox.join("home");
        let cache_files = [
            home.join("Library/Caches/deno/deps/cache.bin"),
            home.join(".bun/install/cache/package/index.js"),
            home.join("Library/Caches/composer/files/package.zip"),
            home.join(".composer/cache/repo/metadata.json"),
            home.join("Library/Caches/mise/node/remote_versions.msgpack.z"),
            home.join("Library/Caches/ccache/a/result"),
            home.join("Library/Caches/Mozilla.sccache/0/compile-result"),
            home.join(".gem/ruby/4.0.0/cache/example-1.0.0.gem"),
            home.join(".hex/cache/registry.ets"),
            home.join("Library/Caches/copilot/marketplace/index.json"),
            home.join(".m2/repository/org/example/demo/1.0/demo-1.0.jar"),
            home.join(".nuget/packages/example/1.0/example.1.0.nupkg"),
        ];
        let protected_files = [
            home.join(".deno/bin/deno"),
            home.join(".bun/bin/bun"),
            home.join(".composer/auth.json"),
            home.join(".local/share/mise/installs/node/22/bin/node"),
            home.join("Library/Caches/mise/http-tarballs/tool/extracted/bin/http-backend-tool"),
            home.join("project/vendor/package/source.php"),
            home.join("Library/Preferences/ccache/ccache.conf"),
            home.join("Library/Application Support/Mozilla.sccache/config"),
            home.join(".gem/ruby/4.0.0/gems/example-1.0.0/lib/example.rb"),
            home.join(".hex/hex.config"),
            home.join(".copilot/settings.json"),
            home.join(".m2/settings.xml"),
            home.join(".nuget/NuGet/NuGet.Config"),
            home.join("project/pom.xml"),
        ];
        for fixture in cache_files.iter().chain(&protected_files) {
            fs::create_dir_all(fixture.parent().expect("the fixture must have a parent"))
                .expect("the isolated developer tool directory must be created");
            fs::write(fixture, b"MangoDisk developer cache fixture")
                .expect("the isolated developer tool fixture must be written");
        }

        let _restore = EnvironmentRestore(vec![("HOME", std::env::var_os("HOME"))]);
        std::env::set_var("HOME", &home);
        let rule_ids = [
            "dev.deno-cache",
            "dev.bun-cache",
            "dev.composer-cache",
            "dev.mise-cache",
            "dev.ccache-cache",
            "dev.sccache-cache",
            "dev.rubygems-cache",
            "dev.hex-cache",
            "dev.copilot-cli-cache",
            "dev.maven-cache",
            "dev.nuget-cache",
        ]
        .map(str::to_string)
        .to_vec();

        let preview = CleanupService::execute(CleanupRequest {
            rule_ids: rule_ids.clone(),
            source_selections: Vec::new(),
            dry_run: true,
            project_roots: Vec::new(),
        })
        .expect("isolated developer cache preview must succeed");
        assert_eq!(preview.failed_item_count, 0, "{:?}", preview.actions);
        assert!(cache_files.iter().all(|fixture| fixture.exists()));
        assert!(protected_files.iter().all(|fixture| fixture.exists()));

        let result = CleanupService::execute(CleanupRequest {
            rule_ids,
            source_selections: Vec::new(),
            dry_run: false,
            project_roots: Vec::new(),
        })
        .expect("isolated developer cache cleanup must succeed");
        assert_eq!(result.failed_item_count, 0, "{:?}", result.actions);
        assert_eq!(result.affected_item_count, 12);
        assert!(cache_files.iter().all(|fixture| !fixture.exists()));
        assert!(protected_files.iter().all(|fixture| fixture.exists()));
    }

    #[test]
    #[ignore = "modifies HOME and executes isolated cleanup; run this test alone"]
    fn ai_cache_rules_clean_only_rebuildable_data_and_preserve_models() {
        let _operation_lock = crate::shared::operation::test_operation_lock();
        // HOME cannot be nested under the system temporary directory because
        // the real system.user-temp rule would correctly own its parent. Keep
        // the isolated home under target to avoid user data and preserve the
        // same non-overlapping relationship as a real home directory.
        let sandbox = std::env::current_dir()
            .expect("the test process must have a working directory")
            .join("target")
            .join(format!(
                "mangodisk-ai-cache-cleanup-test-{}-{}",
                std::process::id(),
                now_ms()
            ));
        let _sandbox_cleanup = DirectoryCleanup(sandbox.clone());
        let home = sandbox.join("home");
        let downloads = home.join("Downloads");
        let huggingface_hub = home.join(".cache/huggingface/hub/models--fixture/blobs");
        let xet_environment = home.join(".cache/huggingface/xet/environment");
        let xet_chunk_cache = xet_environment.join("chunk_cache");
        let xet_shard_cache = xet_environment.join("shard_cache");
        let xet_staging = xet_environment.join("staging");
        let project = home.join("project");
        for directory in [
            &downloads,
            &huggingface_hub,
            &xet_chunk_cache,
            &xet_shard_cache,
            &xet_staging,
            &project,
        ] {
            fs::create_dir_all(directory).expect("the isolated rule directory must be created");
        }

        let stale_partial = downloads.join("old-model.crdownload");
        let recent_partial = downloads.join("active-model.crdownload");
        let completed_download = downloads.join("archive.zip");
        let downloaded_model = huggingface_hub.join("downloaded-model.bin");
        let xet_chunk = xet_chunk_cache.join("chunk.bin");
        let xet_shard = xet_shard_cache.join("shard.mdb");
        let resumable_upload = xet_staging.join("upload.mdb");
        let project_model = project.join("model.bin");
        for fixture in [
            &stale_partial,
            &recent_partial,
            &completed_download,
            &downloaded_model,
            &xet_chunk,
            &xet_shard,
            &resumable_upload,
            &project_model,
        ] {
            fs::write(fixture, b"MangoDisk AI cache fixture")
                .expect("the isolated cleanup fixture must be written");
        }
        let stale_time = SystemTime::now()
            .checked_sub(Duration::from_secs(8 * 86_400))
            .expect("the fixture timestamp must support an eight-day offset");
        fs::File::options()
            .write(true)
            .open(&stale_partial)
            .expect("the stale download fixture must open")
            .set_times(fs::FileTimes::new().set_modified(stale_time))
            .expect("the stale download timestamp must be updated");

        let _restore = EnvironmentRestore(vec![("HOME", std::env::var_os("HOME"))]);
        std::env::set_var("HOME", &home);

        assert!(
            validate_rule_root(&downloads, &MatcherSpec::All).is_err(),
            "Downloads must not be authorized as a broad cleanup root"
        );
        assert!(
            validate_rule_root(
                &downloads,
                &MatcherSpec::AllOf(vec![
                    MatcherSpec::OlderThanDays(6),
                    MatcherSpec::ExtensionIn(vec!["crdownload".to_string()]),
                ]),
            )
            .is_err(),
            "a recent partial download must not be authorized"
        );
        assert!(
            validate_rule_root(
                &downloads,
                &MatcherSpec::AllOf(vec![
                    MatcherSpec::OlderThanDays(7),
                    MatcherSpec::ExtensionIn(vec!["zip".to_string()]),
                ]),
            )
            .is_err(),
            "a regular downloaded file must not be authorized"
        );

        let retired_rule = CleanupService::execute(CleanupRequest {
            rule_ids: vec![
                "system.stale-partial-downloads".to_string(),
                "ai.model-cache".to_string(),
            ],
            source_selections: Vec::new(),
            dry_run: false,
            project_roots: Vec::new(),
        });
        assert!(
            retired_rule.is_err(),
            "a retired unsafe rule ID must be rejected before deletion"
        );
        assert!(
            stale_partial.exists(),
            "an unknown rule must prevent all deletion in the request"
        );
        assert!(
            CleanupService::execute(CleanupRequest {
                rule_ids: vec!["ai.gemini-temp-files".to_string()],
                source_selections: Vec::new(),
                dry_run: false,
                project_roots: Vec::new(),
            })
            .is_err(),
            "the retired rule that covered Gemini sessions must stay unavailable"
        );

        let result = CleanupService::execute(CleanupRequest {
            rule_ids: vec![
                "system.stale-partial-downloads".to_string(),
                "ai.huggingface-xet-cache".to_string(),
            ],
            source_selections: Vec::new(),
            dry_run: false,
            project_roots: Vec::new(),
        })
        .expect("isolated stale-download and AI transfer-cache cleanup must succeed");

        assert_eq!(
            result.failed_item_count, 0,
            "isolated cleanup must not fail: {:?}",
            result.actions
        );
        assert_eq!(result.affected_item_count, 3);
        assert!(
            !stale_partial.exists(),
            "a partial download older than seven days must be deleted"
        );
        assert!(
            !xet_chunk.exists(),
            "the Xet download transfer cache must be deleted"
        );
        assert!(
            !xet_shard.exists(),
            "the Xet upload transfer cache must be deleted"
        );
        assert!(downloaded_model.exists(), "Hugging Face models must remain");
        assert!(
            resumable_upload.exists(),
            "resumable Xet uploads must remain"
        );
        assert!(
            recent_partial.exists(),
            "recent partial downloads must remain"
        );
        assert!(
            completed_download.exists(),
            "completed downloads must remain"
        );
        assert!(
            project_model.exists(),
            "models inside projects must remain unchanged"
        );
    }
}

#[cfg(all(test, windows))]
mod windows_cleanup_tests {
    use std::{
        ffi::OsString,
        fs,
        path::PathBuf,
        time::{Duration, Instant, SystemTime},
    };

    use super::*;
    use crate::cleanup::CleanupRequest;

    struct EnvironmentRestore(Vec<(&'static str, Option<OsString>)>);

    struct DirectoryCleanup(PathBuf);

    impl Drop for DirectoryCleanup {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }

    impl Drop for EnvironmentRestore {
        fn drop(&mut self) {
            for (name, value) in self.0.drain(..) {
                match value {
                    Some(value) => std::env::set_var(name, value),
                    None => std::env::remove_var(name),
                }
            }
        }
    }

    /// Executes the production Dart cache rule in a snapshot-backed Windows VM.
    ///
    /// The test deletes the current account's real `.dartServer` cache and
    /// therefore requires an explicit environment gate. It validates dry-run,
    /// Known Folder resolution, whole-root deletion, live accounting, and final
    /// root absence while printing only aggregate counts and timings.
    #[test]
    #[ignore = "deletes the real Dart analysis cache in an isolated Windows VM"]
    fn real_dart_analysis_cache_uses_whole_root_cleanup() {
        assert_eq!(
            std::env::var("MANGODISK_TEST_REAL_DART_CACHE").as_deref(),
            Ok("1"),
            "set MANGODISK_TEST_REAL_DART_CACHE=1 only in a snapshot-backed Windows VM"
        );
        let local_app_data = std::env::var_os("LOCALAPPDATA")
            .map(PathBuf::from)
            .expect("LOCALAPPDATA must be available");
        let cache_root = local_app_data.join(".dartServer");
        assert!(
            cache_root.is_dir(),
            "the real Dart cache fixture must exist"
        );
        let request = || CleanupRequest {
            rule_ids: vec!["dev.dart-analysis-cache".to_string()],
            source_selections: Vec::new(),
            dry_run: false,
            project_roots: Vec::new(),
        };

        let preview_started = Instant::now();
        let mut preview_request = request();
        preview_request.dry_run = true;
        let preview = CleanupService::execute(preview_request)
            .expect("the real Dart cache preview must succeed");
        let preview_ms = preview_started.elapsed().as_secs_f64() * 1_000.0;
        assert_eq!(preview.actions.len(), 1);
        assert_eq!(preview.failed_item_count, 0, "{:?}", preview.actions);
        assert!(preview.expected_bytes > 0);
        assert!(cache_root.exists(), "dry-run must preserve the Dart cache");

        let cleanup_started = Instant::now();
        let result =
            CleanupService::execute(request()).expect("the real Dart cache cleanup must succeed");
        let cleanup_ms = cleanup_started.elapsed().as_secs_f64() * 1_000.0;
        assert_eq!(result.actions.len(), 1);
        assert_eq!(result.failed_item_count, 0, "{:?}", result.actions);
        assert!(result.released_bytes > 0);
        assert!(result.affected_item_count > 0);
        assert!(
            !cache_root.exists(),
            "the complete Dart cache root must be removed"
        );
        println!(
            "real_dart_analysis_cleanup preview_ms={preview_ms:.2} cleanup_ms={cleanup_ms:.2} expected_bytes={} released_bytes={} affected_item_count={}",
            preview.expected_bytes,
            result.released_bytes,
            result.affected_item_count
        );
    }

    #[test]
    #[ignore = "modifies process environment; run this test alone"]
    fn zoom_diagnostic_rule_preserves_recent_logs_and_recordings() {
        let _operation_lock = crate::shared::operation::test_operation_lock();
        let sandbox = std::env::current_dir()
            .expect("test process should have a working directory")
            .join("target")
            .join(format!(
                "mangodisk-zoom-cleanup-test-{}-{}",
                std::process::id(),
                now_ms()
            ));
        let _sandbox_cleanup = DirectoryCleanup(sandbox.clone());
        let profile = sandbox.join("UserProfile");
        let roaming = sandbox.join("RoamingAppData");
        let old_log = roaming.join("Zoom/logs/old-diagnostic.log");
        let recent_log = roaming.join("Zoom/logs/recent-diagnostic.log");
        let recording = profile.join("Documents/Zoom/meeting-recording.mp4");
        for fixture in [&old_log, &recent_log, &recording] {
            fs::create_dir_all(fixture.parent().expect("fixture must have a parent"))
                .expect("should create the isolated Zoom directory");
            fs::write(fixture, b"MangoDisk Zoom cleanup fixture")
                .expect("should write the isolated Zoom fixture");
        }
        let old_time = SystemTime::now()
            .checked_sub(Duration::from_secs(15 * 86_400))
            .expect("test time should move back by fifteen days");
        fs::File::options()
            .write(true)
            .open(&old_log)
            .expect("should open the old Zoom log fixture")
            .set_times(fs::FileTimes::new().set_modified(old_time))
            .expect("should set the old Zoom log modification time");

        let _restore = EnvironmentRestore(vec![
            ("USERPROFILE", std::env::var_os("USERPROFILE")),
            ("APPDATA", std::env::var_os("APPDATA")),
        ]);
        std::env::set_var("USERPROFILE", &profile);
        std::env::set_var("APPDATA", &roaming);

        let preview = CleanupService::execute(CleanupRequest {
            rule_ids: vec!["app.zoom-diagnostic-cache".to_string()],
            source_selections: Vec::new(),
            dry_run: true,
            project_roots: Vec::new(),
        })
        .expect("isolated Zoom diagnostic preview should succeed");
        assert_eq!(preview.failed_item_count, 0);
        assert!(old_log.exists());
        assert!(recent_log.exists());
        assert!(recording.exists());

        let result = CleanupService::execute(CleanupRequest {
            rule_ids: vec!["app.zoom-diagnostic-cache".to_string()],
            source_selections: Vec::new(),
            dry_run: false,
            project_roots: Vec::new(),
        })
        .expect("isolated Zoom diagnostic cleanup should succeed");

        assert_eq!(result.failed_item_count, 0, "{:?}", result.actions);
        assert_eq!(result.affected_item_count, 1);
        assert!(
            !old_log.exists(),
            "Zoom diagnostic logs older than two weeks should be deleted"
        );
        assert!(
            recent_log.exists(),
            "recent Zoom diagnostic logs should remain available"
        );
        assert!(
            recording.exists(),
            "Zoom recordings must remain outside the cleanup boundary"
        );
    }

    #[test]
    #[ignore = "modifies process environment; run this test alone"]
    fn crash_dumps_and_windows_error_reports_are_actually_cleaned_in_isolated_roots() {
        const FIXTURE_CONTENT: &[u8] = b"MangoDisk safe cleanup fixture";

        let _operation_lock = crate::shared::operation::test_operation_lock();
        let sandbox = std::env::current_dir()
            .expect("test process should have a working directory")
            .join("target")
            .join(format!(
                "mangodisk-windows-cleanup-test-{}-{}",
                std::process::id(),
                now_ms()
            ));
        let _sandbox_cleanup = DirectoryCleanup(sandbox.clone());
        let local = sandbox.join("LocalAppData");
        let program_data = sandbox.join("ProgramData");
        let crash_dump = local.join("CrashDumps/fixture crash.dmp");
        let user_report =
            local.join("Microsoft/Windows/WER/ReportArchive/MangoDisk_User_Fixture/Report.wer");
        let system_report = program_data
            .join("Microsoft/Windows/WER/ReportQueue/MangoDisk_System_Fixture/Report.wer");
        let temporary_report = program_data.join("Microsoft/Windows/WER/Temp/fixture.tmp");
        for fixture in [&crash_dump, &user_report, &system_report, &temporary_report] {
            fs::create_dir_all(fixture.parent().expect("fixture must have a parent"))
                .expect("should create isolated diagnostic directory");
            fs::write(fixture, FIXTURE_CONTENT).expect("should write isolated diagnostic fixture");
        }

        let _restore = EnvironmentRestore(vec![
            ("LOCALAPPDATA", std::env::var_os("LOCALAPPDATA")),
            ("PROGRAMDATA", std::env::var_os("PROGRAMDATA")),
        ]);
        std::env::set_var("LOCALAPPDATA", &local);
        std::env::set_var("PROGRAMDATA", &program_data);

        let result = CleanupService::execute(CleanupRequest {
            rule_ids: vec![
                "system.crash-dumps".to_string(),
                "system.error-reports".to_string(),
            ],
            source_selections: Vec::new(),
            dry_run: false,
            project_roots: Vec::new(),
        })
        .expect("isolated CrashDumps and WER cleanup should succeed");

        assert_eq!(result.failed_item_count, 0);
        assert_eq!(result.affected_item_count, 4);
        assert_eq!(
            result.released_bytes,
            4 * u64::try_from(FIXTURE_CONTENT.len()).expect("fixture length should fit in u64")
        );
        assert!([crash_dump, user_report, system_report, temporary_report]
            .into_iter()
            .all(|fixture| !fixture.exists()));
    }

    #[test]
    #[ignore = "modifies process environment; run this test alone"]
    fn developer_cache_rules_preserve_windows_configuration_and_credentials() {
        let _operation_lock = crate::shared::operation::test_operation_lock();
        let sandbox = std::env::current_dir()
            .expect("test process should have a working directory")
            .join("target")
            .join(format!(
                "mangodisk-developer-cache-windows-cleanup-test-{}-{}",
                std::process::id(),
                now_ms()
            ));
        let _sandbox_cleanup = DirectoryCleanup(sandbox.clone());
        let profile = sandbox.join("UserProfile");
        let local = sandbox.join("LocalAppData");
        let roaming = sandbox.join("RoamingAppData");
        let cache_files = [
            roaming.join("ccache/a/result"),
            local.join("Mozilla/sccache/cache/0/compile-result"),
            profile.join(".hex/cache/registry.ets"),
            local.join("copilot/marketplace/index.json"),
        ];
        let protected_files = [
            roaming.join("ccache/ccache.conf"),
            roaming.join("Mozilla/sccache/config/config"),
            profile.join(".hex/hex.config"),
            profile.join(".copilot/settings.json"),
        ];
        for fixture in cache_files.iter().chain(&protected_files) {
            fs::create_dir_all(fixture.parent().expect("fixture must have a parent"))
                .expect("should create the isolated developer tool directory");
            fs::write(fixture, b"MangoDisk developer cache fixture")
                .expect("should write the isolated developer tool fixture");
        }

        let _restore = EnvironmentRestore(vec![
            ("USERPROFILE", std::env::var_os("USERPROFILE")),
            ("LOCALAPPDATA", std::env::var_os("LOCALAPPDATA")),
            ("APPDATA", std::env::var_os("APPDATA")),
        ]);
        std::env::set_var("USERPROFILE", &profile);
        std::env::set_var("LOCALAPPDATA", &local);
        std::env::set_var("APPDATA", &roaming);
        let rule_ids = [
            "dev.ccache-cache",
            "dev.sccache-cache",
            "dev.hex-cache",
            "dev.copilot-cli-cache",
        ]
        .map(str::to_string)
        .to_vec();

        let preview = CleanupService::execute(CleanupRequest {
            rule_ids: rule_ids.clone(),
            source_selections: Vec::new(),
            dry_run: true,
            project_roots: Vec::new(),
        })
        .expect("isolated developer cache preview should succeed");
        assert_eq!(preview.failed_item_count, 0, "{:?}", preview.actions);
        assert!(cache_files.iter().all(|fixture| fixture.exists()));
        assert!(protected_files.iter().all(|fixture| fixture.exists()));

        let result = CleanupService::execute(CleanupRequest {
            rule_ids,
            source_selections: Vec::new(),
            dry_run: false,
            project_roots: Vec::new(),
        })
        .expect("isolated developer cache cleanup should succeed");
        assert_eq!(result.failed_item_count, 0, "{:?}", result.actions);
        assert_eq!(result.affected_item_count, 4);
        assert!(cache_files.iter().all(|fixture| !fixture.exists()));
        assert!(protected_files.iter().all(|fixture| fixture.exists()));
    }

    #[test]
    #[ignore = "modifies process environment; run this test alone"]
    fn ai_cache_rules_clean_only_rebuildable_data_and_preserve_models() {
        let _operation_lock = crate::shared::operation::test_operation_lock();
        let sandbox = std::env::current_dir()
            .expect("test process should have a working directory")
            .join("target")
            .join(format!(
                "mangodisk-ai-cache-windows-cleanup-test-{}-{}",
                std::process::id(),
                now_ms()
            ));
        let _sandbox_cleanup = DirectoryCleanup(sandbox.clone());
        let profile = sandbox.join("UserProfile");
        let local = sandbox.join("LocalAppData");
        let roaming = sandbox.join("RoamingAppData");
        let downloads = profile.join("Downloads");
        let huggingface_hub = profile.join(".cache/huggingface/hub/models--fixture/blobs");
        let xet_environment = profile.join(".cache/huggingface/xet/environment");
        let xet_chunk_cache = xet_environment.join("chunk_cache");
        let xet_shard_cache = xet_environment.join("shard_cache");
        let xet_staging = xet_environment.join("staging");
        let project = profile.join("project");
        let adobe_local = local.join("Adobe/Common/Media Cache Files");
        let adobe_roaming = roaming.join("Adobe/Common/Media Cache Files");
        for directory in [
            &downloads,
            &huggingface_hub,
            &xet_chunk_cache,
            &xet_shard_cache,
            &xet_staging,
            &project,
            &adobe_local,
            &adobe_roaming,
        ] {
            fs::create_dir_all(directory).expect("should create isolated rule directory");
        }

        let stale_partial = downloads.join("old-model.crdownload");
        let recent_partial = downloads.join("active-model.crdownload");
        let completed_download = downloads.join("archive.zip");
        let downloaded_model = huggingface_hub.join("downloaded-model.bin");
        let xet_chunk = xet_chunk_cache.join("chunk.bin");
        let xet_shard = xet_shard_cache.join("shard.mdb");
        let resumable_upload = xet_staging.join("upload.mdb");
        let project_model = project.join("model.bin");
        let local_media_cache = adobe_local.join("local-cache.bin");
        let roaming_media_cache = adobe_roaming.join("roaming-cache.bin");
        for fixture in [
            &stale_partial,
            &recent_partial,
            &completed_download,
            &downloaded_model,
            &xet_chunk,
            &xet_shard,
            &resumable_upload,
            &project_model,
            &local_media_cache,
            &roaming_media_cache,
        ] {
            fs::write(fixture, b"MangoDisk round 04 fixture")
                .expect("should write isolated cleanup fixture");
        }
        let stale_time = SystemTime::now()
            .checked_sub(Duration::from_secs(8 * 86_400))
            .expect("test time should move back by eight days");
        fs::File::options()
            .write(true)
            .open(&stale_partial)
            .expect("should open stale download fixture")
            .set_times(fs::FileTimes::new().set_modified(stale_time))
            .expect("should set stale download modification time");

        let _restore = EnvironmentRestore(vec![
            ("USERPROFILE", std::env::var_os("USERPROFILE")),
            ("LOCALAPPDATA", std::env::var_os("LOCALAPPDATA")),
            ("APPDATA", std::env::var_os("APPDATA")),
        ]);
        std::env::set_var("USERPROFILE", &profile);
        std::env::set_var("LOCALAPPDATA", &local);
        std::env::set_var("APPDATA", &roaming);

        assert!(
            validate_rule_root(&downloads, &MatcherSpec::All).is_err(),
            "Downloads must never be authorized for full-root cleanup"
        );

        let retired_rule = CleanupService::execute(CleanupRequest {
            rule_ids: vec![
                "system.stale-partial-downloads".to_string(),
                "ai.model-cache".to_string(),
            ],
            source_selections: Vec::new(),
            dry_run: false,
            project_roots: Vec::new(),
        });
        assert!(retired_rule.is_err());
        assert!(stale_partial.exists());
        assert!(
            CleanupService::execute(CleanupRequest {
                rule_ids: vec!["ai.gemini-temp-files".to_string()],
                source_selections: Vec::new(),
                dry_run: false,
                project_roots: Vec::new(),
            })
            .is_err(),
            "retired Gemini session cleanup rule must remain unavailable"
        );

        let result = CleanupService::execute(CleanupRequest {
            rule_ids: vec![
                "system.stale-partial-downloads".to_string(),
                "app.adobe-media-cache".to_string(),
                "ai.huggingface-xet-cache".to_string(),
            ],
            source_selections: Vec::new(),
            dry_run: false,
            project_roots: Vec::new(),
        })
        .expect("isolated AI cache cleanup should succeed");

        assert_eq!(
            result.failed_item_count, 0,
            "isolated cleanup should not fail: {:?}",
            result.actions
        );
        assert_eq!(result.affected_item_count, 5);
        assert!(!stale_partial.exists());
        assert!(!xet_chunk.exists());
        assert!(!xet_shard.exists());
        assert!(!local_media_cache.exists());
        assert!(!roaming_media_cache.exists());
        assert!(downloaded_model.exists());
        assert!(resumable_upload.exists());
        assert!(recent_partial.exists());
        assert!(completed_download.exists());
        assert!(project_model.exists());
    }
}
