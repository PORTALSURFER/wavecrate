//! Installation-path validation for updater writes.
//!
//! The updater refuses absolute, parent-traversing, or symlinked paths so a
//! release archive cannot escape the installation directory while applying an
//! update.

use std::{
    fs,
    io::ErrorKind,
    path::{Component, Path, PathBuf},
};

#[cfg(test)]
use std::sync::{Mutex, OnceLock};

use super::UpdateError;

/// Return a validated child path rooted under an installation directory.
pub(crate) fn ensure_child_path(dir: &Path, name: &str) -> Result<PathBuf, UpdateError> {
    let relative = sanitize_relative_path(name)?;
    let dir = dir
        .canonicalize()
        .map_err(|err| UpdateError::Invalid(format!("Invalid install dir: {err}")))?;
    let candidate = dir.join(relative);
    if !candidate.starts_with(&dir) {
        return Err(UpdateError::Invalid(format!(
            "Refusing to write outside install dir: {}",
            candidate.display()
        )));
    }
    ensure_no_symlink_path(&candidate)?;
    Ok(candidate)
}

fn ensure_no_symlink_path(path: &Path) -> Result<(), UpdateError> {
    let mut current = PathBuf::new();
    for component in path.components() {
        match component {
            #[cfg(windows)]
            Component::Prefix(_) => {
                current.push(component.as_os_str());
                continue;
            }
            Component::RootDir => {
                current.push(component.as_os_str());
                continue;
            }
            _ => {
                current.push(component.as_os_str());
            }
        }
        match symlink_metadata(&current) {
            Ok(metadata) => {
                if metadata.file_type().is_symlink() {
                    return Err(UpdateError::Invalid(format!(
                        "Refusing to write through symlink: {}",
                        current.display()
                    )));
                }
            }
            Err(err) if err.kind() == ErrorKind::NotFound => break,
            Err(err) => {
                if allow_symlink_validation_errors() {
                    return Ok(());
                }
                return Err(UpdateError::Invalid(format!(
                    "Failed to validate update path for symlinks at {}: {err}",
                    current.display()
                )));
            }
        }
    }
    Ok(())
}

fn allow_symlink_validation_errors() -> bool {
    std::env::var("SEMPAL_UPDATER_ALLOW_SYMLINK_ERRORS")
        .ok()
        .map(|value| {
            let value = value.trim().to_ascii_lowercase();
            matches!(value.as_str(), "1" | "true" | "yes")
        })
        .unwrap_or(false)
}

#[cfg(test)]
fn symlink_metadata(path: &Path) -> std::io::Result<fs::Metadata> {
    if let Some(hook) = SYMLINK_METADATA_HOOK.get()
        && let Ok(guard) = hook.lock()
        && let Some(hook) = *guard
    {
        return hook(path);
    }
    fs::symlink_metadata(path)
}

#[cfg(not(test))]
fn symlink_metadata(path: &Path) -> std::io::Result<fs::Metadata> {
    fs::symlink_metadata(path)
}

fn sanitize_relative_path(name: &str) -> Result<PathBuf, UpdateError> {
    let mut sanitized = PathBuf::new();
    let mut saw_component = false;
    for component in Path::new(name).components() {
        match component {
            Component::CurDir => {}
            Component::Normal(part) => {
                sanitized.push(part);
                saw_component = true;
            }
            Component::ParentDir | Component::RootDir | Component::Prefix(_) => {
                return Err(UpdateError::Invalid(format!("Invalid update path: {name}")));
            }
        }
    }
    if !saw_component {
        return Err(UpdateError::Invalid(format!("Invalid update path: {name}")));
    }
    Ok(sanitized)
}

#[cfg(test)]
type SymlinkMetadataHook = fn(&Path) -> std::io::Result<fs::Metadata>;

#[cfg(test)]
static SYMLINK_METADATA_HOOK: OnceLock<Mutex<Option<SymlinkMetadataHook>>> = OnceLock::new();

#[cfg(test)]
struct SymlinkMetadataHookGuard {
    prev: Option<SymlinkMetadataHook>,
}

#[cfg(test)]
impl SymlinkMetadataHookGuard {
    fn new(hook: Option<SymlinkMetadataHook>) -> Self {
        let cell = SYMLINK_METADATA_HOOK.get_or_init(|| Mutex::new(None));
        let mut guard = cell.lock().expect("symlink metadata hook lock");
        let prev = std::mem::replace(&mut *guard, hook);
        Self { prev }
    }
}

