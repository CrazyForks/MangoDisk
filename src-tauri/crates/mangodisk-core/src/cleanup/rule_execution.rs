use std::{
    cell::RefCell,
    collections::HashSet,
    fs,
    path::{Path, PathBuf},
    time::Instant,
};

use mangodisk_platform::{current_platform, DirectoryTreeAggregateError, Platform};

use crate::{
    applications::catalog::ProcessSnapshot,
    cleanup::{
        measurement::{measure_path_filtered, MeasureResult},
        rules::{
            matches_rule, protected_paths::validate_automatic_cleanup_root, CompiledRule,
            MatcherSpec, ScanPlan,
        },
        source_selection::{cleanup_source_path, SourceScope},
        CleanupActionKind, CleanupActionReason, CleanupActionResult, CleanupActionStatus,
    },
    filesystem::{
        metadata::{diagnostic_path, is_link_like, modified_ms},
        permanent_delete::{
            delete_directory_tree_permanently_with_cancellation,
            delete_empty_directory_permanently, delete_path_permanently,
            prepare_path_for_permanent_delete,
        },
    },
    shared::operation::OperationGuard,
};

pub(super) struct RuleExecutionContext<'a> {
    pub(super) ownership_plan: &'a ScanPlan,
    pub(super) process_snapshot: &'a ProcessSnapshot,
    pub(super) source_scope: Option<&'a SourceScope>,
    pub(super) operation: &'a OperationGuard,
    pub(super) dry_run: bool,
}

pub(super) fn execute_rule(
    rule: &CompiledRule,
    rule_index: usize,
    before: Option<MeasureResult>,
    context: &RuleExecutionContext<'_>,
    report_item: &mut dyn FnMut(&Path, &DeleteStats),
) -> CleanupActionResult {
    let measured_bytes = before.as_ref().map_or(0, |measurement| measurement.bytes);
    let running = context
        .process_snapshot
        .matching_processes(&rule.required_stopped_processes);
    if !running.is_empty() {
        let process_list = running.join(",");
        log::warn!(
            "cleanup_rule_blocked rule_id={} running_processes={}",
            rule.id,
            process_list
        );
        return CleanupActionResult {
            rule_id: rule.id.to_string(),
            action_kind: CleanupActionKind::Delete,
            status: CleanupActionStatus::Blocked,
            reason_code: Some(CleanupActionReason::RunningProcesses),
            bytes_expected: measured_bytes,
            released_bytes: 0,
            affected_item_count: 0,
            failed_item_count: 1,
            running_processes: running,
        };
    }
    if context.dry_run {
        // Preview never enters the destructive traversal, so its estimate must
        // come from the read-only measurement completed by the service.
        debug_assert!(before.is_some(), "dry-run cleanup requires a measurement");
        return CleanupActionResult {
            rule_id: rule.id.to_string(),
            action_kind: CleanupActionKind::Delete,
            status: CleanupActionStatus::Previewed,
            reason_code: None,
            bytes_expected: measured_bytes,
            released_bytes: 0,
            affected_item_count: 0,
            failed_item_count: 0,
            running_processes: Vec::new(),
        };
    }

    let mut stats = DeleteStats::default();
    for root in &rule.roots {
        if context.operation.ensure_not_cancelled().is_err() {
            stats.failed_item_count = stats.failed_item_count.saturating_add(1);
            break;
        }
        if !root.exists() {
            continue;
        }
        match validate_rule_root(root, &rule.matcher) {
            Ok(canonical_root) => {
                let handled = rule.deletes_whole_root()
                    && try_delete_whole_root(
                        rule,
                        rule_index,
                        root,
                        context,
                        &mut stats,
                        report_item,
                    );
                if !handled {
                    delete_root_contents_with_progress(
                        root,
                        &canonical_root,
                        &rule.matcher,
                        &|path, metadata| {
                            context
                                .ownership_plan
                                .rule_owns_path(rule_index, path, metadata)
                                && context.source_scope.is_none_or(|scope| {
                                    scope.selects(&cleanup_source_path(root, path))
                                })
                        },
                        &|| context.operation.ensure_not_cancelled().is_err(),
                        &mut stats,
                        report_item,
                    );
                }
            }
            Err(error) => {
                let error_digest = blake3::hash(error.as_bytes()).to_hex().to_string();
                log::warn!(
                    "cleanup_root_validation_failed rule_id={} path={} error_digest={}",
                    rule.id,
                    diagnostic_path(root),
                    error_digest
                );
                stats.failed_item_count += 1;
            }
        }
    }
    CleanupActionResult {
        rule_id: rule.id.to_string(),
        action_kind: CleanupActionKind::Delete,
        status: if stats.failed_item_count == 0 {
            CleanupActionStatus::Completed
        } else {
            CleanupActionStatus::Partial
        },
        // Cancellation belongs to this action only when it actually stopped
        // remaining work. A request arriving after the rule completed is
        // observed by the outer loop before the next rule, preventing an
        // contradictory Completed + Cancelled action result.
        reason_code: (stats.failed_item_count > 0).then(|| {
            if context.operation.ensure_not_cancelled().is_err() {
                CleanupActionReason::Cancelled
            } else {
                CleanupActionReason::ItemsSkipped
            }
        }),
        // Whole-rule cleanup discovers and deletes each candidate in one pass.
        // Scoped cleanup retains its preflight estimate because the selected
        // source paths must be proven live before the first mutation.
        bytes_expected: before.map_or(stats.matched_bytes, |measurement| measurement.bytes),
        released_bytes: stats.deleted_bytes,
        affected_item_count: stats.affected_item_count,
        failed_item_count: stats.failed_item_count,
        running_processes: Vec::new(),
    }
}

