use std::sync::{
    atomic::{AtomicBool, AtomicU64, AtomicU8, Ordering},
    Arc, Mutex, OnceLock,
};
use std::{
    fs::{self, File, OpenOptions},
    io::ErrorKind,
    time::Instant,
};

use crate::shared::application_paths;
use fs2::FileExt;

use super::{CoreError, CoreResult};

static COORDINATOR: OnceLock<OperationCoordinator> = OnceLock::new();
#[cfg(test)]
static TEST_OPERATION_LOCK: OnceLock<Mutex<()>> = OnceLock::new();
const OPERATION_RUNNING: u8 = 0;
const OPERATION_COMPLETED: u8 = 1;
pub(crate) const OPERATION_CANCELLED_ERROR: &str = "operation cancelled";

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum CoordinatedOperationKind {
    CleanupScan,
    Analysis,
    LargeFiles,
    DuplicateFiles,
    ApplicationScan,
    Applications,
    ApplicationLeftoverCleanup,
    ApplicationClose,
    Cleanup,
    PermanentDelete,
    StartupScan,
    StartupChange,
    SystemSettingsScan,
    SystemSettingsChange,
    SystemMaintenanceScan,
    SystemMaintenanceExecution,
}

impl CoordinatedOperationKind {
    pub(crate) fn as_str(self) -> &'static str {
        match self {
            Self::CleanupScan => "cleanup_scan",
            Self::Analysis => "analysis",
            Self::LargeFiles => "large_files",
            Self::DuplicateFiles => "duplicate_files",
            Self::ApplicationScan => "application_scan",
            Self::Applications => "applications",
            Self::ApplicationLeftoverCleanup => "application_leftover_cleanup",
            Self::ApplicationClose => "application_close",
            Self::Cleanup => "cleanup",
            Self::PermanentDelete => "permanent_delete",
            Self::StartupScan => "startup_scan",
            Self::StartupChange => "startup_change",
            Self::SystemSettingsScan => "system_settings_scan",
            Self::SystemSettingsChange => "system_settings_change",
            Self::SystemMaintenanceScan => "system_maintenance_scan",
            Self::SystemMaintenanceExecution => "system_maintenance_execution",
        }
    }

    /// Classifies operations that can change files, application state, or system configuration.
    /// Read-only scans may overlap the dedicated maintenance session, while every mutation remains
    /// mutually exclusive across both the in-process coordinator and the process lock boundary.
    const fn is_mutating(self) -> bool {
        matches!(
            self,
            Self::Applications
                | Self::ApplicationLeftoverCleanup
                | Self::ApplicationClose
                | Self::Cleanup
                | Self::PermanentDelete
                | Self::StartupChange
                | Self::SystemSettingsChange
                | Self::SystemMaintenanceExecution
        )
    }
}

/// Adapter-owned handle for cancelling one class of active Core operation.
///
/// The token contains no UI or terminal state. Desktop and CLI adapters can
/// therefore request cancellation through the same Core contract.
#[derive(Clone, Copy, Debug)]
pub struct OperationCancellationToken {
    kind: CoordinatedOperationKind,
}

impl OperationCancellationToken {
    pub const fn cleanup_scan() -> Self {
        Self {
            kind: CoordinatedOperationKind::CleanupScan,
        }
    }

    pub const fn analysis() -> Self {
        Self {
            kind: CoordinatedOperationKind::Analysis,
        }
    }

    pub const fn large_files() -> Self {
        Self {
            kind: CoordinatedOperationKind::LargeFiles,
        }
    }

    pub const fn duplicate_files() -> Self {
        Self {
            kind: CoordinatedOperationKind::DuplicateFiles,
        }
    }

    pub const fn cleanup() -> Self {
        Self {
            kind: CoordinatedOperationKind::Cleanup,
        }
    }

    pub const fn applications() -> Self {
        Self {
            kind: CoordinatedOperationKind::Applications,
        }
    }

