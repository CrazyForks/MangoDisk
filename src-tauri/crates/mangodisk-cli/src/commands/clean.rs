use std::{
    io::{self, IsTerminal, Write},
    path::Path,
};

use mangodisk_core::{
    diagnostic_path, CleanupGroup, CleanupRequest, CleanupScanResult, CleanupScanService,
    CleanupService, OperationCancellationToken, ScanRuleResult,
};
use serde_json::json;

use crate::{
    arguments::{CleanArgs, CleanSelection, OutputFormat},
    commands::{CliFailure, CommandContext},
    exit_code::CliExitCode,
    output::CommandOutcome,
    progress::CliProgressSink,
};

const GROUP_ORDER: [CleanupGroup; 10] = [
    CleanupGroup::System,
    CleanupGroup::UserCache,
    CleanupGroup::Browser,
    CleanupGroup::Application,
    CleanupGroup::Development,
    CleanupGroup::Project,
    CleanupGroup::Xcode,
    CleanupGroup::ApplicationOptimization,
    CleanupGroup::Ai,
    CleanupGroup::Container,
];

pub fn run(
    arguments: CleanArgs,
    context: &CommandContext<'_>,
) -> Result<CommandOutcome, CliFailure> {
    let scan = {
        let _active = context
            .cancellation
            .activate(OperationCancellationToken::cleanup_scan());
        CleanupScanService::scan_with_deep_project_discovery(
            arguments.deep_project_discovery,
            CliProgressSink::new(
                context.format,
                context.progress_enabled,
                context.include_full_paths,
            ),
        )?
    };
    let scan_summary = render_scan(
        &scan,
        arguments.details,
        context.include_full_paths,
        context.color_enabled,
    );

    if !arguments.apply {
        return CommandOutcome::success(
            "clean.scan",
            format!("{scan_summary}\n\nNo files changed."),
            scan,
        )
        .map_err(Into::into);
    }

    let selection = arguments.selection.unwrap_or(CleanSelection::Recommended);
    let selected_rules = selected_rules(&scan, selection);
    let selected_rule_ids = selected_rules
        .iter()
        .map(|rule| rule.rule_id.clone())
        .collect::<Vec<_>>();
    let selected_bytes = selected_rules.iter().map(|rule| rule.bytes).sum::<u64>();

    if selected_rule_ids.is_empty() {
        return CommandOutcome::success(
            "clean.apply",
            format!(
                "{scan_summary}\n\nNo {} cleanup items are currently available. No files changed.",
                selection_label(selection)
            ),
            json!({
                "scan": scan,
                "selection": selection_label(selection),
                "selectedRuleIds": selected_rule_ids,
                "selectedBytes": selected_bytes,
                "cleanup": null,
            }),
        )
        .map_err(Into::into);
    }

    if context.format == OutputFormat::Human {
        println!("{scan_summary}");
        eprintln!(
            "\nSelected {} item(s), {} using the {} selection.",
            selected_rule_ids.len(),
            format_bytes(selected_bytes),
            selection_label(selection)
        );
    }

    if !arguments.dry_run
        && !arguments.yes
        && !confirm_cleanup(
            context.format,
            selection,
            selected_rule_ids.len(),
            selected_bytes,
        )?
    {
        let mut outcome = CommandOutcome::success(
            "clean.apply",
            "Cleanup cancelled. No files changed.",
            json!({
                "scan": scan,
                "selection": selection_label(selection),
                "selectedRuleIds": selected_rule_ids,
                "selectedBytes": selected_bytes,
                "cleanup": null,
                "cancelled": true,
            }),
        )
        .map_err(CliFailure::from)?;
        outcome.exit_code = CliExitCode::Cancelled;
        return Ok(outcome);
    }

    let request = CleanupRequest {
        rule_ids: selected_rule_ids.clone(),
        dry_run: arguments.dry_run,
        project_roots: Vec::new(),
        source_selections: Vec::new(),
    };
    let result = {
        let _active = context
            .cancellation
            .activate(OperationCancellationToken::cleanup());
        CleanupService::execute(request)?
    };
    let exit_code = if context.cancellation.was_cancelled() {
        CliExitCode::Cancelled
    } else if result.failed_item_count > 0 {
        CliExitCode::CompletedWithWarnings
    } else {
        CliExitCode::Success
    };
    let action = if result.dry_run {
        "Dry run completed"
    } else {
        "Cleanup completed"
    };
    let human = if result.dry_run {
        format!(
            "{action}: {} selected, {} item(s) passed preflight, {} item(s) skipped or failed.",
            format_bytes(selected_bytes),
            result.affected_item_count,
            result.failed_item_count
        )
    } else {
        format!(
            "{action}: {} reclaimed, {} item(s) affected, {} item(s) skipped or failed.",
            format_bytes(result.released_bytes),
            result.affected_item_count,
            result.failed_item_count
        )
    };
    let mut outcome = CommandOutcome::success(
        "clean.apply",
        human,
        json!({
            "scan": scan,
            "selection": selection_label(selection),
            "selectedRuleIds": selected_rule_ids,
            "selectedBytes": selected_bytes,
            "cleanup": result,
        }),
    )
    .map_err(CliFailure::from)?;
    outcome.exit_code = exit_code;
    Ok(outcome)
}

