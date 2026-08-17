use std::{
    collections::BTreeMap,
    fs,
    path::{Path, PathBuf},
    process::Command,
    time::{Instant, SystemTime, UNIX_EPOCH},
};

use plist::{Dictionary, Value};

use crate::{
    PlatformCancellation, PlatformError, PlatformErrorCode, PlatformResult,
    PlatformStartupArtifact, PlatformStartupChangeRequest, PlatformStartupChangeResult,
    PlatformStartupConfiguredState, PlatformStartupControlCapability,
    PlatformStartupCoverageReason, PlatformStartupCoverageStatus, PlatformStartupDesiredState,
    PlatformStartupDiagnosticCode, PlatformStartupIdentityConfidence, PlatformStartupOwner,
    PlatformStartupRuntimeState, PlatformStartupScope, PlatformStartupSourceKind,
    PlatformStartupSourceResult, PlatformStartupSummarySource, PlatformStartupTarget,
    PlatformStartupTargetKind, PlatformStartupTrigger, PlatformStartupTrustState,
};

use super::metadata::code_signature_metadata;

struct LaunchdSource {
    source_id: &'static str,
    paths: Vec<PathBuf>,
    source_kind: PlatformStartupSourceKind,
    scope: PlatformStartupScope,
    capability: PlatformStartupControlCapability,
    domain: LaunchdDomain,
    system_owned: bool,
}

#[derive(Clone, Copy)]
enum LaunchdDomain {
    Gui,
    System,
}

#[derive(Clone, Copy)]
enum ScanIssue {
    AccessDenied,
    InvalidData,
    StateUnavailable,
}

pub(super) fn scan(cancellation: &PlatformCancellation) -> Vec<PlatformStartupSourceResult> {
    let sources = launchd_sources();
    let gui_overrides = disabled_overrides(LaunchdDomain::Gui, None);
    let system_overrides = disabled_overrides(LaunchdDomain::System, None);

    sources
        .iter()
        .map(|source| {
            if cancellation.is_cancelled() {
                return PlatformStartupSourceResult {
                    source_id: source.source_id.to_owned(),
                    required: true,
                    status: PlatformStartupCoverageStatus::Cancelled,
                    reason: Some(PlatformStartupCoverageReason::Cancelled),
                    items: Vec::new(),
                    elapsed_ms: 0,
                };
            }
            let overrides = match source.domain {
                LaunchdDomain::Gui => gui_overrides.as_ref(),
                LaunchdDomain::System => system_overrides.as_ref(),
            };
            scan_source(source, overrides, cancellation)
        })
        .collect()
}

fn launchd_sources() -> Vec<LaunchdSource> {
    let home = dirs::home_dir();
    vec![
        LaunchdSource {
            source_id: "macos.launchd.user_agents",
            paths: home
                .map(|path| vec![path.join("Library/LaunchAgents")])
                .unwrap_or_default(),
            source_kind: PlatformStartupSourceKind::LaunchAgent,
            scope: PlatformStartupScope::CurrentUser,
            capability: PlatformStartupControlCapability::Toggleable,
            domain: LaunchdDomain::Gui,
            system_owned: false,
        },
        LaunchdSource {
            source_id: "macos.launchd.local_agents",
            paths: vec![PathBuf::from("/Library/LaunchAgents")],
            source_kind: PlatformStartupSourceKind::LaunchAgent,
            scope: PlatformStartupScope::Machine,
            capability: PlatformStartupControlCapability::ElevationRequired,
            domain: LaunchdDomain::Gui,
            system_owned: false,
        },
        LaunchdSource {
            source_id: "macos.launchd.local_daemons",
            paths: vec![PathBuf::from("/Library/LaunchDaemons")],
            source_kind: PlatformStartupSourceKind::LaunchDaemon,
            scope: PlatformStartupScope::Machine,
            capability: PlatformStartupControlCapability::ElevationRequired,
            domain: LaunchdDomain::System,
            system_owned: false,
        },
        LaunchdSource {
            source_id: "macos.launchd.system_agents",
            paths: vec![
                PathBuf::from("/System/Library/LaunchAgents"),
                PathBuf::from("/Library/Apple/System/Library/LaunchAgents"),
            ],
            source_kind: PlatformStartupSourceKind::LaunchAgent,
            scope: PlatformStartupScope::System,
            capability: PlatformStartupControlCapability::ViewOnly,
            domain: LaunchdDomain::Gui,
            system_owned: true,
        },
        LaunchdSource {
            source_id: "macos.launchd.system_daemons",
            paths: vec![
                PathBuf::from("/System/Library/LaunchDaemons"),
                PathBuf::from("/Library/Apple/System/Library/LaunchDaemons"),
            ],
            source_kind: PlatformStartupSourceKind::LaunchDaemon,
            scope: PlatformStartupScope::System,
            capability: PlatformStartupControlCapability::ViewOnly,
            domain: LaunchdDomain::System,
            system_owned: true,
        },
    ]
}

