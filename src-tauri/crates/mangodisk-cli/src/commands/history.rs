use mangodisk_core::HistoryService;

use crate::{arguments::HistorySubcommand, commands::CliFailure, output::CommandOutcome};

pub fn run(command: HistorySubcommand) -> Result<CommandOutcome, CliFailure> {
    match command {
        HistorySubcommand::List => {
            let records = HistoryService::list()?;
            CommandOutcome::success(
                "history.list",
                format!("Found {} cleanup history records.", records.len()),
                records,
            )
            .map_err(Into::into)
        }
    }
}
