use mangodisk_core::{
    AnalysisService, DuplicateFileService, LargeFileService, OperationCancellationToken,
};

use crate::{
    arguments::StorageSubcommand,
    commands::{CliFailure, CommandContext},
    output::CommandOutcome,
    progress::CliProgressSink,
};

pub fn run(
    command: StorageSubcommand,
    context: &CommandContext<'_>,
) -> Result<CommandOutcome, CliFailure> {
    match command {
        StorageSubcommand::Analyze(arguments) => {
            let _active = context
                .cancellation
                .activate(OperationCancellationToken::analysis());
            let result = AnalysisService::analyze_with_progress(
                Some(arguments.root.to_string_lossy().into_owned()),
                arguments.refresh,
                progress_sink(context),
            )?;
            CommandOutcome::success(
                "storage.analyze",
                format!(
                    "Analyzed {} bytes across {} top-level entries.",
                    result.total_bytes,
                    result.entries.len()
                ),
                result,
            )
            .map_err(Into::into)
        }
        StorageSubcommand::LargeFiles(arguments) => {
            let _active = context
                .cancellation
                .activate(OperationCancellationToken::large_files());
            let result = LargeFileService::find_with_progress(
                Some(arguments.root.to_string_lossy().into_owned()),
                arguments.minimum_bytes,
                arguments.refresh,
                progress_sink(context),
            )?;
            CommandOutcome::success(
                "storage.large-files",
                format!(
                    "Found {} large files using {} bytes.",
                    result.total_count, result.total_bytes
                ),
                result,
            )
            .map_err(Into::into)
        }
        StorageSubcommand::DuplicateFiles(arguments) => {
            let _active = context
                .cancellation
                .activate(OperationCancellationToken::duplicate_files());
            let roots = arguments
                .root
                .into_iter()
                .map(|root| root.to_string_lossy().into_owned())
                .collect::<Vec<_>>();
            let result = DuplicateFileService::find_with_progress(
                roots,
                arguments.minimum_bytes,
                progress_sink(context),
            )?;
            CommandOutcome::success(
                "storage.duplicate-files",
                format!(
                    "Found {} duplicate groups with {} reclaimable bytes.",
                    result.total_group_count, result.reclaimable_bytes
                ),
                result,
            )
            .map_err(Into::into)
        }
    }
}

fn progress_sink(context: &CommandContext<'_>) -> CliProgressSink {
    CliProgressSink::new(
        context.format,
        context.progress_enabled,
        context.include_full_paths,
    )
}