/// Tries the optimized complete-root deletion and reports whether this call
/// fully handled the root. `false` is a safe, pre-mutation downgrade request;
/// any failure after staging starts is recorded here and never retried through
/// a second deletion strategy.
fn try_delete_whole_root(
    rule: &CompiledRule,
    rule_index: usize,
    root: &Path,
    context: &RuleExecutionContext<'_>,
    stats: &mut DeleteStats,
    report_item: &mut dyn FnMut(&Path, &DeleteStats),
) -> bool {
    let fallback_reason = if context.source_scope.is_some() {
        Some("source_scope")
    } else if !context
        .ownership_plan
        .rule_exclusively_owns_root(rule_index, root)
    {
        Some("nested_ownership")
    } else {
        None
    };
    if let Some(reason) = fallback_reason {
        log::info!(
            "cleanup_whole_root_fallback rule_id={} reason={}",
            rule.id,
            reason
        );
        return false;
    }

    // Capture the physical root identity before measuring. The permanent
    // deletion boundary checks the same identity after its atomic rename, so
    // replacing the cache root during aggregation cannot redirect deletion.
    let prepared = match prepare_path_for_permanent_delete(root) {
        Ok(prepared) if prepared.metadata().is_dir() => prepared,
        Ok(_) => {
            stats.failed_item_count = stats.failed_item_count.saturating_add(1);
            return true;
        }
        Err(error) => {
            log::warn!(
                "cleanup_whole_root_prepare_failed rule_id={} error_digest={}",
                rule.id,
                blake3::hash(error.to_string().as_bytes()).to_hex()
            );
            stats.failed_item_count = stats.failed_item_count.saturating_add(1);
            return true;
        }
    };
    let started = Instant::now();
    let is_cancelled = || context.operation.ensure_not_cancelled().is_err();
    let aggregate = match current_platform().fast_directory_tree_aggregate(
        root,
        &is_cancelled,
        &|_, _, _| {},
    ) {
        Ok(Some(aggregate)) if aggregate.skipped_count == 0 => aggregate,
        Ok(Some(aggregate)) => {
            log::info!(
                "cleanup_whole_root_fallback rule_id={} reason=skipped_entries skipped_count={} strategy={}",
                rule.id,
                aggregate.skipped_count,
                aggregate.strategy
            );
            drop(prepared);
            return false;
        }
        Ok(None) => {
            log::info!(
                "cleanup_whole_root_fallback rule_id={} reason=native_aggregate_unavailable",
                rule.id
            );
            drop(prepared);
            return false;
        }
        Err(DirectoryTreeAggregateError::Cancelled) => {
            stats.failed_item_count = stats.failed_item_count.saturating_add(1);
            return true;
        }
        Err(DirectoryTreeAggregateError::Platform(error)) => {
            log::warn!(
                "cleanup_whole_root_fallback rule_id={} reason=native_aggregate_failed error_digest={}",
                rule.id,
                blake3::hash(error.as_bytes()).to_hex()
            );
            drop(prepared);
            return false;
        }
    };
    if context.operation.ensure_not_cancelled().is_err() {
        stats.failed_item_count = stats.failed_item_count.saturating_add(1);
        return true;
    }

    stats.matched_bytes = stats.matched_bytes.saturating_add(aggregate.bytes);
    match delete_directory_tree_permanently_with_cancellation(
        prepared,
        aggregate.bytes,
        aggregate.file_count,
        &is_cancelled,
    ) {
        Ok(outcome) => {
            stats.deleted_bytes = stats.deleted_bytes.saturating_add(outcome.released_bytes());
            stats.affected_item_count = stats
                .affected_item_count
                .saturating_add(outcome.affected_item_count());
            report_item(root, stats);
            log::info!(
                "cleanup_whole_root_completed rule_id={} expected_file_count={} expected_bytes={} affected_item_count={} released_bytes={} strategy={} elapsed_ms={}",
                rule.id,
                aggregate.file_count,
                aggregate.bytes,
                outcome.affected_item_count(),
                outcome.released_bytes(),
                aggregate.strategy,
                started.elapsed().as_millis()
            );
        }
        Err(error) => {
            stats.deleted_bytes = stats.deleted_bytes.saturating_add(error.released_bytes());
            stats.affected_item_count = stats
                .affected_item_count
                .saturating_add(error.affected_item_count());
            stats.failed_item_count = stats.failed_item_count.saturating_add(1);
            report_item(root, stats);
            log::warn!(
                "cleanup_whole_root_delete_failed rule_id={} released_bytes={} affected_item_count={} error_digest={}",
                rule.id,
                error.released_bytes(),
                error.affected_item_count(),
                blake3::hash(error.to_string().as_bytes()).to_hex()
            );
        }
    }
    true
}