pub(super) fn change(
    request: &PlatformStartupChangeRequest,
) -> PlatformResult<PlatformStartupChangeResult> {
    change_with_context(request, None, false)
}

pub(super) fn privileged_change(
    request: &PlatformStartupChangeRequest,
    interactive_user_id: u32,
) -> PlatformResult<PlatformStartupChangeResult> {
    change_with_context(request, Some(interactive_user_id), true)
}

fn change_with_context(
    request: &PlatformStartupChangeRequest,
    interactive_user_id: Option<u32>,
    privileged: bool,
) -> PlatformResult<PlatformStartupChangeResult> {
    let source = launchd_sources()
        .into_iter()
        .find(|source| source.source_id == request.source_id)
        .ok_or_else(|| {
            PlatformError::new(
                PlatformErrorCode::Unsupported,
                "launchd source is not available for configured-state changes",
            )
        })?;
    if (!privileged && source.scope != PlatformStartupScope::CurrentUser)
        || (privileged
            && !matches!(
                source.source_id,
                "macos.launchd.local_agents" | "macos.launchd.local_daemons"
            ))
    {
        return Err(PlatformError::new(
            PlatformErrorCode::AccessDenied,
            "launchd item requires a privileged operation",
        ));
    }
    let overrides = disabled_overrides(source.domain, interactive_user_id).ok_or_else(|| {
        PlatformError::new(
            PlatformErrorCode::OperationFailed,
            "launchd disabled state is unavailable",
        )
    })?;
    let (label, current) = find_item(&source, &overrides, &request.provider_item_id)?
        .ok_or_else(|| PlatformError::item_changed("launchd item no longer exists"))?;
    if current != request.expected_artifact {
        return Err(PlatformError::item_changed(
            "launchd item changed after preflight",
        ));
    }
    let expected_capability = if privileged {
        PlatformStartupControlCapability::ElevationRequired
    } else {
        PlatformStartupControlCapability::Toggleable
    };
    if current.control_capability != expected_capability {
        return Err(PlatformError::new(
            PlatformErrorCode::Unsupported,
            "launchd item is not toggleable",
        ));
    }
    let desired = match request.desired_state {
        PlatformStartupDesiredState::Enabled => PlatformStartupConfiguredState::Enabled,
        PlatformStartupDesiredState::Disabled => PlatformStartupConfiguredState::Disabled,
    };
    if current.configured_state != desired {
        let action = match request.desired_state {
            PlatformStartupDesiredState::Enabled => "enable",
            PlatformStartupDesiredState::Disabled => "disable",
        };
        let target = match source.domain {
            LaunchdDomain::Gui => format!(
                "gui/{}/{}",
                interactive_user_id.unwrap_or_else(|| unsafe { libc::geteuid() }),
                label
            ),
            LaunchdDomain::System => format!("system/{label}"),
        };
        let output = Command::new("/bin/launchctl")
            .args([action, &target])
            .output()
            .map_err(|error| PlatformError::io("change launchd configured state", &error))?;
        if !output.status.success() {
            return Err(PlatformError::new(
                PlatformErrorCode::OperationFailed,
                "launchd rejected the configured-state change",
            ));
        }
    }
    let verified_overrides =
        disabled_overrides(source.domain, interactive_user_id).ok_or_else(|| {
            PlatformError::new(
                PlatformErrorCode::OperationFailed,
                "launchd state verification is unavailable",
            )
        })?;
    let (_, verified) = find_item(&source, &verified_overrides, &request.provider_item_id)?
        .ok_or_else(|| {
            PlatformError::item_changed("launchd item disappeared during verification")
        })?;
    Ok(PlatformStartupChangeResult {
        previous_state: current.configured_state,
        configured_state: verified.configured_state,
        verified: verified.configured_state == desired,
    })
}