fn selected_rules(scan: &CleanupScanResult, selection: CleanSelection) -> Vec<&ScanRuleResult> {
    scan.rules
        .iter()
        .filter(|rule| {
            rule.selectable
                && rule.bytes > 0
                && match selection {
                    CleanSelection::Recommended => rule.recommended_selected,
                    CleanSelection::All => true,
                }
        })
        .collect()
}

fn confirm_cleanup(
    format: OutputFormat,
    selection: CleanSelection,
    selected_count: usize,
    selected_bytes: u64,
) -> Result<bool, CliFailure> {
    if format != OutputFormat::Human || !io::stdin().is_terminal() {
        return Err(CliFailure::confirmation_required(
            "non-interactive cleanup requires --yes; use --dry-run to verify without changes",
        ));
    }

    let warning = match selection {
        CleanSelection::Recommended => "Apply the recommended cleanup",
        CleanSelection::All => "Apply all selectable items, including items that require review",
    };
    eprint!(
        "{warning} ({selected_count} item(s), {})? [y/N] ",
        format_bytes(selected_bytes)
    );
    io::stderr().flush()?;
    let mut response = String::new();
    io::stdin().read_line(&mut response)?;
    Ok(matches!(
        response.trim().to_ascii_lowercase().as_str(),
        "y" | "yes"
    ))
}

fn render_scan(
    scan: &CleanupScanResult,
    details: bool,
    include_full_paths: bool,
    color_enabled: bool,
) -> String {
    let mut lines = vec![
        style("Cleanup scan", "1;36", color_enabled),
        format!(
            "{} candidate(s), {} reclaimable, completed in {}.",
            scan.rules
                .iter()
                .filter(|rule| rule.selectable && rule.bytes > 0)
                .count(),
            style(&format_bytes(scan.reclaimable_bytes), "1;32", color_enabled),
            format_duration(scan.elapsed_ms)
        ),
    ];

    for group in GROUP_ORDER {
        let mut rules = scan
            .rules
            .iter()
            .filter(|rule| rule.group == group && rule.bytes > 0)
            .collect::<Vec<_>>();
        if rules.is_empty() {
            continue;
        }
        rules.sort_by(|left, right| {
            right
                .bytes
                .cmp(&left.bytes)
                .then_with(|| left.rule_id.cmp(&right.rule_id))
        });
        let group_bytes = rules.iter().map(|rule| rule.bytes).sum::<u64>();
        lines.push(String::new());
        lines.push(style(
            &format!(
                "{} ({}, {} item(s))",
                group_label(group),
                format_bytes(group_bytes),
                rules.len()
            ),
            "1;34",
            color_enabled,
        ));
        for rule in rules {
            let marker = if rule.recommended_selected && rule.selectable {
                "recommended"
            } else if rule.selectable {
                "review"
            } else {
                "unavailable"
            };
            lines.push(format!(
                "  {:<44} {:>10}  {}",
                humanize_rule_id(&rule.rule_id),
                format_bytes(rule.bytes),
                marker
            ));
            if details {
                lines.push(format!(
                    "    {} file(s), {} source(s){}",
                    rule.file_count,
                    rule.source_count,
                    if rule.sources_truncated {
                        " (summary truncated)"
                    } else {
                        ""
                    }
                ));
                for source in &rule.sources {
                    let path = if include_full_paths {
                        source.path.clone()
                    } else {
                        diagnostic_path(Path::new(&source.path))
                    };
                    lines.push(format!("    - {}  {}", format_bytes(source.bytes), path));
                }
            }
        }
    }

    lines.join("\n")
}

