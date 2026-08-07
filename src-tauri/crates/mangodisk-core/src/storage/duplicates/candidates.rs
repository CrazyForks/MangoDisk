use std::{
    collections::{HashMap, HashSet},
    fs::{self, File},
    path::{Path, PathBuf},
    time::{SystemTime, UNIX_EPOCH},
};

use mangodisk_platform::{current_platform, Platform, ScanPurpose};

use crate::shared::{operation::OperationGuard, TraversalStage};

const PROTECTED_FILE_EXTENSIONS: [&str; 3] = ["bin", "dll", "jar"];

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) struct FileIdentity {
    pub(super) volume: u64,
    pub(super) index: u64,
}

#[derive(Clone)]
pub(super) struct FileCandidate {
    pub(super) root_ordinal: usize,
    pub(super) path: PathBuf,
    pub(super) bytes: u64,
    pub(super) modified_at: Option<SystemTime>,
    pub(super) modified_at_ms: Option<u64>,
    pub(super) identity: Option<FileIdentity>,
}

pub(super) struct PhysicalIdentityFilter {
    pub(super) candidates: Vec<FileCandidate>,
    pub(super) alias_count: usize,
    pub(super) unavailable_count: usize,
}

/// Defines which files and subtrees are meaningful during one duplicate discovery scan.
///
/// The broad-scope decision is intentionally computed once per root. Resolving platform user
/// directories for every candidate previously added repeated filesystem work to the hottest
/// classification path and made native and generic enumeration harder to keep semantically
/// identical.
#[derive(Clone, Copy)]
pub(super) struct DuplicateCandidatePolicy {
    broad_discovery: bool,
}

impl DuplicateCandidatePolicy {
    pub(super) fn for_scan_root(scan_root: &Path) -> Self {
        Self {
            broad_discovery: is_broad_user_scope(scan_root),
        }
    }

    pub(super) fn should_prune_directory(path: &Path) -> bool {
        let Some(name) = path.file_name().and_then(|name| name.to_str()) else {
            return false;
        };
        // Hidden implementation trees are noisy during broad discovery and often contain VCS or
        // tool metadata rather than independent user copies. Visible build and dependency folders
        // deliberately remain eligible: their names are not reliable safety boundaries, and both
        // developers and ordinary applications can store user-managed files inside them.
        name.starts_with('.')
    }

    pub(super) fn should_exclude_file(self, path: &Path) -> bool {
        if path
            .file_name()
            .and_then(|name| name.to_str())
            .is_some_and(|name| name.eq_ignore_ascii_case(".DS_Store"))
        {
            return true;
        }
        self.broad_discovery
            && path
                .extension()
                .and_then(|extension| extension.to_str())
                .is_some_and(|extension| {
                    PROTECTED_FILE_EXTENSIONS
                        .iter()
                        .any(|candidate| extension.eq_ignore_ascii_case(candidate))
                })
    }
}

pub(super) struct CandidateEnumeration<'a> {
    root_ordinal: usize,
    minimum_bytes: u64,
    visit: &'a dyn Fn(TraversalStage, &Path, u64),
    size_groups: &'a mut HashMap<u64, Vec<FileCandidate>>,
    skipped_count: &'a mut u64,
    scanned_file_count: &'a mut u64,
    operation: &'a OperationGuard,
    policy: DuplicateCandidatePolicy,
}

pub(super) struct CandidateEnumerationRequest<'a> {
    pub(super) root_ordinal: usize,
    pub(super) minimum_bytes: u64,
    pub(super) visit: &'a dyn Fn(TraversalStage, &Path, u64),
    pub(super) size_groups: &'a mut HashMap<u64, Vec<FileCandidate>>,
    pub(super) skipped_count: &'a mut u64,
    pub(super) scanned_file_count: &'a mut u64,
    pub(super) operation: &'a OperationGuard,
    pub(super) policy: DuplicateCandidatePolicy,
}

impl<'a> CandidateEnumeration<'a> {
    pub(super) fn new(request: CandidateEnumerationRequest<'a>) -> Self {
        Self {
            root_ordinal: request.root_ordinal,
            minimum_bytes: request.minimum_bytes,
            visit: request.visit,
            size_groups: request.size_groups,
            skipped_count: request.skipped_count,
            scanned_file_count: request.scanned_file_count,
            operation: request.operation,
            policy: request.policy,
        }
    }