fn find_item(
    source: &LaunchdSource,
    overrides: &BTreeMap<String, bool>,
    provider_item_id: &str,
) -> PlatformResult<Option<(String, PlatformStartupArtifact)>> {
    for directory in &source.paths {
        let entries = match fs::read_dir(directory) {
            Ok(entries) => entries,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => continue,
            Err(error) => return Err(PlatformError::io("read launchd source", &error)),
        };
        for entry in entries {
            let Ok(entry) = entry else {
                continue;
            };
            let path = entry.path();
            if !path
                .extension()
                .is_some_and(|extension| extension.eq_ignore_ascii_case("plist"))
            {
                continue;
            }
            let Ok(metadata) = fs::symlink_metadata(&path) else {
                continue;
            };
            if !metadata.is_file() || metadata.file_type().is_symlink() {
                continue;
            }
            let Ok(value) = Value::from_file(&path) else {
                continue;
            };
            let Some(dictionary) = value.as_dictionary() else {
                continue;
            };
            let artifact = artifact_from_dictionary(
                dictionary,
                &path,
                metadata.modified().ok(),
                source,
                Some(overrides),
            );
            if artifact.provider_item_id == provider_item_id {
                let label = string_value(dictionary, "Label").ok_or_else(|| {
                    PlatformError::new(
                        PlatformErrorCode::InvalidData,
                        "launchd item has no service label",
                    )
                })?;
                return Ok(Some((label, artifact)));
            }
        }
    }
    Ok(None)
}

fn scan_source(
    source: &LaunchdSource,
    overrides: Option<&BTreeMap<String, bool>>,
    cancellation: &PlatformCancellation,
) -> PlatformStartupSourceResult {
    let started = Instant::now();
    let mut items = Vec::new();
    let mut issues = if overrides.is_none() {
        vec![ScanIssue::StateUnavailable]
    } else {
        Vec::new()
    };

    for path in &source.paths {
        if cancellation.is_cancelled() {
            return PlatformStartupSourceResult {
                source_id: source.source_id.to_owned(),
                required: true,
                status: PlatformStartupCoverageStatus::Cancelled,
                reason: Some(PlatformStartupCoverageReason::Cancelled),
                items,
                elapsed_ms: started.elapsed().as_millis() as u64,
            };
        }
        let entries = match fs::read_dir(path) {
            Ok(entries) => entries,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => continue,
            Err(error) if error.kind() == std::io::ErrorKind::PermissionDenied => {
                issues.push(ScanIssue::AccessDenied);
                continue;
            }
            Err(_) => {
                issues.push(ScanIssue::InvalidData);
                continue;
            }
        };
        for entry in entries {
            if cancellation.is_cancelled() {
                return PlatformStartupSourceResult {
                    source_id: source.source_id.to_owned(),
                    required: true,
                    status: PlatformStartupCoverageStatus::Cancelled,
                    reason: Some(PlatformStartupCoverageReason::Cancelled),
                    items,
                    elapsed_ms: started.elapsed().as_millis() as u64,
                };
            }
            let entry = match entry {
                Ok(entry) => entry,
                Err(_) => {
                    issues.push(ScanIssue::InvalidData);
                    continue;
                }
            };
            let plist_path = entry.path();
            if !plist_path
                .extension()
                .is_some_and(|extension| extension.eq_ignore_ascii_case("plist"))
            {
                continue;
            }
            let Ok(metadata) = fs::symlink_metadata(&plist_path) else {
                issues.push(ScanIssue::InvalidData);
                continue;
            };
            if !metadata.is_file() || metadata.file_type().is_symlink() {
                continue;
            }
            let value = match Value::from_file(&plist_path) {
                Ok(value) => value,
                Err(_) => {
                    issues.push(ScanIssue::InvalidData);
                    continue;
                }
            };
            let Some(dictionary) = value.as_dictionary() else {
                issues.push(ScanIssue::InvalidData);
                continue;
            };
            if !has_launchd_identity(dictionary) {
                // Some uninstallers leave an empty launchd property list behind. It is not a
                // runnable startup item and cannot be changed safely, so keep it out of the
                // user-facing catalog while retaining partial-coverage diagnostics.
                issues.push(ScanIssue::InvalidData);
                continue;
            }
            items.push(artifact_from_dictionary(
                dictionary,
                &plist_path,
                metadata.modified().ok(),
                source,
                overrides,
            ));
        }
    }

    let reason = coverage_reason(&issues);
    PlatformStartupSourceResult {
        source_id: source.source_id.to_owned(),
        required: true,
        status: if reason.is_some() {
            PlatformStartupCoverageStatus::Partial
        } else {
            PlatformStartupCoverageStatus::Complete
        },
        reason,
        items,
        elapsed_ms: started.elapsed().as_millis() as u64,
    }
}

