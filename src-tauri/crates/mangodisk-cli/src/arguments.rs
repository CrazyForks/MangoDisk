use std::path::PathBuf;

use clap::{Args, Parser, Subcommand, ValueEnum};

#[derive(Debug, Parser)]
#[command(
    name = "mangodisk",
    version,
    about = "Safely inspect and reclaim disk space",
    long_about = "MangoDisk scans cleanup candidates without changing files by default.\n\
Use `mangodisk clean --apply` to review and apply the same recommendations as the desktop app.",
    after_help = "Examples:\n  \
  mangodisk clean\n  \
  mangodisk clean --apply\n  \
  mangodisk clean --apply --selection all\n  \
  mangodisk clean --apply --dry-run\n  \
  mangodisk clean --format json --no-progress"
)]
pub struct Cli {
    /// Select the output format. JSON formats never contain terminal colors.
    #[arg(long, global = true, value_enum, default_value_t = OutputFormat::Human)]
    pub format: OutputFormat,

    /// Control colored human-readable output.
    #[arg(long, global = true, value_enum, default_value_t = ColorMode::Auto)]
    pub color: ColorMode,

    /// Include complete local paths in machine-readable output and progress.
    #[arg(long, global = true)]
    pub include_full_paths: bool,

    /// Suppress progress written to stderr.
    #[arg(long, global = true)]
    pub no_progress: bool,

    #[command(subcommand)]
    pub command: Command,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
pub enum OutputFormat {
    Human,
    Json,
    Jsonl,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
pub enum ColorMode {
    Auto,
    Always,
    Never,
}

#[derive(Debug, Subcommand)]
pub enum Command {
    /// Scan cleanup candidates and optionally apply a reviewed selection.
    Clean(CleanArgs),
    /// Inspect installed applications or their leftovers without changing the system.
    Applications(ApplicationsCommand),
    /// Analyze storage, large files, or exact duplicates.
    Storage(StorageCommand),
    /// Inspect persisted operation history.
    History(HistoryCommand),
}

#[derive(Debug, Args)]
#[command(after_help = "Examples:\n  \
    mangodisk clean\n  \
    mangodisk clean --apply\n  \
    mangodisk clean --apply --selection all\n  \
    mangodisk clean --apply --dry-run --yes")]
pub struct CleanArgs {
    /// Apply the selected cleanup rules after the read-only scan.
    #[arg(long)]
    pub apply: bool,

    /// Choose which cleanup candidates to apply.
    ///
    /// `recommended` matches the desktop app's smart selection. `all` includes
    /// every currently selectable candidate and requires additional review.
    #[arg(long, value_enum, requires = "apply")]
    pub selection: Option<CleanSelection>,

    /// Run preflight and measurement without changing files.
    #[arg(long, requires = "apply")]
    pub dry_run: bool,

    /// Skip the interactive confirmation.
    ///
    /// Required with `--apply` when stdin is not an interactive terminal.
    #[arg(long, requires = "apply")]
    pub yes: bool,

    /// Search local volumes for additional development projects.
    #[arg(long)]
    pub deep_project_discovery: bool,

    /// Include source counts and paths in human-readable scan output.
    #[arg(long)]
    pub details: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
pub enum CleanSelection {
    Recommended,
    All,
}

#[derive(Debug, Args)]
pub struct ApplicationsCommand {
    #[command(subcommand)]
    pub command: ApplicationsSubcommand,
}

#[derive(Debug, Subcommand)]
pub enum ApplicationsSubcommand {
    /// Inspect installed applications and their uninstall capabilities.
    Uninstall(ApplicationUninstallCommand),
    /// Inspect high-confidence data left by uninstalled applications.
    Leftovers(ApplicationLeftoversCommand),
}

#[derive(Debug, Args)]
pub struct ApplicationUninstallCommand {
    #[command(subcommand)]
    pub command: ApplicationUninstallSubcommand,
}

#[derive(Debug, Subcommand)]
pub enum ApplicationUninstallSubcommand {
    /// List installed applications without changing the system.
    Scan,
    /// Measure one application and exact-identifier associations without changing files.
    Inspect(ApplicationUninstallInspectArgs),
}

#[derive(Debug, Args)]
pub struct ApplicationUninstallInspectArgs {
    /// Stable application identifier returned by `applications uninstall scan`.
    #[arg(long)]
    pub application_id: String,
}

#[derive(Debug, Args)]
pub struct ApplicationLeftoversCommand {
    #[command(subcommand)]
    pub command: ApplicationLeftoversSubcommand,
}

#[derive(Debug, Subcommand)]
pub enum ApplicationLeftoversSubcommand {
    /// Scan application leftovers without changing files.
    Scan,
}

#[derive(Debug, Args)]
pub struct StorageCommand {
    #[command(subcommand)]
    pub command: StorageSubcommand,
}

#[derive(Debug, Subcommand)]
pub enum StorageSubcommand {
    /// Analyze one disk or folder.
    Analyze(StorageAnalyzeArgs),
    /// Find large files under one disk or folder.
    LargeFiles(StorageLargeFilesArgs),
    /// Find byte-identical duplicate files under one or more folders.
    DuplicateFiles(StorageDuplicateFilesArgs),
}

#[derive(Debug, Args)]
pub struct StorageAnalyzeArgs {
    #[arg(long)]
    pub root: PathBuf,

