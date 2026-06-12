//! Installed-manifest loading and stale path discovery.

use std::collections::HashSet;
use std::fs;
use std::path::{Path, PathBuf};

use super::{UpdateError, UpdateManifest, ValidatedInstallRoot};

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
    install_root: &ValidatedInstallRoot,
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
            stale.push(install_root.child_path(file)?);
        }
    }
    Ok(stale)
}

fn validate_installed_manifest(
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