pub(super) fn cancelled_action(
    rule_id: &str,
    action_kind: CleanupActionKind,
    bytes_expected: u64,
) -> CleanupActionResult {
    CleanupActionResult {
        rule_id: rule_id.to_string(),
        action_kind,
        status: CleanupActionStatus::Blocked,
        reason_code: Some(CleanupActionReason::Cancelled),
        bytes_expected,
        released_bytes: 0,
        affected_item_count: 0,
        failed_item_count: 1,
        running_processes: Vec::new(),
    }
}

pub(super) fn measure_owned_rule(
    plan: &ScanPlan,
    rule_index: usize,
    source_scope: Option<&SourceScope>,
) -> Result<MeasureResult, String> {
    let rule = &plan.rules[rule_index];
    let known_sources = RefCell::new(HashSet::<PathBuf>::new());
    let total = rule
        .roots
        .iter()
        .fold(MeasureResult::default(), |mut total, root| {
            let result = measure_path_filtered(root, Some(&rule.matcher), &|path, metadata| {
                if !plan.rule_owns_path(rule_index, path, metadata) {
                    return false;
                }
                let source = cleanup_source_path(root, path);
                known_sources.borrow_mut().insert(source.clone());
                source_scope.is_none_or(|scope| scope.selects(&source))
            });
            total.bytes = total.bytes.saturating_add(result.bytes);
            total.file_count = total.file_count.saturating_add(result.file_count);
            total.skipped_count = total.skipped_count.saturating_add(result.skipped_count);
            total
        });
    if let Some(scope) = source_scope {
        scope.validate_known_paths(known_sources.borrow().iter().map(PathBuf::as_path))?;
    }
    Ok(total)
}

/// Revalidates a declared cleanup root against the live filesystem.
///
/// Symbolic links and Windows reparse points cannot become roots because their
/// targets could cross into user data or another volume.
pub(super) fn validate_rule_root(root: &Path, matcher: &MatcherSpec) -> Result<PathBuf, String> {
    current_platform()
        .validate_path_no_links(root)
        .map_err(|error| error.to_string())?;
    let metadata = fs::symlink_metadata(root).map_err(|error| error.to_string())?;
    if !metadata.is_dir() {
        return Err("the cleanup rule root must be a directory".to_string());
    }
    let canonical = current_platform()
        .canonicalize_no_links(root)
        .map_err(|error| error.to_string())?;
    if let Err(error) = current_platform().validate_cleanup_root(&canonical) {
        if !is_narrow_stale_download_root(&canonical, matcher)? {
            return Err(error.to_string());
        }
    }
    let home = current_platform()
        .user_directories()
        .map_err(|error| error.to_string())?
        .home_directory()
        .to_path_buf();
    validate_automatic_cleanup_root(&canonical, &home)?;
    Ok(canonical)
}