    pub const fn application_leftover_cleanup() -> Self {
        Self {
            kind: CoordinatedOperationKind::ApplicationLeftoverCleanup,
        }
    }

    pub const fn application_scan() -> Self {
        Self {
            kind: CoordinatedOperationKind::ApplicationScan,
        }
    }

    pub const fn startup_scan() -> Self {
        Self {
            kind: CoordinatedOperationKind::StartupScan,
        }
    }

    pub const fn startup_change() -> Self {
        Self {
            kind: CoordinatedOperationKind::StartupChange,
        }
    }

    pub const fn system_settings_scan() -> Self {
        Self {
            kind: CoordinatedOperationKind::SystemSettingsScan,
        }
    }

    pub const fn system_maintenance_scan() -> Self {
        Self {
            kind: CoordinatedOperationKind::SystemMaintenanceScan,
        }
    }

    pub fn cancel(self) {
        OperationGuard::cancel(self.kind);
    }
}

struct ActiveOperation {
    id: u64,
    kind: CoordinatedOperationKind,
    cancelled: Arc<AtomicBool>,
}

struct OperationCoordinator {
    next_id: AtomicU64,
    active: Mutex<ActiveOperations>,
}

#[derive(Default)]
struct ActiveOperations {
    foreground: Option<ActiveOperation>,
    maintenance: Option<ActiveOperation>,
}

#[derive(Clone, Copy)]
enum ProcessLockMode {
    Shared,
    Exclusive,
}

struct ProcessOperationLock {
    file: File,
}

impl ProcessOperationLock {
    fn acquire(kind: CoordinatedOperationKind) -> CoreResult<Self> {
        Self::acquire_named("operation.lock", kind, ProcessLockMode::Exclusive)
    }

    fn acquire_named(
        file_name: &str,
        kind: CoordinatedOperationKind,
        mode: ProcessLockMode,
    ) -> CoreResult<Self> {
        let directory = application_paths()?.runtime_directory();
        fs::create_dir_all(directory).map_err(|error| {
            CoreError::operation_failed(format!(
                "failed to create the operation lock directory: {error}"
            ))
        })?;
        let file = OpenOptions::new()
            .create(true)
            .truncate(false)
            .read(true)
            .write(true)
            .open(directory.join(file_name))
            .map_err(|error| {
                CoreError::operation_failed(format!("failed to open the operation lock: {error}"))
            })?;
        let result = match mode {
            ProcessLockMode::Shared => FileExt::try_lock_shared(&file),
            ProcessLockMode::Exclusive => FileExt::try_lock_exclusive(&file),
        };
        match result {
            Ok(()) => Ok(Self { file }),
            Err(error) if is_lock_contention(&error) => Err(CoreError::operation_busy(format!(
                "another MangoDisk operation is already running; requested={} lock={file_name}",
                kind.as_str(),
            ))),
            Err(error) => Err(CoreError::operation_failed(format!(
                "failed to acquire the operation lock: {error}"
            ))),
        }
    }
}

fn is_lock_contention(error: &std::io::Error) -> bool {
    if error.kind() == ErrorKind::WouldBlock {
        return true;
    }

    // LockFileEx reports sharing and lock violations as native Windows errors
    // instead of ErrorKind::WouldBlock on some toolchain versions. Mapping only
    // those two codes preserves stable busy semantics without hiding genuine
    // permission or filesystem failures.
    #[cfg(windows)]
    {
        matches!(error.raw_os_error(), Some(32 | 33))
    }

    #[cfg(not(windows))]
    {
        false
    }
}

impl Drop for ProcessOperationLock {
    fn drop(&mut self) {
        if let Err(error) = FileExt::unlock(&self.file) {
            log::warn!("operation_process_lock_release_failed error={error}");
        }
    }
}