const fn selection_label(selection: CleanSelection) -> &'static str {
    match selection {
        CleanSelection::Recommended => "recommended",
        CleanSelection::All => "all",
    }
}

const fn group_label(group: CleanupGroup) -> &'static str {
    match group {
        CleanupGroup::System => "System",
        CleanupGroup::UserCache => "User caches",
        CleanupGroup::Browser => "Browser data",
        CleanupGroup::Application => "Application caches",
        CleanupGroup::Development => "Developer tools",
        CleanupGroup::Project => "Project build artifacts",
        CleanupGroup::Xcode => "Xcode data",
        CleanupGroup::ApplicationOptimization => "Application optimization",
        CleanupGroup::Ai => "AI models and caches",
        CleanupGroup::Container => "Container data",
    }
}

fn humanize_rule_id(rule_id: &str) -> String {
    let value = rule_id.rsplit('.').next().unwrap_or(rule_id);
    value
        .split(['-', '_'])
        .filter(|part| !part.is_empty())
        .map(|part| match part.to_ascii_lowercase().as_str() {
            "ai" => "AI".to_string(),
            "ios" => "iOS".to_string(),
            "macos" => "macOS".to_string(),
            "xcode" => "Xcode".to_string(),
            "npm" => "npm".to_string(),
            "pnpm" => "pnpm".to_string(),
            "pip" => "pip".to_string(),
            other => {
                let mut chars = other.chars();
                match chars.next() {
                    Some(first) => first.to_uppercase().chain(chars).collect(),
                    None => String::new(),
                }
            }
        })
        .collect::<Vec<_>>()
        .join(" ")
}

fn format_bytes(bytes: u64) -> String {
    const UNITS: [&str; 5] = ["B", "KB", "MB", "GB", "TB"];
    let mut value = bytes as f64;
    let mut unit = 0;
    while value >= 1024.0 && unit < UNITS.len() - 1 {
        value /= 1024.0;
        unit += 1;
    }
    if unit == 0 {
        format!("{bytes} B")
    } else if value >= 10.0 {
        format!("{value:.1} {}", UNITS[unit])
    } else {
        format!("{value:.2} {}", UNITS[unit])
    }
}

fn format_duration(elapsed_ms: u64) -> String {
    if elapsed_ms < 1_000 {
        format!("{elapsed_ms} ms")
    } else {
        format!("{:.1} s", elapsed_ms as f64 / 1_000.0)
    }
}

fn style(value: &str, ansi_code: &str, enabled: bool) -> String {
    if enabled {
        format!("\u{1b}[{ansi_code}m{value}\u{1b}[0m")
    } else {
        value.to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rule_identifier_is_readable_without_ui_localization() {
        assert_eq!(
            humanize_rule_id("project.rust-build-artifacts"),
            "Rust Build Artifacts"
        );
        assert_eq!(
            humanize_rule_id("ai.xcode-model-cache"),
            "Xcode Model Cache"
        );
    }

    #[test]
    fn byte_format_uses_compact_binary_units() {
        assert_eq!(format_bytes(512), "512 B");
        assert_eq!(format_bytes(1_048_576), "1.00 MB");
        assert_eq!(format_bytes(12 * 1_073_741_824), "12.0 GB");
    }
}