fn has_launchd_identity(dictionary: &Dictionary) -> bool {
    string_value(dictionary, "Label").is_some()
        || string_value(dictionary, "Program").is_some()
        || !string_array(dictionary, "ProgramArguments").is_empty()
}

fn artifact_from_dictionary(
    dictionary: &Dictionary,
    plist_path: &Path,
    modified: Option<SystemTime>,
    source: &LaunchdSource,
    overrides: Option<&BTreeMap<String, bool>>,
) -> PlatformStartupArtifact {
    let label = string_value(dictionary, "Label");
    let arguments = string_array(dictionary, "ProgramArguments");
    let program = string_value(dictionary, "Program").or_else(|| arguments.first().cloned());
    let target_path = program.as_deref().map(PathBuf::from);
    let (owner, trust) = target_path
        .as_deref()
        .map(|target| owner_from_target(target, label.as_deref(), source.system_owned))
        .unwrap_or_else(|| {
            (
                unresolved_owner(),
                if source.system_owned {
                    PlatformStartupTrustState::System
                } else {
                    PlatformStartupTrustState::Unknown
                },
            )
        });
    let display_name = owner
        .name
        .clone()
        .or_else(|| label.clone())
        .or_else(|| {
            plist_path
                .file_stem()
                .map(|value| value.to_string_lossy().into_owned())
        })
        .unwrap_or_else(|| "launchd item".to_owned());
    let mut diagnostics = Vec::new();
    if label.is_none() {
        diagnostics.push(PlatformStartupDiagnosticCode::MissingIdentity);
    }
    if target_path.is_none() {
        diagnostics.push(PlatformStartupDiagnosticCode::MissingTarget);
    }
    if overrides.is_none() {
        diagnostics.push(PlatformStartupDiagnosticCode::StateUnavailable);
    }
    let configured_state = configured_state(dictionary, label.as_deref(), overrides);
    let target_identity = target_path
        .as_deref()
        .map(|path| path.to_string_lossy().into_owned())
        .unwrap_or_else(|| {
            label
                .clone()
                .unwrap_or_else(|| plist_path.to_string_lossy().into_owned())
        });

    PlatformStartupArtifact {
        provider_item_id: format!(
            "{}:{}",
            label.as_deref().unwrap_or("missing-label"),
            plist_path.to_string_lossy()
        ),
        source_kind: source.source_kind,
        scope: source.scope,
        triggers: launchd_triggers(dictionary, source.source_kind),
        display_name,
        configuration_path: Some(plist_path.to_path_buf()),
        target: PlatformStartupTarget {
            kind: target_kind(target_path.as_deref()),
            identity_key: target_identity,
            path: target_path.clone(),
            executable_name: target_path.as_deref().and_then(|path| {
                path.file_name()
                    .map(|value| value.to_string_lossy().into_owned())
            }),
            arguments: if arguments.is_empty() {
                Vec::new()
            } else {
                arguments.into_iter().skip(1).collect()
            },
        },
        owner,
        configured_state,
        runtime_state: PlatformStartupRuntimeState::Unknown,
        control_capability: if label.is_some() {
            source.capability
        } else {
            PlatformStartupControlCapability::ViewOnly
        },
        trust,
        modified_at_ms: modified.and_then(system_time_ms),
        diagnostics,
    }
}

fn configured_state(
    dictionary: &Dictionary,
    label: Option<&str>,
    overrides: Option<&BTreeMap<String, bool>>,
) -> PlatformStartupConfiguredState {
    if let (Some(label), Some(overrides)) = (label, overrides) {
        if let Some(disabled) = overrides.get(label) {
            return if *disabled {
                PlatformStartupConfiguredState::Disabled
            } else {
                PlatformStartupConfiguredState::Enabled
            };
        }
    }
    match dictionary.get("Disabled").and_then(Value::as_boolean) {
        Some(true) => PlatformStartupConfiguredState::Disabled,
        Some(false) => PlatformStartupConfiguredState::Enabled,
        None if overrides.is_some() => PlatformStartupConfiguredState::Enabled,
        None => PlatformStartupConfiguredState::Unknown,
    }
}