impl OperationCoordinator {
    fn global() -> &'static Self {
        COORDINATOR.get_or_init(|| Self {
            next_id: AtomicU64::new(1),
            active: Mutex::new(ActiveOperations::default()),
        })
    }
}

/// Coordinates disk-intensive and mutating operations across threads and
/// processes. This prevents concurrent adapters from invalidating cache or
/// cleanup state while another operation is still using it.
pub(crate) struct OperationGuard {
    id: u64,
    kind: CoordinatedOperationKind,
    cancelled: Arc<AtomicBool>,
    started: Instant,
    outcome: AtomicU8,
    _foreground_lock: Option<ProcessOperationLock>,
    _maintenance_lock: Option<ProcessOperationLock>,
    _mutation_lock: Option<ProcessOperationLock>,
}

impl OperationGuard {
    pub(crate) fn start(kind: CoordinatedOperationKind) -> CoreResult<Self> {
        let coordinator = OperationCoordinator::global();
        let mut active = coordinator.active.lock().map_err(|_| {
            CoreError::operation_failed("the operation coordinator is temporarily unavailable")
        })?;
        let conflict = if kind == CoordinatedOperationKind::SystemMaintenanceExecution {
            active.maintenance.as_ref().or_else(|| {
                active
                    .foreground
                    .as_ref()
                    .filter(|operation| operation.kind.is_mutating())
            })
        } else {
            active.foreground.as_ref().or_else(|| {
                kind.is_mutating()
                    .then(|| active.maintenance.as_ref())
                    .flatten()
            })
        };
        if let Some(operation) = conflict {
            return Err(CoreError::operation_busy(format!(
                "another MangoDisk operation is already running: {} ({})",
                operation.kind.as_str(),
                operation.id
            )));
        }

        // Foreground operations retain their historical single-operation lock. System maintenance
        // uses a separate exclusive session lock and a shared mutation lock, allowing read-only
        // scans to remain responsive while still excluding cleanup, deletion, and setting changes
        // in this process and in another MangoDisk adapter process.
        let foreground_lock = (kind != CoordinatedOperationKind::SystemMaintenanceExecution)
            .then(|| ProcessOperationLock::acquire(kind))
            .transpose()?;
        let maintenance_lock = (kind == CoordinatedOperationKind::SystemMaintenanceExecution)
            .then(|| {
                ProcessOperationLock::acquire_named(
                    "system-maintenance.lock",
                    kind,
                    ProcessLockMode::Exclusive,
                )
            })
            .transpose()?;
        let mutation_lock = if kind == CoordinatedOperationKind::SystemMaintenanceExecution {
            Some(ProcessOperationLock::acquire_named(
                "mutation.lock",
                kind,
                ProcessLockMode::Shared,
            )?)
        } else if kind.is_mutating() {
            Some(ProcessOperationLock::acquire_named(
                "mutation.lock",
                kind,
                ProcessLockMode::Exclusive,
            )?)
        } else {
            None
        };
        let id = coordinator.next_id.fetch_add(1, Ordering::Relaxed);
        let cancelled = Arc::new(AtomicBool::new(false));
        let operation = ActiveOperation {
            id,
            kind,
            cancelled: Arc::clone(&cancelled),
        };
        if kind == CoordinatedOperationKind::SystemMaintenanceExecution {
            active.maintenance = Some(operation);
        } else {
            active.foreground = Some(operation);
        }
        log::info!(
            "operation_started operation_id={} operation_kind={}",
            id,
            kind.as_str()
        );
        Ok(Self {
            id,
            kind,
            cancelled,
            started: Instant::now(),
            outcome: AtomicU8::new(OPERATION_RUNNING),
            _foreground_lock: foreground_lock,
            _maintenance_lock: maintenance_lock,
            _mutation_lock: mutation_lock,
        })
    }

    pub(crate) fn id(&self) -> u64 {
        self.id
    }

