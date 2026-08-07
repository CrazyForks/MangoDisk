use std::{
    collections::{HashMap, HashSet},
    sync::{Mutex, OnceLock},
    time::Instant,
};

use mangodisk_platform::InstalledApplication;
use mangodisk_platform::{
    current_platform, ControlledExecutable, Platform, PlatformCancellation, SystemInventory,
};

static SYSTEM_INVENTORY: OnceLock<Mutex<Option<CachedSystemInventory>>> = OnceLock::new();

#[derive(Debug, Clone)]
struct CachedSystemInventory {
    revision: String,
    inventory: SystemInventory,
}

#[derive(Debug, Clone)]
pub(crate) struct ApplicationInventory {
    applications: Vec<InstalledApplication>,
    application_versions: HashMap<String, Vec<String>>,
    application_identifiers: HashSet<String>,
    applications_complete: bool,
    executable_names: HashSet<String>,
    executables: HashMap<String, ControlledExecutable>,
    developer_tools_complete: bool,
    filesystem_kinds: HashSet<String>,
    filesystem_complete: bool,
    capabilities: HashSet<String>,
    capabilities_complete: bool,
    os_version: String,
    pub(crate) application_count: usize,
    pub(crate) inventory_complete: bool,
}

#[derive(Debug)]
pub(crate) struct ScanContext {
    pub(crate) inventory: ApplicationInventory,
}

#[derive(Debug, Default)]
pub(crate) struct ProcessSnapshot {
    running_processes: HashSet<String>,
    running_executable_paths: HashSet<String>,
    pub(crate) process_count: usize,
}

impl ProcessSnapshot {
    pub(crate) fn capture() -> Result<Self, String> {
        Self::capture_with_cancellation(&PlatformCancellation::new(|| false))
    }

    pub(crate) fn capture_with_cancellation(
        cancellation: &PlatformCancellation,
    ) -> Result<Self, String> {
        current_platform()
            .running_process_names_with_cancellation(cancellation)
            .map_err(|error| error.to_string())
            .map(Self::from_process_names)
    }

    pub(crate) fn matching_processes(&self, names: &[String]) -> Vec<String> {
        names
            .iter()
            .filter(|name| {
                process_aliases(name)
                    .iter()
                    .any(|alias| self.running_processes.contains(alias))
            })
            .cloned()
            .collect()
    }

    pub(crate) fn contains_any(&self, names: &[String]) -> bool {
        names.iter().any(|name| {
            process_aliases(name)
                .iter()
                .any(|alias| self.running_processes.contains(alias))
        })
    }

    pub(crate) fn matching_application_processes(
        &self,
        identity_names: &[String],
        executable_paths: &[std::path::PathBuf],
    ) -> Vec<String> {
        let mut matches = self.matching_processes(identity_names);
        matches.extend(executable_paths.iter().filter_map(|path| {
            let normalized = normalize(&path.to_string_lossy().replace('\\', "/"));
            self.running_executable_paths
                .contains(&normalized)
                .then(|| {
                    path.file_name()
                        .map(|value| value.to_string_lossy().into_owned())
                        .unwrap_or_else(|| normalized.clone())
                })
        }));
        matches.sort_by_key(|value| value.to_ascii_lowercase());
        matches.dedup_by(|left, right| left.eq_ignore_ascii_case(right));
        matches
    }

    fn from_process_names(processes: Vec<String>) -> Self {
        let process_count = processes.len();
        let running_executable_paths = processes
            .iter()
            .map(|value| normalize(&value.replace('\\', "/")))
            .collect();
        Self {
            running_processes: processes
                .into_iter()
                .flat_map(|value| process_aliases(&value))
                .collect(),
            running_executable_paths,
            process_count,
        }
    }
}

impl ScanContext {
    pub(crate) fn capture() -> Self {
        Self::capture_with_revision().0
    }

