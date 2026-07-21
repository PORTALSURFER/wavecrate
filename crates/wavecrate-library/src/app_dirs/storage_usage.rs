//! On-disk usage reporting for the global database and rebuildable cache.

use std::{fs, io, path::Path};

use cap_primitives::{ambient_authority, fs as capability_fs};

use crate::sample_sources::LIBRARY_DB_FILE_NAME;

use super::app_root_dir;

const CACHE_DIR_NAME: &str = "cache";
const SQLITE_SIDECAR_SUFFIXES: [&str; 3] = ["", "-wal", "-shm"];

/// Logical on-disk bytes owned by the global library database and cache.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct GlobalStorageUsage {
    /// Bytes in `library.db` and its SQLite WAL/shared-memory sidecars.
    pub database_bytes: u64,
    /// Bytes in regular files below the rebuildable global cache root.
    pub cache_bytes: u64,
}

impl GlobalStorageUsage {
    /// Total bytes represented by the database and cache measurements.
    pub fn total_bytes(self) -> u64 {
        self.database_bytes.saturating_add(self.cache_bytes)
    }
}

/// Measure the current global library database and rebuildable cache footprint.
///
/// The traversal does not follow symbolic links, so a linked cache root or
/// descendant cannot make the reported size escape the app-owned boundary.
pub fn global_storage_usage() -> Result<GlobalStorageUsage, String> {
    let root = app_root_dir().map_err(|error| error.to_string())?;
    global_storage_usage_at(&root)
}

fn global_storage_usage_at(root: &Path) -> Result<GlobalStorageUsage, String> {
    let database_bytes = SQLITE_SIDECAR_SUFFIXES
        .into_iter()
        .try_fold(0_u64, |total, suffix| {
            let path = root.join(format!("{LIBRARY_DB_FILE_NAME}{suffix}"));
            checked_add(total, regular_file_size(&path)?, &path)
        })?;
    let cache_root = root.join(CACHE_DIR_NAME);
    let cache_bytes = directory_regular_file_size(&cache_root)?;
    Ok(GlobalStorageUsage {
        database_bytes,
        cache_bytes,
    })
}

fn directory_regular_file_size(root: &Path) -> Result<u64, String> {
    directory_regular_file_size_with_observer(root, |_| {})
}

fn directory_regular_file_size_with_observer(
    root: &Path,
    mut before_descend: impl FnMut(&Path),
) -> Result<u64, String> {
    let root_metadata = match fs::symlink_metadata(root) {
        Ok(metadata) => metadata,
        Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(0),
        Err(error) => return Err(read_error("metadata", root, error)),
    };
    if !root_metadata.file_type().is_dir() {
        return Ok(0);
    }

    let parent = root
        .parent()
        .ok_or_else(|| format!("Global cache root has no parent: {}", root.display()))?;
    let root_name = root
        .file_name()
        .ok_or_else(|| format!("Global cache root has no file name: {}", root.display()))?;
    let parent_directory = capability_fs::open_ambient_dir(parent, ambient_authority())
        .map_err(|error| read_error("cache parent directory", parent, error))?;
    let root_directory =
        match capability_fs::open_dir_nofollow(&parent_directory, Path::new(root_name)) {
            Ok(directory) => directory,
            Err(error) if entry_changed_before_open(&error) => return Ok(0),
            Err(error) => return Err(read_error("cache directory", root, error)),
        };

    let mut total = 0_u64;
    let mut pending = vec![(root_directory, root.to_path_buf())];
    while let Some((directory, display_path)) = pending.pop() {
        let entries = match capability_fs::read_base_dir(&directory) {
            Ok(entries) => entries,
            Err(error) if error.kind() == io::ErrorKind::NotFound => continue,
            Err(error) => return Err(read_error("directory", &display_path, error)),
        };
        for entry in entries {
            let entry = match entry {
                Ok(entry) => entry,
                Err(error) if error.kind() == io::ErrorKind::NotFound => continue,
                Err(error) => return Err(read_error("directory entry", &display_path, error)),
            };
            let path = display_path.join(entry.file_name());
            let file_type = match entry.file_type() {
                Ok(file_type) => file_type,
                Err(error) if error.kind() == io::ErrorKind::NotFound => continue,
                Err(error) => return Err(read_error("file type", &path, error)),
            };
            if file_type.is_dir() {
                before_descend(&path);
                match capability_fs::open_dir_nofollow(&directory, Path::new(&entry.file_name())) {
                    Ok(child) => pending.push((child, path)),
                    Err(error) if entry_changed_before_open(&error) => continue,
                    Err(error) => return Err(read_error("directory", &path, error)),
                }
            } else if file_type.is_file() {
                let metadata = match entry.metadata() {
                    Ok(metadata) => metadata,
                    Err(error) if error.kind() == io::ErrorKind::NotFound => continue,
                    Err(error) => return Err(read_error("metadata", &path, error)),
                };
                if metadata.file_type().is_file() {
                    total = checked_add(total, metadata.len(), &path)?;
                }
            }
        }
    }
    Ok(total)
}