    pub(crate) fn cancelled(&self) -> &AtomicBool {
        self.cancelled.as_ref()
    }

    /// Background validation receives only the flag so a worker cannot extend
    /// guard ownership and keep the process lock alive accidentally.
    pub(crate) fn cancellation_flag(&self) -> Arc<AtomicBool> {
        Arc::clone(&self.cancelled)
    }

    pub(crate) fn ensure_not_cancelled(&self) -> CoreResult<()> {
        if self.cancelled.load(Ordering::Relaxed) {
            Err(CoreError::operation_cancelled())
        } else {
            Ok(())
        }
    }

    pub(crate) fn complete(&self) {
        self.outcome.store(OPERATION_COMPLETED, Ordering::Relaxed);
    }

    pub(crate) fn cancel(kind: CoordinatedOperationKind) {
        let Ok(active) = OperationCoordinator::global().active.lock() else {
            log::warn!(
                "operation_cancel_failed operation_kind={} reason=coordinator_poisoned",
                kind.as_str()
            );
            return;
        };
        let operation = if kind == CoordinatedOperationKind::SystemMaintenanceExecution {
            active.maintenance.as_ref()
        } else {
            active.foreground.as_ref()
        };
        let Some(operation) = operation.filter(|operation| operation.kind == kind) else {
            return;
        };
        operation.cancelled.store(true, Ordering::Relaxed);
        log::info!(
            "operation_cancel_requested operation_id={} operation_kind={}",
            operation.id,
            operation.kind.as_str()
        );
    }
}

impl Drop for OperationGuard {
    fn drop(&mut self) {
        let Ok(mut active) = OperationCoordinator::global().active.lock() else {
            log::warn!(
                "operation_release_failed operation_id={} operation_kind={} reason=coordinator_poisoned",
                self.id,
                self.kind.as_str()
            );
            return;
        };
        let slot = if self.kind == CoordinatedOperationKind::SystemMaintenanceExecution {
            &mut active.maintenance
        } else {
            &mut active.foreground
        };
        if slot
            .as_ref()
            .is_some_and(|operation| operation.id == self.id)
        {
            *slot = None;
        }
        let cancelled = self.cancelled.load(Ordering::Relaxed);
        let status = if cancelled {
            "cancelled"
        } else {
            match self.outcome.load(Ordering::Relaxed) {
                OPERATION_COMPLETED => "completed",
                _ => "failed",
            }
        };
        if status == "failed" {
            log::warn!(
                "operation_finished operation_id={} operation_kind={} status={} cancelled={} elapsed_ms={}",
                self.id,
                self.kind.as_str(),
                status,
                cancelled,
                self.started.elapsed().as_millis()
            );
        } else {
            log::info!(
                "operation_finished operation_id={} operation_kind={} status={} cancelled={} elapsed_ms={}",
                self.id,
                self.kind.as_str(),
                status,
                cancelled,
                self.started.elapsed().as_millis()
            );
        }
    }
}

