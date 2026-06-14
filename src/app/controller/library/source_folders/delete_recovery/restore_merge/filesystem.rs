//! Direct filesystem operations for retained-restore merge execution.

use std::{fs, path::Path};

pub(super) fn restore_retained_folder(
    staged_dir: &Path,
    target_dir: &Path,
    original_relative: &Path,
) -> Result<(), String> {
    prepare_parent(target_dir, "restore destination")?;
    fs::rename(staged_dir, target_dir).map_err(|err| {
        format!(
            "Failed to restore retained folder {}: {err}",
            original_relative.display()
        )
    })
}

pub(super) fn restore_retained_file(
    staged_file: &Path,
    target_file: &Path,
    original_relative: &Path,
) -> Result<(), String> {
    prepare_parent(target_file, "restore destination")?;
    fs::rename(staged_file, target_file).map_err(|err| {
        format!(
            "Failed to restore retained file {}: {err}",
            original_relative.display()
        )
    })
}

pub(super) fn restore_retained_entry(staged_path: &Path, target_path: &Path) -> Result<(), String> {
    prepare_parent(target_path, "restore destination")?;
    fs::rename(staged_path, target_path).map_err(|err| {
        format!(
            "Failed to restore retained entry {}: {err}",
            staged_path.display()
        )
    })
}

pub(super) fn preserve_existing_conflict_copy(
    target_file: &Path,
    backup: &Path,
) -> Result<(), String> {
    prepare_parent(backup, "conflict backup")?;
    fs::rename(target_file, backup)
        .map_err(|err| format!("Failed to preserve newer conflict copy: {err}"))
}

pub(super) fn preserve_retained_conflict(
    staged_path: &Path,
    fallback: &Path,
) -> Result<(), String> {
    prepare_parent(fallback, "restore destination")?;
    fs::rename(staged_path, fallback).map_err(|err| {
        format!(
            "Failed to preserve retained conflict {}: {err}",
            staged_path.display()
        )
    })
}

pub(super) fn discard_duplicate_staged_file(staged_file: &Path) -> Result<(), String> {
    fs::remove_file(staged_file)
        .map_err(|err| format!("Failed to discard duplicate staged file: {err}"))
}

fn prepare_parent(path: &Path, context: &str) -> Result<(), String> {
    let Some(parent) = path.parent() else {
        return Ok(());
    };
    fs::create_dir_all(parent)
        .map_err(|err| format!("Failed to prepare {context} {}: {err}", parent.display()))
}
