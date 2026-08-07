use std::{
    io::{self, IsTerminal, Write},
    path::Path,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc, Mutex,
    },
};

use mangodisk_core::{
    diagnostic_path, OperationCancellationToken, ProgressSink, TraversalProgress,
};
use serde_json::json;

use crate::{arguments::OutputFormat, output::OUTPUT_SCHEMA_VERSION};

#[derive(Clone, Default)]
pub struct CancellationController {
    active: Arc<Mutex<Option<OperationCancellationToken>>>,
    cancelled: Arc<AtomicBool>,
}

impl CancellationController {
    pub fn install() -> Result<Self, String> {
        let controller = Self::default();
        let signal_controller = controller.clone();
        ctrlc::set_handler(move || signal_controller.cancel())
            .map_err(|error| format!("failed to install the cancellation handler: {error}"))?;
        Ok(controller)
    }

    pub fn activate(&self, token: OperationCancellationToken) -> ActiveCancellation<'_> {
        if let Ok(mut active) = self.active.lock() {
            *active = Some(token);
        }
        // Ctrl+C can arrive after argument parsing but before Core creates its
        // operation guard. Preserve that intent and cancel the newly activated
        // operation instead of resetting the signal during adapter hand-off.
        if self.cancelled.load(Ordering::Relaxed) {
            token.cancel();
        }
        ActiveCancellation { controller: self }
    }

    pub fn was_cancelled(&self) -> bool {
        self.cancelled.load(Ordering::Relaxed)
    }

    fn cancel(&self) {
        self.cancelled.store(true, Ordering::Relaxed);
        if let Ok(active) = self.active.lock() {
            if let Some(token) = *active {
                token.cancel();
            }
        }
    }
}

pub struct ActiveCancellation<'a> {
    controller: &'a CancellationController,
}

impl Drop for ActiveCancellation<'_> {
    fn drop(&mut self) {
        if let Ok(mut active) = self.controller.active.lock() {
            *active = None;
        }
    }
}

pub struct CliProgressSink {
    format: OutputFormat,
    enabled: bool,
    include_full_paths: bool,
    interactive: bool,
    rendered_line: Mutex<bool>,
}

impl CliProgressSink {
    pub fn new(format: OutputFormat, enabled: bool, include_full_paths: bool) -> Self {
        Self {
            format,
            enabled,
            include_full_paths,
            interactive: format == OutputFormat::Human && io::stderr().is_terminal(),
            rendered_line: Mutex::new(false),
        }
    }
}

impl ProgressSink for CliProgressSink {
    fn report(&self, progress: TraversalProgress) {
        if !self.enabled {
            return;
        }
        let path = if self.include_full_paths {
            progress.current_path.clone()
        } else {
            diagnostic_path(Path::new(&progress.current_path))
        };
        match self.format {
            OutputFormat::Human if self.interactive => {
                eprint!(
                    "\r\x1b[2KScanning {} items ({}) · {}",
                    progress.items_scanned,
                    format_bytes(progress.bytes_scanned),
                    path
                );
                let _ = io::stderr().flush();
                if let Ok(mut rendered) = self.rendered_line.lock() {
                    *rendered = true;
                }
            }
            OutputFormat::Human => eprintln!(
                "Scanning {} items ({}), found {} · {}",
                progress.items_scanned,
                format_bytes(progress.bytes_scanned),
                progress.found_items,
                path
            ),
            OutputFormat::Json | OutputFormat::Jsonl => eprintln!(
                "{}",
                json!({
                    "schemaVersion": OUTPUT_SCHEMA_VERSION,
                    "type": "progress",
                    "data": {
                        "operationId": progress.operation_id,
                        "currentPath": path,
                        "itemsScanned": progress.items_scanned,
                        "bytesScanned": progress.bytes_scanned,
                        "completedSteps": progress.completed_steps,
                        "totalSteps": progress.total_steps,
                        "foundItems": progress.found_items,
                        "foundBytes": progress.found_bytes,
                        "elapsedMs": progress.elapsed_ms,
                    }
                })
            ),
        }
    }
}

impl Drop for CliProgressSink {
    fn drop(&mut self) {
        if self.interactive && self.rendered_line.get_mut().is_ok_and(|rendered| *rendered) {
            eprint!("\r\x1b[2K");
            let _ = io::stderr().flush();
        }
    }
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
    } else {
        format!("{value:.1} {}", UNITS[unit])
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn activation_preserves_a_signal_received_before_core_starts() {
        let controller = CancellationController::default();
        controller.cancel();

        let _active = controller.activate(OperationCancellationToken::cleanup_scan());

        assert!(controller.was_cancelled());
    }

    #[test]
    fn progress_bytes_use_compact_units() {
        assert_eq!(format_bytes(512), "512 B");
        assert_eq!(format_bytes(1_572_864), "1.5 MB");
    }
}