#[cfg(test)]
pub(crate) fn test_operation_lock() -> std::sync::MutexGuard<'static, ()> {
    TEST_OPERATION_LOCK
        .get_or_init(|| Mutex::new(()))
        .lock()
        // The test-only mutex carries no business state. Recovering a poisoned
        // guard prevents one assertion failure from hiding later independent
        // disk test results.
        .unwrap_or_else(std::sync::PoisonError::into_inner)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::CoreErrorCode;

    #[test]
    fn cancellation_token_stops_the_matching_operation() {
        let _test_guard = test_operation_lock();
        let operation = OperationGuard::start(CoordinatedOperationKind::CleanupScan)
            .expect("the isolated operation should start");

        OperationCancellationToken::cleanup_scan().cancel();

        assert_eq!(
            operation
                .ensure_not_cancelled()
                .expect_err("the operation should be cancelled")
                .code(),
            CoreErrorCode::OperationCancelled
        );
    }

    #[test]
    fn application_scan_cancellation_does_not_cancel_application_execution() {
        let _test_guard = test_operation_lock();
        let operation = OperationGuard::start(CoordinatedOperationKind::Applications)
            .expect("the isolated application operation should start");

        OperationCancellationToken::application_scan().cancel();

        operation
            .ensure_not_cancelled()
            .expect("scan cancellation must not affect an application mutation");
    }

    #[test]
    fn process_lock_is_released_when_the_guard_drops() {
        let _test_guard = test_operation_lock();
        let first = ProcessOperationLock::acquire(CoordinatedOperationKind::Analysis)
            .expect("the first process lock should succeed");
        let error = ProcessOperationLock::acquire(CoordinatedOperationKind::Analysis)
            .err()
            .expect("a second process lock should be rejected");
        assert_eq!(error.code(), CoreErrorCode::OperationBusy);

        drop(first);
        let second = ProcessOperationLock::acquire(CoordinatedOperationKind::Analysis)
            .expect("the process lock should be reusable after release");
        drop(second);
    }

    #[test]
    fn mutation_lock_allows_maintenance_readers_and_rejects_a_writer() {
        let _test_guard = test_operation_lock();
        let first = ProcessOperationLock::acquire_named(
            "mutation.lock",
            CoordinatedOperationKind::SystemMaintenanceExecution,
            ProcessLockMode::Shared,
        )
        .expect("the first maintenance reader should acquire the mutation lock");
        let second = ProcessOperationLock::acquire_named(
            "mutation.lock",
            CoordinatedOperationKind::SystemMaintenanceExecution,
            ProcessLockMode::Shared,
        )
        .expect("a second maintenance reader should share the mutation lock");
        let error = ProcessOperationLock::acquire_named(
            "mutation.lock",
            CoordinatedOperationKind::Cleanup,
            ProcessLockMode::Exclusive,
        )
        .err()
        .expect("a mutation writer must be rejected while maintenance is active");
        assert_eq!(error.code(), CoreErrorCode::OperationBusy);

        drop(second);
        drop(first);
        let writer = ProcessOperationLock::acquire_named(
            "mutation.lock",
            CoordinatedOperationKind::Cleanup,
            ProcessLockMode::Exclusive,
        )
        .expect("the mutation writer should start after maintenance readers finish");
        drop(writer);
    }

    #[test]
    fn maintenance_execution_allows_a_read_only_foreground_scan() {
        let _test_guard = test_operation_lock();
        let maintenance =
            OperationGuard::start(CoordinatedOperationKind::SystemMaintenanceExecution)
                .expect("maintenance should start in isolation");
        let scan = OperationGuard::start(CoordinatedOperationKind::SystemSettingsScan)
            .expect("a read-only settings scan should remain available during maintenance");

        drop(scan);
        drop(maintenance);
    }

    #[test]
    fn maintenance_execution_blocks_foreground_mutations() {
        let _test_guard = test_operation_lock();
        let maintenance =
            OperationGuard::start(CoordinatedOperationKind::SystemMaintenanceExecution)
                .expect("maintenance should start in isolation");
        let error = OperationGuard::start(CoordinatedOperationKind::SystemSettingsChange)
            .err()
            .expect("a settings mutation must not overlap system maintenance");

        assert_eq!(error.code(), CoreErrorCode::OperationBusy);
        drop(maintenance);
    }

    #[test]
    fn foreground_mutations_block_maintenance_execution() {
        let _test_guard = test_operation_lock();
        let mutation = OperationGuard::start(CoordinatedOperationKind::Cleanup)
            .expect("cleanup should start in isolation");
        let error = OperationGuard::start(CoordinatedOperationKind::SystemMaintenanceExecution)
            .err()
            .expect("system maintenance must not overlap cleanup mutation");

        assert_eq!(error.code(), CoreErrorCode::OperationBusy);
        drop(mutation);
    }
}