fn is_narrow_stale_download_root(
    canonical_root: &Path,
    matcher: &MatcherSpec,
) -> Result<bool, String> {
    // Downloads is personal content and cannot be a general cleanup root. Stale
    // partial downloads are the only narrow exception: the root must exactly
    // match the current user's Downloads directory and the matcher must require
    // both a seven-day age gate and the complete temporary-extension allowlist.
    let downloads = current_platform()
        .user_directories()
        .map_err(|error| error.to_string())?
        .home_directory()
        .join("Downloads");
    let Ok(canonical_downloads) = fs::canonicalize(downloads) else {
        return Ok(false);
    };
    if canonical_root != canonical_downloads {
        return Ok(false);
    }
    let MatcherSpec::AllOf(items) = matcher else {
        return Ok(false);
    };
    let has_age_gate = items
        .iter()
        .any(|item| matches!(item, MatcherSpec::OlderThanDays(days) if *days >= 7));
    let allowed_extensions = ["crdownload", "download", "partial", "part"];
    let has_strict_extension_gate = items.iter().any(|item| {
        let MatcherSpec::ExtensionIn(values) = item else {
            return false;
        };
        !values.is_empty()
            && values.iter().all(|value| {
                allowed_extensions
                    .iter()
                    .any(|allowed| value.trim_start_matches('.').eq_ignore_ascii_case(allowed))
            })
    });
    Ok(has_age_gate && has_strict_extension_gate)
}

#[cfg(test)]
pub(super) fn delete_root_contents(
    root: &Path,
    canonical_root: &Path,
    matcher: &MatcherSpec,
    owns_path: &dyn Fn(&Path, &fs::Metadata) -> bool,
    is_cancelled: &dyn Fn() -> bool,
    stats: &mut DeleteStats,
) {
    delete_root_contents_with_progress(
        root,
        canonical_root,
        matcher,
        owns_path,
        is_cancelled,
        stats,
        &mut |_, _| {},
    );
}

pub(super) fn delete_root_contents_with_progress(
    root: &Path,
    canonical_root: &Path,
    matcher: &MatcherSpec,
    owns_path: &dyn Fn(&Path, &fs::Metadata) -> bool,
    is_cancelled: &dyn Fn() -> bool,
    stats: &mut DeleteStats,
    report_item: &mut dyn FnMut(&Path, &DeleteStats),
) {
    if is_cancelled() {
        stats.failed_item_count += 1;
        return;
    }
    if validate_cleanup_directory(root, canonical_root).is_err() {
        stats.failed_item_count += 1;
        return;
    }
    let Ok(entries) = fs::read_dir(root) else {
        stats.failed_item_count += 1;
        return;
    };
    let mut traversal = DeleteTraversalContext {
        canonical_root,
        matcher,
        owns_path,
        is_cancelled,
        report_item,
    };
    for entry in entries {
        if is_cancelled() {
            stats.failed_item_count += 1;
            break;
        }
        let Ok(entry) = entry else {
            stats.failed_item_count += 1;
            continue;
        };
        // Revalidate the root before each child. If another process replaces it
        // with a symlink or junction during cleanup, traversal stops immediately.
        if !revalidate_cleanup_directory(root, canonical_root) {
            stats.failed_item_count += 1;
            break;
        }
        delete_entry(&entry.path(), canonical_root, stats, &mut traversal);
    }
}

struct DeleteTraversalContext<'a> {
    canonical_root: &'a Path,
    matcher: &'a MatcherSpec,
    owns_path: &'a dyn Fn(&Path, &fs::Metadata) -> bool,
    is_cancelled: &'a dyn Fn() -> bool,
    report_item: &'a mut dyn FnMut(&Path, &DeleteStats),
}

