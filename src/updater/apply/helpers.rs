use std::collections::HashSet;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use serde_json;

use super::{StaleRemovalFailure, UpdateError, UpdateManifest, ensure_child_path, fs_ops};

/// Resolve the root payload directory for an update archive.
pub(super) fn validate_root_dir(unpack_dir: &Path, expected: &str) -> Result<PathBuf, UpdateError> {
    let expected_root = unpack_dir.join(expected);
    if expected_root.is_dir() {
        return Ok(expected_root);
    }
    if unpack_dir.join("update-manifest.json").is_file() {
        return Ok(unpack_dir.to_path_buf());
    }
    let entries = fs_ops::list_root_entries(unpack_dir)?;
    let mut dirs = entries
        .into_iter()
        .filter(|p| p.is_dir())
        .collect::<Vec<_>>();
    if dirs.len() != 1 {
        return Err(UpdateError::Invalid(
            "Archive must contain exactly one root directory".into(),
        ));
    }
    let root = dirs.pop().unwrap();
    let Some(name) = root.file_name().and_then(|s| s.to_str()) else {
        return Err(UpdateError::Invalid(
            "Invalid archive root directory".into(),
        ));
    };
    if name != expected {
        return Err(UpdateError::Invalid(format!(
            "Archive root directory must be '{expected}/', got '{name}/'"
        )));
    }
    Ok(root)
}

/// Load the installed manifest if available.
pub(super) fn load_installed_manifest(
    install_dir: &Path,
) -> Result<Option<UpdateManifest>, UpdateError> {
    let manifest_path = install_dir.join("update-manifest.json");
    if !manifest_path.is_file() {
        return Ok(None);
    }
    let manifest_bytes = fs::read(&manifest_path)?;
    let manifest: UpdateManifest = serde_json::from_slice(&manifest_bytes)?;
    Ok(Some(manifest))
}

pub(super) fn collect_stale_files(
    install_dir: &Path,
    installed: &UpdateManifest,
    current: &UpdateManifest,
) -> Result<Vec<PathBuf>, UpdateError> {
    validate_installed_manifest(installed, current)?;
    let current_files = current
        .files
        .iter()
        .map(|file| file.as_str())
        .collect::<HashSet<_>>();
    let mut stale = Vec::new();
    for file in installed.files.iter() {
        if !current_files.contains(file.as_str()) {
            stale.push(ensure_child_path(install_dir, file)?);
        }
    }
    Ok(stale)
}

pub(super) fn validate_installed_manifest(
    installed: &UpdateManifest,
    current: &UpdateManifest,
) -> Result<(), UpdateError> {
    if installed.app != current.app {
        return Err(UpdateError::Invalid(format!(
            "Installed manifest app mismatch: expected {}, got {}",
            current.app, installed.app
        )));
    }
    if installed.target != current.target {
        return Err(UpdateError::Invalid(format!(
            "Installed manifest target mismatch: expected {}, got {}",
            current.target, installed.target
        )));
    }
    if installed.platform != current.platform {
        return Err(UpdateError::Invalid(format!(
            "Installed manifest platform mismatch: expected {}, got {}",
            current.platform, installed.platform
        )));
    }
    if installed.arch != current.arch {
        return Err(UpdateError::Invalid(format!(
            "Installed manifest arch mismatch: expected {}, got {}",
            current.arch, installed.arch
        )));
    }
    if installed.files.is_empty() {
        return Err(UpdateError::Invalid(
            "Installed manifest files list is empty".into(),
        ));
    }
    Ok(())
}

pub(super) fn remove_stale_paths(
    paths: &[PathBuf],
    install_dir: &Path,
) -> Result<Vec<StaleRemovalFailure>, UpdateError> {
    let mut failures = Vec::new();
    for path in paths {
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

/// Relaunch helper for an updated executable.
pub(super) fn relaunch_app(
    install_dir: &Path,
    app: &str,
    manifest: &UpdateManifest,
) -> Result<(), UpdateError> {
    let candidate = app_executable_name(app, manifest);
    let exe = install_dir.join(&candidate);
    if !exe.exists() {
        return Err(UpdateError::Invalid(format!(
            "Updated executable missing: {}",
            exe.display()
        )));
    }
    let exe_display = exe.display().to_string();
    let mut cmd = Command::new(&exe);
    cmd.spawn()
        .map_err(|err| UpdateError::Invalid(format!("Failed to relaunch {exe_display}: {err}")))?;
    Ok(())
}

fn app_executable_name(app: &str, manifest: &UpdateManifest) -> String {
    let exe = format!("{app}.exe");
    if manifest.files.iter().any(|f| f == &exe) {
        return exe;
    }
    app.to_string()
}