    pub(super) fn scan(&mut self, path: &Path, scan_root: &Path) -> Result<(), String> {
        self.operation
            .ensure_not_cancelled()
            .map_err(|error| error.to_string())?;
        let entries = match fs::read_dir(path) {
            Ok(entries) => entries,
            Err(_) => {
                *self.skipped_count += 1;
                return Ok(());
            }
        };
        for entry in entries {
            self.operation
                .ensure_not_cancelled()
                .map_err(|error| error.to_string())?;
            let Ok(entry) = entry else {
                *self.skipped_count += 1;
                continue;
            };
            let child = entry.path();
            if current_platform()
                .should_skip(&child, scan_root, ScanPurpose::DuplicateFiles)
                .is_some()
            {
                *self.skipped_count += 1;
                continue;
            }
            let Ok(metadata) = fs::symlink_metadata(&child) else {
                *self.skipped_count += 1;
                continue;
            };
            if current_platform().is_link_like(&metadata) {
                *self.skipped_count += 1;
                continue;
            }
            if metadata.is_dir() {
                if DuplicateCandidatePolicy::should_prune_directory(&child) {
                    // Broad duplicate discovery should expose independent user copies, not hidden
                    // implementation trees. An explicitly selected hidden root remains inspectable
                    // because only descendants are evaluated here.
                    *self.skipped_count += 1;
                    continue;
                }
                self.scan(&child, scan_root)?;
                continue;
            }
            if !metadata.is_file() {
                continue;
            }
            *self.scanned_file_count += 1;
            (self.visit)(TraversalStage::Analyzing, &child, metadata.len());
            if metadata.len() < self.minimum_bytes {
                continue;
            }
            if self.policy.should_exclude_file(&child) {
                *self.skipped_count += 1;
                continue;
            }
            self.size_groups
                .entry(metadata.len())
                .or_default()
                .push(FileCandidate {
                    root_ordinal: self.root_ordinal,
                    path: child,
                    bytes: metadata.len(),
                    modified_at: metadata.modified().ok(),
                    modified_at_ms: modified_ms(&metadata),
                    identity: initial_file_identity(&metadata),
                });
        }
        Ok(())
    }
}

/// Returns whether one root represents broad discovery rather than a narrowly selected folder.
///
/// Binary payload extensions are poor cleanup candidates during home- or volume-wide discovery,
/// but the exact duplicate engine must still honor a user who explicitly selects a smaller folder
/// containing those file types. Keeping this decision beside candidate classification prevents
/// platform traversal and presentation code from inventing different meanings for the same scan.
fn is_broad_user_scope(scan_root: &Path) -> bool {
    scan_root == current_platform().system_volume_path()
        || current_platform()
            .user_directories()
            .is_ok_and(|directories| directories.home_directory().starts_with(scan_root))
}

/// Returns the generated or hidden subtree containing a native candidate.
///
/// Generic traversal stops at these directories before reading their children. Native filesystem
/// enumeration may discover candidates without walking the same call stack, so this component
/// check preserves identical product semantics instead of leaking build artifacts into results.
pub(super) fn pruned_directory_ancestor(scan_root: &Path, path: &Path) -> Option<PathBuf> {
    let relative = path.strip_prefix(scan_root).ok()?;
    let mut ancestor = scan_root.to_path_buf();
    for component in relative.parent()?.components() {
        ancestor.push(component.as_os_str());
        if component.as_os_str().to_str().is_none() {
            continue;
        }
        if DuplicateCandidatePolicy::should_prune_directory(&ancestor) {
            return Some(ancestor);
        }
    }
    None
}

/// Revalidates one untrusted native candidate before it enters size grouping.
///
/// Platform-native enumeration only accelerates discovery. Core still owns link rejection,
/// purpose-specific protection, live size and modification time, and stable file identity.
pub(super) struct NativeCandidateRequest<'a> {
    pub(super) root_ordinal: usize,
    pub(super) path: PathBuf,
    pub(super) scan_root: &'a Path,
    pub(super) minimum_bytes: u64,
    pub(super) size_groups: &'a mut HashMap<u64, Vec<FileCandidate>>,
    pub(super) skipped_count: &'a mut u64,
    pub(super) operation: &'a OperationGuard,
    pub(super) policy: DuplicateCandidatePolicy,
}

pub(super) fn collect_native_candidate(request: NativeCandidateRequest<'_>) -> Result<(), String> {
    request
        .operation
        .ensure_not_cancelled()
        .map_err(|error| error.to_string())?;
    if current_platform()
        .should_skip(
            &request.path,
            request.scan_root,
            ScanPurpose::DuplicateFiles,
        )
        .is_some()
    {
        *request.skipped_count = request.skipped_count.saturating_add(1);
        return Ok(());
    }
    let Ok(metadata) = fs::symlink_metadata(&request.path) else {
        *request.skipped_count = request.skipped_count.saturating_add(1);
        return Ok(());
    };
    if !metadata.is_file() || current_platform().is_link_like(&metadata) {
        *request.skipped_count = request.skipped_count.saturating_add(1);
        return Ok(());
    }
    let bytes = metadata.len();
    let excluded = request.policy.should_exclude_file(&request.path);
    if bytes < request.minimum_bytes || excluded {
        if bytes >= request.minimum_bytes && excluded {
            *request.skipped_count = request.skipped_count.saturating_add(1);
        }
        return Ok(());
    }
    request
        .size_groups
        .entry(bytes)
        .or_default()
        .push(FileCandidate {
            root_ordinal: request.root_ordinal,
            path: request.path,
            bytes,
            modified_at: metadata.modified().ok(),
            modified_at_ms: modified_ms(&metadata),
            identity: initial_file_identity(&metadata),
        });
    Ok(())
}