fn launchd_triggers(
    dictionary: &Dictionary,
    source_kind: PlatformStartupSourceKind,
) -> Vec<PlatformStartupTrigger> {
    let mut triggers = Vec::new();
    if dictionary
        .get("RunAtLoad")
        .and_then(Value::as_boolean)
        .unwrap_or(false)
    {
        triggers.push(if source_kind == PlatformStartupSourceKind::LaunchDaemon {
            PlatformStartupTrigger::Boot
        } else {
            PlatformStartupTrigger::UserLogon
        });
    }
    if dictionary.contains_key("KeepAlive") {
        triggers.push(PlatformStartupTrigger::KeepAlive);
    }
    if dictionary.contains_key("StartInterval") || dictionary.contains_key("StartCalendarInterval")
    {
        triggers.push(PlatformStartupTrigger::Scheduled);
    }
    if [
        "WatchPaths",
        "QueueDirectories",
        "MachServices",
        "Sockets",
        "StartOnMount",
    ]
    .into_iter()
    .any(|key| dictionary.contains_key(key))
    {
        triggers.push(PlatformStartupTrigger::Event);
    }
    if triggers.is_empty() {
        triggers.push(PlatformStartupTrigger::Unknown);
    }
    triggers.sort_unstable();
    triggers.dedup();
    triggers
}

fn owner_from_target(
    target: &Path,
    label: Option<&str>,
    system_candidate: bool,
) -> (PlatformStartupOwner, PlatformStartupTrustState) {
    let signature = code_signature_metadata(target, system_candidate);
    if let Some(owner) = super::bundle_index::resolve_owner(label, signature.team_id.as_deref()) {
        return (owner, signature.trust);
    }
    let Some(bundle) = target.ancestors().find(|candidate| {
        candidate
            .extension()
            .is_some_and(|extension| extension.eq_ignore_ascii_case("app"))
    }) else {
        let owner = PlatformStartupOwner {
            identity_key: signature
                .team_id
                .as_ref()
                .map(|team_id| format!("team:{team_id}")),
            name: target
                .file_name()
                .map(|value| value.to_string_lossy().into_owned()),
            publisher: signature.publisher,
            summary: None,
            summary_source: PlatformStartupSummarySource::Unavailable,
            version: None,
            icon_path: target.exists().then(|| target.to_path_buf()),
            confidence: if signature.team_id.is_some() {
                PlatformStartupIdentityConfidence::Strong
            } else {
                PlatformStartupIdentityConfidence::Unresolved
            },
        };
        return (owner, signature.trust);
    };
    let Ok(value) = Value::from_file(bundle.join("Contents/Info.plist")) else {
        return (unresolved_owner(), PlatformStartupTrustState::Unknown);
    };
    let Some(dictionary) = value.as_dictionary() else {
        return (unresolved_owner(), PlatformStartupTrustState::Unknown);
    };
    let identity_key = string_value(dictionary, "CFBundleIdentifier")
        .map(|bundle_id| format!("bundle:{bundle_id}"));
    let name = string_value(dictionary, "CFBundleDisplayName")
        .or_else(|| string_value(dictionary, "CFBundleName"))
        .or_else(|| {
            bundle
                .file_stem()
                .map(|value| value.to_string_lossy().into_owned())
        });
    (
        PlatformStartupOwner {
            confidence: if identity_key.is_some() {
                PlatformStartupIdentityConfidence::Exact
            } else {
                PlatformStartupIdentityConfidence::Strong
            },
            identity_key,
            name,
            publisher: signature.publisher,
            summary: None,
            summary_source: PlatformStartupSummarySource::BundleMetadata,
            version: string_value(dictionary, "CFBundleShortVersionString"),
            icon_path: Some(bundle.to_path_buf()),
        },
        signature.trust,
    )
}

fn unresolved_owner() -> PlatformStartupOwner {
    PlatformStartupOwner {
        identity_key: None,
        name: None,
        publisher: None,
        summary: None,
        summary_source: PlatformStartupSummarySource::Unavailable,
        version: None,
        icon_path: None,
        confidence: PlatformStartupIdentityConfidence::Unresolved,
    }
}