fn entry_changed_before_open(error: &io::Error) -> bool {
    if matches!(
        error.kind(),
        io::ErrorKind::NotFound | io::ErrorKind::NotADirectory
    ) {
        return true;
    }
    #[cfg(unix)]
    if error.raw_os_error() == Some(libc::ELOOP) {
        return true;
    }
    #[cfg(target_os = "windows")]
    if error.raw_os_error() == Some(windows::Win32::Foundation::ERROR_STOPPED_ON_SYMLINK.0 as i32) {
        return true;
    }
    false
}

fn regular_file_size(path: &Path) -> Result<u64, String> {
    match fs::symlink_metadata(path) {
        Ok(metadata) if metadata.file_type().is_file() => Ok(metadata.len()),
        Ok(_) => Ok(0),
        Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(0),
        Err(error) => Err(read_error("metadata", path, error)),
    }
}

fn checked_add(total: u64, bytes: u64, path: &Path) -> Result<u64, String> {
    total.checked_add(bytes).ok_or_else(|| {
        format!(
            "Global storage size overflow while measuring {}",
            path.display()
        )
    })
}

fn read_error(kind: &str, path: &Path, error: io::Error) -> String {
    format!("Failed to read {kind} at {}: {error}", path.display())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn measures_database_sidecars_and_nested_cache_only() {
        let root = tempfile::tempdir().expect("create storage root");
        fs::write(root.path().join(LIBRARY_DB_FILE_NAME), [0_u8; 7]).expect("write database");
        fs::write(root.path().join("library.db-wal"), [0_u8; 5]).expect("write database WAL");
        fs::write(root.path().join("library.db-shm"), [0_u8; 3])
            .expect("write database shared memory");
        fs::write(root.path().join("config.toml"), [0_u8; 19]).expect("write unrelated config");
        let cache = root.path().join(CACHE_DIR_NAME).join("waveforms");
        fs::create_dir_all(&cache).expect("create cache tree");
        fs::write(cache.join("one.cache"), [0_u8; 11]).expect("write first cache payload");
        fs::write(cache.join("two.cache"), [0_u8; 13]).expect("write second cache payload");

        let usage = global_storage_usage_at(root.path()).expect("measure global storage");

        assert_eq!(usage.database_bytes, 15);
        assert_eq!(usage.cache_bytes, 24);
        assert_eq!(usage.total_bytes(), 39);
    }

    #[test]
    fn missing_database_and_cache_report_zero_bytes() {
        let root = tempfile::tempdir().expect("create empty storage root");

        assert_eq!(
            global_storage_usage_at(root.path()).expect("measure empty global storage"),
            GlobalStorageUsage::default()
        );
    }

    #[cfg(unix)]
    #[test]
    fn cache_traversal_does_not_follow_symbolic_links() {
        use std::os::unix::fs::symlink;

        let root = tempfile::tempdir().expect("create storage root");
        let external = tempfile::tempdir().expect("create external root");
        fs::write(external.path().join("large.cache"), [0_u8; 61]).expect("write external payload");
        let cache = root.path().join(CACHE_DIR_NAME);
        fs::create_dir_all(&cache).expect("create cache root");
        fs::write(cache.join("owned.cache"), [0_u8; 5]).expect("write owned payload");
        symlink(external.path(), cache.join("external")).expect("link external directory");

        let usage = global_storage_usage_at(root.path()).expect("measure linked cache");

        assert_eq!(usage.cache_bytes, 5);
    }

    #[cfg(unix)]
    #[test]
    fn cache_traversal_does_not_follow_symbolic_cache_root() {
        use std::os::unix::fs::symlink;

        let root = tempfile::tempdir().expect("create storage root");
        let external = tempfile::tempdir().expect("create external cache root");
        fs::write(external.path().join("external.cache"), [0_u8; 61])
            .expect("write external payload");
        symlink(external.path(), root.path().join(CACHE_DIR_NAME)).expect("link cache root");

        let usage = global_storage_usage_at(root.path()).expect("measure linked cache root");

        assert_eq!(usage.cache_bytes, 0);
    }

    #[cfg(unix)]
    #[test]
    fn cache_traversal_does_not_follow_directory_replaced_by_symbolic_link() {
        use std::os::unix::fs::symlink;

        let root = tempfile::tempdir().expect("create storage root");
        let external = tempfile::tempdir().expect("create external cache root");
        fs::write(external.path().join("external.cache"), [0_u8; 61])
            .expect("write external payload");
        let cache = root.path().join(CACHE_DIR_NAME);
        let replaceable = cache.join("replaceable");
        fs::create_dir_all(&replaceable).expect("create replaceable cache directory");
        fs::write(replaceable.join("owned.cache"), [0_u8; 5]).expect("write owned payload");
        let detached = root.path().join("detached-cache");
        let mut replaced = false;

        let cache_bytes = directory_regular_file_size_with_observer(&cache, |path| {
            if path == replaceable && !replaced {
                fs::rename(&replaceable, &detached).expect("detach enumerated directory");
                symlink(external.path(), &replaceable).expect("replace directory with symlink");
                replaced = true;
            }
        })
        .expect("measure cache during directory replacement");

        assert!(replaced, "replacement hook should observe the directory");
        assert_eq!(cache_bytes, 0);
    }
}