pub(super) fn remove_physical_aliases(
    candidates: Vec<FileCandidate>,
    mut observe: impl FnMut(&Path) -> Result<(), String>,
) -> Result<PhysicalIdentityFilter, String> {
    let mut identities = HashSet::<(u64, u64)>::new();
    let mut alias_count = 0_usize;
    let mut unavailable_count = 0_usize;
    let mut unique_candidates = Vec::with_capacity(candidates.len());
    for mut candidate in candidates {
        observe(&candidate.path)?;
        if candidate.identity.is_none() {
            candidate.identity = load_file_identity(&candidate.path, candidate.bytes);
        }
        let Some(identity) = candidate.identity else {
            // Exact duplicate deletion requires stable physical identity. An unavailable identity
            // fails closed because a same-size path replacement cannot be proven safe.
            unavailable_count = unavailable_count.saturating_add(1);
            continue;
        };
        if identities.insert((identity.volume, identity.index)) {
            unique_candidates.push(candidate);
        } else {
            alias_count = alias_count.saturating_add(1);
        }
    }
    Ok(PhysicalIdentityFilter {
        candidates: unique_candidates,
        alias_count,
        unavailable_count,
    })
}

pub(super) fn normalize_roots(roots: Vec<String>) -> Result<Vec<PathBuf>, String> {
    let mut canonical = roots
        .into_iter()
        .filter(|value| !value.trim().is_empty())
        .map(PathBuf::from)
        .map(|path| {
            let canonical = current_platform()
                .canonicalize_no_links(&path)
                .map_err(|error| format!("the duplicate-file scan root is unsafe: {error}"))?;
            let metadata = fs::symlink_metadata(&canonical).map_err(|error| {
                format!("failed to access the duplicate-file scan root: {error}")
            })?;
            if !metadata.is_dir() {
                return Err(
                    "the duplicate-file scan root must be a directory or volume".to_string()
                );
            }
            Ok(canonical)
        })
        .collect::<Result<Vec<_>, _>>()?;
    if canonical.iter().any(|path| !path.is_dir()) {
        return Err("the duplicate-file scan root must be a directory or volume".to_string());
    }
    // Root ordinals enter cache keys and file facts. Native path ordering provides a stable
    // secondary key without lossy string conversion when equal-depth roots are reordered.
    canonical.sort_by(|left, right| {
        left.components()
            .count()
            .cmp(&right.components().count())
            .then_with(|| left.cmp(right))
    });
    canonical.dedup();
    let mut normalized = Vec::<PathBuf>::new();
    for path in canonical {
        if !normalized.iter().any(|root| path.starts_with(root)) {
            normalized.push(path);
        }
    }
    Ok(normalized)
}

pub(super) fn validate_open_file(
    candidate: &FileCandidate,
    file: &File,
    verify_identity: bool,
) -> Result<(), String> {
    let metadata = file.metadata().map_err(|error| error.to_string())?;
    if !metadata.is_file()
        || metadata.len() != candidate.bytes
        || metadata.modified().ok() != candidate.modified_at
        || (verify_identity
            && candidate
                .identity
                .is_some_and(|expected| file_identity(file, &metadata) != Some(expected)))
    {
        return Err("the file changed during duplicate-content verification".to_string());
    }
    Ok(())
}

pub(super) fn validate_current_path(candidate: &FileCandidate) -> Result<(), String> {
    let metadata = fs::symlink_metadata(&candidate.path).map_err(|error| error.to_string())?;
    if !metadata.is_file()
        || current_platform().is_link_like(&metadata)
        || metadata.len() != candidate.bytes
        || metadata.modified().ok() != candidate.modified_at
    {
        return Err("the file path changed during duplicate-content verification".to_string());
    }
    if candidate
        .identity
        .is_some_and(|expected| path_file_identity(&candidate.path, &metadata) != Some(expected))
    {
        return Err(
            "the file path now refers to a different object during duplicate-content verification"
                .to_string(),
        );
    }
    Ok(())
}

#[cfg(unix)]
fn initial_file_identity(metadata: &fs::Metadata) -> Option<FileIdentity> {
    use std::os::unix::fs::MetadataExt;
    Some(FileIdentity {
        volume: metadata.dev(),
        index: metadata.ino(),
    })
}

#[cfg(windows)]
fn initial_file_identity(_metadata: &fs::Metadata) -> Option<FileIdentity> {
    None
}