fn target_kind(path: Option<&Path>) -> PlatformStartupTargetKind {
    let Some(path) = path else {
        return PlatformStartupTargetKind::Unknown;
    };
    if path
        .extension()
        .is_some_and(|extension| extension.eq_ignore_ascii_case("app"))
        || path.ancestors().any(|candidate| {
            candidate
                .extension()
                .is_some_and(|extension| extension.eq_ignore_ascii_case("app"))
        })
    {
        return PlatformStartupTargetKind::Application;
    }
    if path.extension().is_some_and(|extension| {
        ["sh", "zsh", "bash", "py", "pl", "rb", "js"]
            .into_iter()
            .any(|value| extension.eq_ignore_ascii_case(value))
    }) {
        PlatformStartupTargetKind::Script
    } else {
        PlatformStartupTargetKind::Executable
    }
}

fn disabled_overrides(
    domain: LaunchdDomain,
    interactive_user_id: Option<u32>,
) -> Option<BTreeMap<String, bool>> {
    let target = match domain {
        LaunchdDomain::Gui => format!(
            "gui/{}",
            interactive_user_id.unwrap_or_else(|| unsafe { libc::geteuid() })
        ),
        LaunchdDomain::System => "system".to_owned(),
    };
    let output = Command::new("/bin/launchctl")
        .args(["print-disabled", &target])
        .output()
        .ok()?;
    if !output.status.success() {
        return None;
    }
    let text = String::from_utf8(output.stdout).ok()?;
    Some(parse_disabled_overrides(&text))
}

fn parse_disabled_overrides(text: &str) -> BTreeMap<String, bool> {
    let mut values = BTreeMap::new();
    for line in text.lines() {
        let Some((label, disabled)) = line.split_once("=>") else {
            continue;
        };
        let label = label.trim().trim_matches('"');
        let disabled = disabled.trim().trim_end_matches([';', ',']).trim();
        if label.is_empty() {
            continue;
        }
        match disabled {
            // Current launchctl versions print words while older releases used booleans.
            // Accepting both formats is required because this value is the verification source
            // immediately after an enable or disable operation.
            "true" | "disabled" => {
                values.insert(label.to_owned(), true);
            }
            "false" | "enabled" => {
                values.insert(label.to_owned(), false);
            }
            _ => {}
        }
    }
    values
}

fn coverage_reason(issues: &[ScanIssue]) -> Option<PlatformStartupCoverageReason> {
    if issues
        .iter()
        .any(|issue| matches!(issue, ScanIssue::AccessDenied))
    {
        Some(PlatformStartupCoverageReason::AccessDenied)
    } else if issues
        .iter()
        .any(|issue| matches!(issue, ScanIssue::InvalidData))
    {
        Some(PlatformStartupCoverageReason::InvalidData)
    } else if issues
        .iter()
        .any(|issue| matches!(issue, ScanIssue::StateUnavailable))
    {
        Some(PlatformStartupCoverageReason::StateUnavailable)
    } else {
        None
    }
}

fn string_value(dictionary: &Dictionary, key: &str) -> Option<String> {
    dictionary
        .get(key)
        .and_then(Value::as_string)
        .filter(|value| !value.trim().is_empty())
        .map(str::to_owned)
}

fn string_array(dictionary: &Dictionary, key: &str) -> Vec<String> {
    dictionary
        .get(key)
        .and_then(Value::as_array)
        .map(|values| {
            values
                .iter()
                .filter_map(Value::as_string)
                .map(str::to_owned)
                .collect()
        })
        .unwrap_or_default()
}