fn delete_entry(
    path: &Path,
    canonical_parent: &Path,
    stats: &mut DeleteStats,
    traversal: &mut DeleteTraversalContext<'_>,
) -> bool {
    if (traversal.is_cancelled)() {
        stats.failed_item_count += 1;
        return false;
    }
    if !path
        .parent()
        .is_some_and(|parent| revalidate_cleanup_directory(parent, canonical_parent))
    {
        stats.failed_item_count += 1;
        return false;
    }
    let Ok(prepared) = prepare_path_for_permanent_delete(path) else {
        stats.failed_item_count += 1;
        return false;
    };
    let metadata = prepared.metadata();
    if is_link_like(metadata) {
        stats.failed_item_count += 1;
        return false;
    }
    if metadata.is_file() {
        if !matches_rule(
            traversal.canonical_root,
            path,
            metadata,
            Some(traversal.matcher),
        ) || !(traversal.owns_path)(path, metadata)
        {
            return false;
        }
        // Record the live candidate before the final identity check and delete
        // attempt. This preserves truthful expected-byte reporting for partial
        // failures without requiring a separate measurement traversal.
        stats.matched_bytes = stats.matched_bytes.saturating_add(metadata.len());
        if !path
            .parent()
            .is_some_and(|parent| revalidate_cleanup_directory(parent, canonical_parent))
        {
            stats.failed_item_count += 1;
            return false;
        }
        let Ok(verified) = fs::symlink_metadata(path) else {
            stats.failed_item_count += 1;
            return false;
        };
        if is_link_like(&verified)
            || !verified.is_file()
            || verified.len() != metadata.len()
            || modified_ms(&verified) != modified_ms(metadata)
        {
            stats.failed_item_count += 1;
            return false;
        }
        let released_bytes = metadata.len();
        let removed = match delete_path_permanently(prepared, released_bytes, 1) {
            Ok(()) => {
                // Metadata was captured immediately before deletion, so released
                // bytes can be accumulated without a second full directory walk.
                stats.deleted_bytes = stats.deleted_bytes.saturating_add(released_bytes);
                stats.affected_item_count += 1;
                true
            }
            Err(error) => {
                stats.deleted_bytes = stats.deleted_bytes.saturating_add(error.released_bytes());
                stats.affected_item_count = stats
                    .affected_item_count
                    .saturating_add(error.affected_item_count());
                stats.failed_item_count += 1;
                false
            }
        };
        (traversal.report_item)(path, stats);
        return removed;
    }

    let canonical_directory = match validate_cleanup_directory(path, traversal.canonical_root) {
        Ok(path) => path,
        Err(error) => {
            log::debug!(
                "cleanup_directory_validation_failed path={} error={}",
                diagnostic_path(path),
                error
            );
            stats.failed_item_count += 1;
            return false;
        }
    };
    if canonical_directory.parent() != Some(canonical_parent) {
        stats.failed_item_count += 1;
        return false;
    }
    let Ok(entries) = fs::read_dir(path) else {
        stats.failed_item_count += 1;
        return false;
    };
    if !revalidate_cleanup_directory(path, &canonical_directory) {
        stats.failed_item_count += 1;
        return false;
    }
    let mut all_removed = true;
    let mut had_entry = false;
    for entry in entries {
        if (traversal.is_cancelled)() {
            stats.failed_item_count += 1;
            return false;
        }
        had_entry = true;
        let Ok(entry) = entry else {
            stats.failed_item_count += 1;
            all_removed = false;
            continue;
        };
        if !revalidate_cleanup_directory(path, &canonical_directory) {
            stats.failed_item_count += 1;
            return false;
        }
        if !delete_entry(&entry.path(), &canonical_directory, stats, traversal) {
            all_removed = false;
        }
    }
    // A matcher authorizes only matching files. Removing a directory that was
    // already empty would expand scope through vacuous success. Prune a
    // directory only when it contained entries and this operation removed all
    // of them.
    if had_entry
        && all_removed
        && (!revalidate_cleanup_directory(path, &canonical_directory)
            || delete_empty_directory_permanently(prepared).is_err())
    {
        stats.failed_item_count = stats.failed_item_count.saturating_add(1);
        all_removed = false;
    }
    had_entry && all_removed
}

fn validate_cleanup_directory(path: &Path, canonical_root: &Path) -> Result<PathBuf, String> {
    current_platform()
        .validate_path_no_links(path)
        .map_err(|error| error.to_string())?;
    let metadata = fs::symlink_metadata(path).map_err(|error| error.to_string())?;
    if !metadata.is_dir() || is_link_like(&metadata) {
        return Err("the cleanup path is no longer a regular directory".to_string());
    }
    let canonical = current_platform()
        .canonicalize_no_links(path)
        .map_err(|error| error.to_string())?;
    if canonical != canonical_root && !canonical.starts_with(canonical_root) {
        return Err("the cleanup path escaped the rule root".to_string());
    }
    Ok(canonical)
}

fn revalidate_cleanup_directory(path: &Path, expected_canonical: &Path) -> bool {
    if current_platform().validate_path_no_links(path).is_err() {
        return false;
    }
    let Ok(metadata) = fs::symlink_metadata(path) else {
        return false;
    };
    metadata.is_dir()
        && !is_link_like(&metadata)
        && fs::canonicalize(path).is_ok_and(|canonical| canonical == expected_canonical)
}

#[derive(Default)]
pub(super) struct DeleteStats {
    pub(super) matched_bytes: u64,
    pub(super) deleted_bytes: u64,
    pub(super) affected_item_count: u64,
    pub(super) failed_item_count: u64,
}