#[cfg(unix)]
fn file_identity(_file: &File, metadata: &fs::Metadata) -> Option<FileIdentity> {
    initial_file_identity(metadata)
}

#[cfg(windows)]
fn file_identity(file: &File, _metadata: &fs::Metadata) -> Option<FileIdentity> {
    use std::os::windows::io::AsRawHandle;
    use windows_sys::Win32::Storage::FileSystem::{
        GetFileInformationByHandle, BY_HANDLE_FILE_INFORMATION,
    };

    let mut information = BY_HANDLE_FILE_INFORMATION::default();
    // Stable NTFS identity needs the read-only Win32 volume serial and file index because stable
    // Rust metadata APIs do not expose both values. `File` owns and closes the borrowed handle.
    let succeeded =
        unsafe { GetFileInformationByHandle(file.as_raw_handle(), &mut information) } != 0;
    succeeded.then_some(FileIdentity {
        volume: u64::from(information.dwVolumeSerialNumber),
        index: (u64::from(information.nFileIndexHigh) << 32) | u64::from(information.nFileIndexLow),
    })
}

fn path_file_identity(path: &Path, metadata: &fs::Metadata) -> Option<FileIdentity> {
    initial_file_identity(metadata).or_else(|| {
        let file = File::open(path).ok()?;
        let opened_metadata = file.metadata().ok()?;
        file_identity(&file, &opened_metadata)
    })
}

pub(super) fn load_file_identity(path: &Path, _expected_bytes: u64) -> Option<FileIdentity> {
    let file = File::open(path).ok()?;
    let metadata = file.metadata().ok()?;
    file_identity(&file, &metadata)
}

pub(super) fn modified_ms(metadata: &fs::Metadata) -> Option<u64> {
    metadata
        .modified()
        .ok()
        .and_then(|value| value.duration_since(UNIX_EPOCH).ok())
        .map(|duration| duration.as_millis() as u64)
}

#[cfg(test)]
mod tests {
    use std::path::{Path, PathBuf};

    use mangodisk_platform::{current_platform, Platform};

    use super::{pruned_directory_ancestor, DuplicateCandidatePolicy};

    #[test]
    fn native_candidates_preserve_hidden_subtree_exclusions() {
        let root = Path::new("/workspace");
        assert_eq!(
            pruned_directory_ancestor(
                root,
                Path::new("/workspace/project/.cache/package/archive.bin")
            ),
            Some(PathBuf::from("/workspace/project/.cache"))
        );
        assert_eq!(
            pruned_directory_ancestor(
                root,
                Path::new("/workspace/project/node_modules/package/archive.bin")
            ),
            None
        );
    }

    #[test]
    fn an_explicit_pruned_root_is_not_silently_excluded() {
        let root = Path::new("/workspace/node_modules");
        assert_eq!(
            pruned_directory_ancestor(
                root,
                Path::new("/workspace/node_modules/package/archive.bin")
            ),
            None
        );
    }

    #[test]
    fn discovery_prunes_hidden_but_not_visible_technical_directories() {
        assert!(DuplicateCandidatePolicy::should_prune_directory(Path::new(
            "/workspace/.cache"
        )));
        assert!(!DuplicateCandidatePolicy::should_prune_directory(
            Path::new("/workspace/node_modules")
        ));
        assert!(!DuplicateCandidatePolicy::should_prune_directory(
            Path::new("/workspace/project/target")
        ));
        assert!(!DuplicateCandidatePolicy::should_prune_directory(
            Path::new("/workspace/project/build")
        ));
        assert!(!DuplicateCandidatePolicy::should_prune_directory(
            Path::new("/workspace/Documents")
        ));
    }

    #[test]
    fn protected_payload_extensions_apply_only_to_broad_discovery() {
        let volume_root = current_platform().system_volume_path();
        let broad_payload = volume_root.join("payload.bin");
        assert!(DuplicateCandidatePolicy::for_scan_root(&volume_root)
            .should_exclude_file(&broad_payload));

        let narrow_root = std::env::temp_dir().join("mangodisk-explicit-duplicate-scope");
        let narrow_policy = DuplicateCandidatePolicy::for_scan_root(&narrow_root);
        assert!(!narrow_policy.should_exclude_file(&narrow_root.join("payload.bin")));
        assert!(narrow_policy.should_exclude_file(&narrow_root.join(".DS_Store")));
    }

    #[test]
    fn user_container_is_treated_as_broad_discovery() {
        let directories = current_platform()
            .user_directories()
            .expect("platform user directories should be available");
        let home = directories.home_directory();
        let user_container = home
            .parent()
            .expect("the user home should have a parent directory");

        assert!(DuplicateCandidatePolicy::for_scan_root(user_container)
            .should_exclude_file(&user_container.join("payload.bin")));
    }
}
