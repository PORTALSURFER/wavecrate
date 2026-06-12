//! Stale installed-path removal and empty-parent pruning.

use std::fs;
use std::path::{Path, PathBuf};

use super::{StaleRemovalFailure, UpdateError, ValidatedInstallRoot};

pub(super) fn remove_stale_paths(
    paths: &[PathBuf],
    install_root: &ValidatedInstallRoot,
) -> Result<Vec<StaleRemovalFailure>, UpdateError> {
    let install_dir = install_root.path();
    let mut failures = Vec::new();
    for path in paths {
        ensure_stale_path_is_in_install_root(path, install_dir)?;
        if !path.exists() {
            continue;
        }
        match fs::symlink_metadata(path) {
            Ok(metadata) if metadata.file_type().is_symlink() => {
                if let Err(err) = fs::remove_file(path) {
                    failures.push(stale_removal_failure(path, err));
                }
            }
            Ok(metadata) if metadata.is_dir() => {
                if let Err(err) = fs::remove_dir_all(path) {
                    failures.push(stale_removal_failure(path, err));
                }
            }
            Ok(_) => {
                if let Err(err) = fs::remove_file(path) {
                    failures.push(stale_removal_failure(path, err));
                }
            }
            Err(err) if err.kind() == std::io::ErrorKind::NotFound => continue,
            Err(err) => {
                failures.push(stale_removal_failure(
                    path,
                    format!("metadata error: {err}"),
                ));
            }
        }
        let _ = prune_empty_parents(install_dir, path);
    }
    Ok(failures)
}

fn ensure_stale_path_is_in_install_root(
    path: &Path,
    install_dir: &Path,
) -> Result<(), UpdateError> {
    if path.starts_with(install_dir) {
        return Ok(());
    }
    Err(UpdateError::Invalid(format!(
        "Refusing to remove stale path outside install dir: {}",
        path.display()
    )))
}

fn stale_removal_failure(path: &Path, error: impl ToString) -> StaleRemovalFailure {
    StaleRemovalFailure {
        path: path.to_path_buf(),
        error: error.to_string(),
    }
}

fn prune_empty_parents(install_dir: &Path, path: &Path) -> Result<(), UpdateError> {
    let mut current = path.parent();
    while let Some(dir) = current {
        if dir == install_dir {
            break;
        }
        let metadata = fs::symlink_metadata(dir)?;
        if metadata.file_type().is_symlink() || !metadata.is_dir() {
            break;
        }
        if fs::read_dir(dir)?.next().is_some() {
            break;
        }
        fs::remove_dir(dir)?;
        current = dir.parent();
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn remove_stale_paths_rejects_paths_outside_install_root() {
        let tmp = tempdir().expect("tempdir");
        let install_dir = tmp.path().join("install");
        let outside = tmp.path().join("outside.txt");
        fs::create_dir_all(&install_dir).expect("install dir");
        fs::write(&outside, "outside").expect("outside file");
        let install_root = ValidatedInstallRoot::new(&install_dir).expect("validated root");

        let err = remove_stale_paths(std::slice::from_ref(&outside), &install_root)
            .expect_err("outside stale path must fail");

        assert!(err.to_string().contains("outside install dir"));
        assert!(outside.exists());
    }

    #[test]
    fn remove_stale_paths_prunes_empty_parents_but_stops_at_install_root() {
        let tmp = tempdir().expect("tempdir");
        let install_dir = tmp.path().join("install");
        let nested_dir = install_dir.join("stale").join("nested");
        fs::create_dir_all(&nested_dir).expect("nested dir");
        let install_root = ValidatedInstallRoot::new(&install_dir).expect("validated root");
        let stale_file = install_root
            .child_path("stale/nested/old.txt")
            .expect("stale child path");
        fs::write(&stale_file, "old").expect("stale file");

        let failures = remove_stale_paths(std::slice::from_ref(&stale_file), &install_root)
            .expect("remove stale paths");

        assert!(failures.is_empty());
        assert!(!stale_file.exists());
        assert!(!install_dir.join("stale").exists());
        assert!(install_dir.exists());
    }
}