fn system_time_ms(value: SystemTime) -> Option<u64> {
    value
        .duration_since(UNIX_EPOCH)
        .ok()
        .map(|duration| duration.as_millis() as u64)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn source(kind: PlatformStartupSourceKind) -> LaunchdSource {
        LaunchdSource {
            source_id: "test.launchd",
            paths: Vec::new(),
            source_kind: kind,
            scope: PlatformStartupScope::CurrentUser,
            capability: PlatformStartupControlCapability::Toggleable,
            domain: LaunchdDomain::Gui,
            system_owned: false,
        }
    }

    #[test]
    fn run_at_load_uses_scope_appropriate_trigger() {
        let mut dictionary = Dictionary::new();
        dictionary.insert("RunAtLoad".to_owned(), Value::Boolean(true));

        assert_eq!(
            launchd_triggers(&dictionary, PlatformStartupSourceKind::LaunchAgent),
            vec![PlatformStartupTrigger::UserLogon]
        );
        assert_eq!(
            launchd_triggers(&dictionary, PlatformStartupSourceKind::LaunchDaemon),
            vec![PlatformStartupTrigger::Boot]
        );
    }

    #[test]
    fn event_only_item_is_not_reported_as_login_startup() {
        let mut dictionary = Dictionary::new();
        dictionary.insert(
            "WatchPaths".to_owned(),
            Value::Array(vec![Value::String("/tmp/example".to_owned())]),
        );

        assert_eq!(
            launchd_triggers(&dictionary, PlatformStartupSourceKind::LaunchAgent),
            vec![PlatformStartupTrigger::Event]
        );
    }

    #[test]
    fn disabled_override_takes_precedence_over_plist_default() {
        let mut dictionary = Dictionary::new();
        dictionary.insert("Disabled".to_owned(), Value::Boolean(false));
        let overrides = BTreeMap::from([("com.example.agent".to_owned(), true)]);

        assert_eq!(
            configured_state(&dictionary, Some("com.example.agent"), Some(&overrides)),
            PlatformStartupConfiguredState::Disabled
        );
    }

    #[test]
    fn disabled_overrides_accept_current_launchctl_words() {
        let values = parse_disabled_overrides(
            r#"
disabled services = {
    "homebrew.mxcl.mysql@8.4" => disabled,
    "com.example.enabled" => enabled
}
"#,
        );

        assert_eq!(values.get("homebrew.mxcl.mysql@8.4"), Some(&true));
        assert_eq!(values.get("com.example.enabled"), Some(&false));
    }

    #[test]
    fn disabled_overrides_keep_legacy_boolean_compatibility() {
        let values = parse_disabled_overrides(
            r#"
disabled services = {
    "com.example.disabled" => true;
    "com.example.enabled" => false;
}
"#,
        );

        assert_eq!(values.get("com.example.disabled"), Some(&true));
        assert_eq!(values.get("com.example.enabled"), Some(&false));
    }

    #[test]
    fn missing_target_remains_visible_with_diagnostics() {
        let mut dictionary = Dictionary::new();
        dictionary.insert(
            "Label".to_owned(),
            Value::String("com.example.missing".to_owned()),
        );

        let artifact = artifact_from_dictionary(
            &dictionary,
            Path::new("/Library/LaunchAgents/com.example.missing.plist"),
            None,
            &source(PlatformStartupSourceKind::LaunchAgent),
            Some(&BTreeMap::new()),
        );

        assert_eq!(artifact.display_name, "com.example.missing");
        assert_eq!(
            artifact.configuration_path.as_deref(),
            Some(Path::new("/Library/LaunchAgents/com.example.missing.plist"))
        );
        assert!(artifact
            .diagnostics
            .contains(&PlatformStartupDiagnosticCode::MissingTarget));
        assert_eq!(artifact.target.kind, PlatformStartupTargetKind::Unknown);
    }

    #[test]
    fn missing_launchd_label_is_visible_but_not_manageable() {
        let mut dictionary = Dictionary::new();
        dictionary.insert(
            "Program".to_owned(),
            Value::String("/usr/bin/example".to_owned()),
        );

        let artifact = artifact_from_dictionary(
            &dictionary,
            Path::new("/Library/LaunchAgents/example.plist"),
            None,
            &source(PlatformStartupSourceKind::LaunchAgent),
            Some(&BTreeMap::new()),
        );

        assert!(artifact
            .diagnostics
            .contains(&PlatformStartupDiagnosticCode::MissingIdentity));
        assert_eq!(
            artifact.control_capability,
            PlatformStartupControlCapability::ViewOnly
        );
    }

    #[test]
    fn empty_launchd_property_lists_are_not_startup_items() {
        assert!(!has_launchd_identity(&Dictionary::new()));

        let mut labeled = Dictionary::new();
        labeled.insert(
            "Label".to_owned(),
            Value::String("com.example.agent".to_owned()),
        );
        assert!(has_launchd_identity(&labeled));

        let mut argument_only = Dictionary::new();
        argument_only.insert(
            "ProgramArguments".to_owned(),
            Value::Array(vec![Value::String("/usr/bin/example".to_owned())]),
        );
        assert!(has_launchd_identity(&argument_only));
    }
}