#[cfg(test)]
impl Drop for SymlinkMetadataHookGuard {
    fn drop(&mut self) {
        if let Some(cell) = SYMLINK_METADATA_HOOK.get() {
            let mut guard = cell.lock().expect("symlink metadata hook lock");
            *guard = self.prev;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io;
    use std::sync::{Mutex, OnceLock};
    use tempfile::tempdir;

    struct EnvVarGuard {
        key: String,
        previous: Option<String>,
    }

    impl EnvVarGuard {
        fn set(key: &str, value: &str) -> Self {
            let previous = std::env::var(key).ok();
            unsafe {
                std::env::set_var(key, value);
            }
            Self {
                key: key.to_string(),
                previous,
            }
        }
    }

    impl Drop for EnvVarGuard {
        fn drop(&mut self) {
            if let Some(value) = self.previous.take() {
                unsafe {
                    std::env::set_var(&self.key, value);
                }
            } else {
                unsafe {
                    std::env::remove_var(&self.key);
                }
            }
        }
    }

    fn updater_test_lock() -> &'static Mutex<()> {
        static LOCK: OnceLock<Mutex<()>> = OnceLock::new();
        LOCK.get_or_init(|| Mutex::new(()))
    }

    #[test]
    fn ensure_child_path_rejects_parent_dir() {
        let _lock = updater_test_lock().lock().expect("updater test lock");
        let dir = tempdir().expect("tempdir");
        let err = ensure_child_path(dir.path(), "../evil.txt").expect_err("parent dir must fail");
        assert!(err.to_string().contains("Invalid update path"));
    }

    #[test]
    fn ensure_child_path_rejects_absolute_path() {
        let _lock = updater_test_lock().lock().expect("updater test lock");
        let dir = tempdir().expect("tempdir");
        #[cfg(windows)]
        let name = "C:\\evil.txt";
        #[cfg(not(windows))]
        let name = "/tmp/evil.txt";
        let err = ensure_child_path(dir.path(), name).expect_err("absolute path must fail");
        assert!(err.to_string().contains("Invalid update path"));
    }

    #[test]
    fn ensure_child_path_allows_relative_path() {
        let _lock = updater_test_lock().lock().expect("updater test lock");
        let _guard = EnvVarGuard::set("SEMPAL_UPDATER_ALLOW_SYMLINK_ERRORS", "1");
        let dir = tempdir().expect("tempdir");
        let path = ensure_child_path(dir.path(), "./ok/file.txt").expect("relative path");
        let canonical = dir.path().canonicalize().expect("canonical install dir");
        assert!(path.starts_with(&canonical));
        assert!(path.ends_with(Path::new("ok").join("file.txt")));
    }

    #[cfg(unix)]
    #[test]
    fn ensure_child_path_rejects_symlinked_component() {
        use std::os::unix::fs::symlink;

        let _lock = updater_test_lock().lock().expect("updater test lock");
        let dir = tempdir().expect("tempdir");
        let install = dir.path().join("install");
        let external = dir.path().join("external");
        fs::create_dir_all(&install).expect("install dir");
        fs::create_dir_all(&external).expect("external dir");
        let link = install.join("link");
        symlink(&external, &link).expect("symlink");

        let err = ensure_child_path(&install, "link/file.txt").expect_err("symlink must fail");
        assert!(err.to_string().contains("symlink"));
    }

    #[test]
    fn ensure_child_path_fails_on_symlink_metadata_error() {
        let _lock = updater_test_lock().lock().expect("updater test lock");
        fn fail_metadata(_: &Path) -> io::Result<fs::Metadata> {
            Err(io::Error::new(
                ErrorKind::PermissionDenied,
                "metadata denied",
            ))
        }

        let _env = EnvVarGuard::set("SEMPAL_UPDATER_ALLOW_SYMLINK_ERRORS", "0");
        let _guard = SymlinkMetadataHookGuard::new(Some(fail_metadata));
        let dir = tempdir().expect("tempdir");
        let err = ensure_child_path(dir.path(), "ok/file.txt")
            .expect_err("metadata failures must fail closed");
        assert!(
            err.to_string()
                .contains("Failed to validate update path for symlinks")
        );
    }
}
