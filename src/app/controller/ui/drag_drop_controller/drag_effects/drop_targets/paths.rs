//! Destination-path helpers for drop-target copy and move batches.

use crate::app::controller::jobs::DropTargetTransferKind;
use std::ffi::OsStr;
use std::path::{Path, PathBuf};

/// Build the destination-relative path for a drop-target move.
pub(super) fn move_destination_relative(
    target_root: &Path,
    target_folder: &Path,
    file_name: &OsStr,
) -> Result<PathBuf, String> {
    let relative = join_target_relative_path(target_folder, file_name);
    let destination = target_root.join(&relative);
    if destination.exists() {
        return Err(format!(
            "A file already exists at {}",
            destination.display()
        ));
    }
    Ok(relative)
}

/// Build the destination-relative path for a drop-target copy, adding a suffix on collision.
pub(super) fn copy_destination_relative(
    target_root: &Path,
    target_folder: &Path,
    file_name: &OsStr,
) -> Result<PathBuf, String> {
    let base = join_target_relative_path(target_folder, file_name);
    if !target_root.join(&base).exists() {
        return Ok(base);
    }
    let stem = Path::new(file_name)
        .file_stem()
        .map(|name| name.to_string_lossy().to_string())
        .unwrap_or_else(|| "sample".to_string());
    let extension = Path::new(file_name)
        .extension()
        .map(|ext| ext.to_string_lossy().to_string());
    for index in 1..=999 {
        let file_name = duplicate_copy_name(&stem, extension.as_deref(), index);
        let candidate = join_target_relative_path(target_folder, file_name.as_ref());
        if !target_root.join(&candidate).exists() {
            return Ok(candidate);
        }
    }
    Err("Failed to find destination file name".into())
}

/// Format the progress title for a drop-target transfer batch.
pub(super) fn progress_title(kind: DropTargetTransferKind, count: usize) -> &'static str {
    match (kind, count) {
        (DropTargetTransferKind::Copy, 1) => "Copying sample",
        (DropTargetTransferKind::Copy, _) => "Copying samples",
        (DropTargetTransferKind::Move, 1) => "Moving sample",
        (DropTargetTransferKind::Move, _) => "Moving samples",
    }
}

fn join_target_relative_path(target_folder: &Path, file_name: &OsStr) -> PathBuf {
    if target_folder.as_os_str().is_empty() {
        PathBuf::from(file_name)
    } else {
        target_folder.join(file_name)
    }
}

fn duplicate_copy_name(stem: &str, extension: Option<&str>, index: usize) -> String {
    let suffix = format!("{stem}_copy{index:03}");
    extension.map_or(suffix.clone(), |ext| format!("{suffix}.{ext}"))
}
