use mangodisk_core::{
    ApplicationLeftoverService, ApplicationUninstallService, CoreError, OperationCancellationToken,
};

use crate::{
    arguments::{
        ApplicationLeftoversSubcommand, ApplicationUninstallSubcommand, ApplicationsSubcommand,
    },
    commands::{CliFailure, CommandContext},
    exit_code::CliExitCode,
    output::CommandOutcome,
};

pub fn run(
    command: ApplicationsSubcommand,
    context: &CommandContext<'_>,
) -> Result<CommandOutcome, CliFailure> {
    match command {
        ApplicationsSubcommand::Uninstall(command) => run_uninstall(command.command, context),
        ApplicationsSubcommand::Leftovers(command) => run_leftovers(command.command),
    }
}

fn run_uninstall(
    command: ApplicationUninstallSubcommand,
    context: &CommandContext<'_>,
) -> Result<CommandOutcome, CliFailure> {
    let cancellation_token = match &command {
        ApplicationUninstallSubcommand::Scan => OperationCancellationToken::application_scan(),
        ApplicationUninstallSubcommand::Inspect(_) => OperationCancellationToken::applications(),
    };
    let _active = context.cancellation.activate(cancellation_token);
    if context.cancellation.was_cancelled() {
        return Err(CoreError::operation_cancelled().into());
    }
    match command {
        ApplicationUninstallSubcommand::Scan => {
            let scan = ApplicationUninstallService::scan()?;
            CommandOutcome::success(
                "applications.uninstall.scan",
                format!(
                    "Found {} visible applications; {} can be handled without elevation and {} were hidden by safety policy.",
                    scan.candidates.len(),
                    scan.ready_count,
                    scan.hidden_count
                ),
                scan,
            )
            .map_err(Into::into)
        }
        ApplicationUninstallSubcommand::Inspect(arguments) => {
            let inspection = ApplicationUninstallService::inspect(&arguments.application_id)?;
            CommandOutcome::success(
                "applications.uninstall.inspect",
                format!(
                    "Measured {} components with {} total bytes for {}.",
                    inspection.components.len(),
                    inspection.total_bytes,
                    inspection.application_name
                ),
                inspection,
            )
            .map_err(Into::into)
        }
    }
}

fn run_leftovers(command: ApplicationLeftoversSubcommand) -> Result<CommandOutcome, CliFailure> {
    match command {
        ApplicationLeftoversSubcommand::Scan => {
            let scan = ApplicationLeftoverService::scan()?;
            let access_limited = scan.access_limited;
            let human = if access_limited {
                "Application leftover scan was incomplete because macOS denied access to application containers."
                    .to_string()
            } else {
                format!(
                    "Found {} high-confidence application leftovers with {} bytes.",
                    scan.candidates.len(),
                    scan.total_bytes
                )
            };
            let mut outcome = CommandOutcome::success("applications.leftovers.scan", human, scan)
                .map_err(CliFailure::from)?;
            if access_limited {
                outcome.exit_code = CliExitCode::CompletedWithWarnings;
            }
            Ok(outcome)
        }
    }
}