    /// Captures one inventory and the revision used to validate or populate
    /// its cache. Callers that need a stable before/after comparison reuse
    /// this revision instead of issuing the same expensive operating-system
    /// query twice before any useful work begins.
    pub(crate) fn capture_with_revision() -> (Self, Option<String>) {
        Self::capture_with_revision_and_cancellation(&PlatformCancellation::new(|| false))
    }

    pub(crate) fn capture_with_revision_and_cancellation(
        cancellation: &PlatformCancellation,
    ) -> (Self, Option<String>) {
        let started = Instant::now();
        let (system, inventory_complete, revision) = cached_system_inventory(cancellation);
        let inventory = ApplicationInventory::from_system(system, inventory_complete);
        log::debug!(
            "application_inventory_ready application_count={} tool_count={} filesystem_count={} capability_count={} system_complete={} elapsed_ms={}",
            inventory.application_count,
            inventory.executable_names.len(),
            inventory.filesystem_kinds.len(),
            inventory.capabilities.len(),
            inventory.inventory_complete,
            started.elapsed().as_millis()
        );
        (Self { inventory }, revision)
    }
}

fn cached_system_inventory(
    cancellation: &PlatformCancellation,
) -> (SystemInventory, bool, Option<String>) {
    // The progress event is emitted before inventory capture so adapters can
    // cancel immediately. Honor that request before even the cheap revision
    // probe; on large application roots the probe itself is observable work.
    if cancellation.is_cancelled() {
        return (SystemInventory::default(), false, None);
    }
    let cache = SYSTEM_INVENTORY.get_or_init(|| Mutex::new(None));
    let revision = current_platform().system_inventory_revision_with_cancellation(cancellation);
    if let (Ok(revision), Ok(guard)) = (&revision, cache.lock()) {
        if let Some(cached) = guard.as_ref().filter(|cached| cached.revision == *revision) {
            return (cached.inventory.clone(), true, Some(revision.clone()));
        }
    }

    match current_platform().system_inventory_with_cancellation(cancellation) {
        Ok(inventory) => {
            let revision = revision.ok();
            if let Some(revision) = &revision {
                if let Ok(mut guard) = cache.lock() {
                    *guard = Some(CachedSystemInventory {
                        revision: revision.clone(),
                        inventory: inventory.clone(),
                    });
                }
            }
            (inventory, true, revision)
        }
        Err(error) => {
            if cancellation.is_cancelled() {
                return (SystemInventory::default(), false, None);
            }
            log::warn!(
                "application_inventory_capture_failed error_digest={}",
                blake3::hash(error.as_bytes()).to_hex()
            );
            // A stale inventory is more useful than an empty one after a
            // revision-probe failure, but only positive matches remain safe.
            if let Ok(guard) = cache.lock() {
                if let Some(cached) = guard.as_ref() {
                    return (cached.inventory.clone(), false, None);
                }
            }
            (SystemInventory::default(), false, None)
        }
    }
}

impl ApplicationInventory {
    fn from_system(system: SystemInventory, inventory_complete: bool) -> Self {
        let application_count = system.installed_applications.len();
        let applications = system.installed_applications;
        let mut application_identifiers = HashSet::new();
        let mut application_versions = HashMap::<String, Vec<String>>::new();
        let mut executable_names = HashSet::new();
        let mut executables = HashMap::new();
        for application in &applications {
            let version = application.version.as_deref();
            for identifier in application
                .identifiers
                .iter()
                .chain(std::iter::once(&application.name))
            {
                let identifier = normalize(identifier);
                application_identifiers.insert(identifier.clone());
                if let Some(version) = version {
                    let versions = application_versions.entry(identifier).or_default();
                    if !versions.iter().any(|existing| existing == version) {
                        versions.push(version.to_string());
                    }
                }
            }
        }
        for tool in system.developer_tools {
            let name = normalize(&tool.name);
            executable_names.insert(name.clone());
            executables.insert(name, tool.executable);
        }
        Self {
            applications,
            application_versions,
            application_identifiers,
            applications_complete: system.installed_applications_complete,
            executable_names,
            executables,
            developer_tools_complete: system.developer_tools_complete,
            filesystem_kinds: system
                .filesystem_kinds
                .into_iter()
                .map(|value| normalize(&value))
                .collect(),
            filesystem_complete: system.filesystem_complete,
            capabilities: system
                .capabilities
                .into_iter()
                .map(|value| normalize(&value))
                .collect(),
            capabilities_complete: system.capabilities_complete,
            os_version: system.os_version,
            application_count,
            inventory_complete,
        }
    }