    /// Ignore a compatible persisted index and scan again.
    #[arg(long)]
    pub refresh: bool,
}

#[derive(Debug, Args)]
pub struct StorageLargeFilesArgs {
    #[arg(long)]
    pub root: PathBuf,

    #[arg(long, default_value_t = 50 * 1024 * 1024)]
    pub minimum_bytes: u64,

    /// Ignore a compatible persisted index and scan again.
    #[arg(long)]
    pub refresh: bool,
}

#[derive(Debug, Args)]
pub struct StorageDuplicateFilesArgs {
    #[arg(long, required = true)]
    pub root: Vec<PathBuf>,

    #[arg(long, default_value_t = 1024 * 1024)]
    pub minimum_bytes: u64,
}

#[derive(Debug, Args)]
pub struct HistoryCommand {
    #[command(subcommand)]
    pub command: HistorySubcommand,
}

#[derive(Debug, Subcommand)]
pub enum HistorySubcommand {
    /// List cleanup operation records.
    List,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn clean_defaults_to_a_read_only_recommended_scan() {
        let parsed =
            Cli::try_parse_from(["mangodisk", "clean"]).expect("the documented command must parse");

        let Command::Clean(arguments) = parsed.command else {
            panic!("clean command expected");
        };
        assert!(!arguments.apply);
        assert_eq!(arguments.selection, None);
        assert!(!arguments.dry_run);
        assert!(!arguments.yes);
    }

    #[test]
    fn clean_accepts_an_explicit_all_selection_dry_run() {
        let parsed = Cli::try_parse_from([
            "mangodisk",
            "--format",
            "json",
            "clean",
            "--apply",
            "--selection",
            "all",
            "--dry-run",
            "--yes",
        ])
        .expect("the documented command must parse");

        assert_eq!(parsed.format, OutputFormat::Json);
        let Command::Clean(arguments) = parsed.command else {
            panic!("clean command expected");
        };
        assert!(arguments.apply);
        assert_eq!(arguments.selection, Some(CleanSelection::All));
        assert!(arguments.dry_run);
        assert!(arguments.yes);
    }

    #[test]
    fn clean_rejects_a_selection_without_apply() {
        let error = Cli::try_parse_from(["mangodisk", "clean", "--selection", "all"])
            .expect_err("selection must require --apply");

        assert!(error.to_string().contains("--apply"));
    }

    #[test]
    fn application_leftover_scan_uses_the_applications_domain() {
        let parsed = Cli::try_parse_from(["mangodisk", "applications", "leftovers", "scan"])
            .expect("the documented application leftover scan must parse");

        let Command::Applications(command) = parsed.command else {
            panic!("applications command expected");
        };
        let ApplicationsSubcommand::Leftovers(command) = command.command else {
            panic!("application leftover command expected");
        };
        assert!(matches!(
            command.command,
            ApplicationLeftoversSubcommand::Scan
        ));
    }

    #[test]
    fn application_uninstall_scan_uses_the_applications_domain() {
        let parsed = Cli::try_parse_from([
            "mangodisk",
            "--format",
            "json",
            "applications",
            "uninstall",
            "scan",
        ])
        .expect("the documented application uninstall scan must parse");

        let Command::Applications(command) = parsed.command else {
            panic!("applications command expected");
        };
        let ApplicationsSubcommand::Uninstall(command) = command.command else {
            panic!("application uninstall command expected");
        };
        assert!(matches!(
            command.command,
            ApplicationUninstallSubcommand::Scan
        ));
    }

    #[test]
    fn application_uninstall_inspect_requires_a_catalog_identifier() {
        let parsed = Cli::try_parse_from([
            "mangodisk",
            "applications",
            "uninstall",
            "inspect",
            "--application-id",
            "application-example",
        ])
        .expect("the documented application uninstall inspect command must parse");

        let Command::Applications(command) = parsed.command else {
            panic!("applications command expected");
        };
        let ApplicationsSubcommand::Uninstall(command) = command.command else {
            panic!("application uninstall command expected");
        };
        let ApplicationUninstallSubcommand::Inspect(arguments) = command.command else {
            panic!("application uninstall inspect command expected");
        };
        assert_eq!(arguments.application_id, "application-example");
    }

    #[test]
    fn application_uninstall_plan_is_not_a_public_command() {
        let result = Cli::try_parse_from([
            "mangodisk",
            "applications",
            "uninstall",
            "plan",
            "--application-id",
            "application-example",
        ]);

        assert!(result.is_err());
    }

    #[test]
    fn application_leftover_execution_is_not_a_public_command() {
        let result = Cli::try_parse_from([
            "mangodisk",
            "applications",
            "leftovers",
            "execute",
            "--plan",
            "plan.json",
        ]);

        assert!(result.is_err());
    }
}