    /// Specialized operations must execute the absolute path captured by the
    /// inventory. Aliases are compile-time constants and never rule-provided.
    pub(crate) fn executable(&self, aliases: &[&str]) -> Option<ControlledExecutable> {
        aliases
            .iter()
            .find_map(|alias| self.executables.get(&normalize(alias)).cloned())
    }

    pub(crate) fn executable_inventory_complete(&self) -> bool {
        self.inventory_complete && self.developer_tools_complete
    }

    pub(crate) fn application_inventory_complete(&self) -> bool {
        self.inventory_complete && self.applications_complete
    }

    pub(crate) fn installed_applications(&self) -> &[InstalledApplication] {
        &self.applications
    }

    #[cfg(target_os = "macos")]
    pub(crate) fn has_application_identifier(&self, identifier: &str) -> bool {
        self.application_identifiers
            .contains(&normalize(identifier))
    }

    pub(crate) fn applications_complete(&self) -> bool {
        self.inventory_complete && self.applications_complete
    }

    pub(crate) fn has_application(&self, identifiers: &[String]) -> bool {
        identifiers
            .iter()
            .map(|value| normalize(value))
            .any(|value| self.application_identifiers.contains(&value))
    }

    pub(crate) fn application_versions(&self, identifier: &str) -> Option<&[String]> {
        self.application_versions
            .get(&normalize(identifier))
            .map(Vec::as_slice)
    }

    pub(crate) fn developer_tools_complete(&self) -> bool {
        self.developer_tools_complete
    }

    pub(crate) fn has_executable(&self, names: &[String]) -> bool {
        names
            .iter()
            .map(|value| normalize(value))
            .any(|value| self.executable_names.contains(&value))
    }

    pub(crate) fn os_version(&self) -> &str {
        &self.os_version
    }

    pub(crate) fn filesystem_complete(&self) -> bool {
        self.filesystem_complete
    }

    pub(crate) fn has_filesystem_kind(&self, values: &[String]) -> bool {
        values
            .iter()
            .map(|value| normalize(value))
            .any(|value| self.filesystem_kinds.contains(&value))
    }

    pub(crate) fn capabilities_complete(&self) -> bool {
        self.capabilities_complete
    }

    pub(crate) fn has_capability(&self, values: &[String]) -> bool {
        values
            .iter()
            .map(|value| normalize(value))
            .any(|value| self.capabilities.contains(&value))
    }

    #[cfg(all(test, target_os = "macos"))]
    pub(crate) fn fixture(
        applications: Vec<InstalledApplication>,
        applications_complete: bool,
    ) -> Self {
        Self::from_system(
            SystemInventory {
                installed_applications: applications,
                installed_applications_complete: applications_complete,
                developer_tools_complete: true,
                filesystem_complete: true,
                capabilities_complete: true,
                ..SystemInventory::default()
            },
            true,
        )
    }
}

fn process_aliases(value: &str) -> Vec<String> {
    let normalized = value.replace('\\', "/");
    let name = normalized.rsplit('/').next().unwrap_or(&normalized);
    let mut aliases = vec![normalize(name)];
    if let Some(without_app) = aliases[0].strip_suffix(".app") {
        aliases.push(without_app.to_string());
    }
    if let Some(without_exe) = aliases[0].strip_suffix(".exe") {
        aliases.push(without_exe.to_string());
    }
    aliases
}

fn normalize(value: &str) -> String {
    value.trim().to_lowercase()
}
