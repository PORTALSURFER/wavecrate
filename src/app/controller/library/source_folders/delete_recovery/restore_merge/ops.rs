//! Filesystem-side retained-restore merge operations.

use super::util::{
    files_match, modified_nanos, read_dir_paths, remove_dir_if_empty, source_relative,
    timestamped_conflict_path,
};
use super::{
    ExistingFileRelocation, RestoredFileDisposition, RestoredFileRecord, RetainedRestoreMergeReport,
};
use std::fs;
use std::path::Path;
use time::{OffsetDateTime, format_description::FormatItem, macros::format_description};

pub(super) fn merge_directory(
    staged_dir: &Path,
    target_dir: &Path,
    source_root: &Path,
    original_relative: &Path,
    stamp: &str,
    report: &mut RetainedRestoreMergeReport,
) -> Result<(), String> {
    if !target_dir.exists() {
        return move_whole_directory(
            staged_dir,
            target_dir,
            source_root,
            original_relative,
            original_relative,
            RestoredFileDisposition::RestoredCanonical,
            report,
        );
    }
    if !target_dir.is_dir() {
        report.had_conflicts = true;
        let fallback = timestamped_conflict_path(target_dir, "recovered", stamp);
        let final_relative = source_relative(source_root, &fallback)?;
        return move_whole_directory(
            staged_dir,
            &fallback,
            source_root,
            original_relative,
            &final_relative,
            RestoredFileDisposition::RestoredTimestamped,
            report,
        );
    }
    for staged_child in read_dir_paths(staged_dir)? {
        let name = staged_child
            .file_name()
            .ok_or_else(|| format!("Staged path missing file name: {}", staged_child.display()))?;
        let target_child = target_dir.join(name);
        let child_relative = source_relative(source_root, &target_child)?;
        if staged_child.is_dir() {
            merge_directory(
                &staged_child,
                &target_child,
                source_root,
                &child_relative,
                stamp,
                report,
            )?;
        } else if staged_child.is_file() {
            merge_file(
                &staged_child,
                &target_child,
                source_root,
                &child_relative,
                stamp,
                report,
            )?;
        } else {
            move_non_file_conflict(&staged_child, &target_child, source_root, stamp, report)?;
        }
    }
    remove_dir_if_empty(staged_dir)?;
    Ok(())
}

pub(super) fn utc_conflict_stamp() -> Result<String, String> {
    static FORMAT: &[FormatItem<'static>] =
        format_description!("[year][month][day]T[hour][minute][second]Z");
    OffsetDateTime::now_utc()
        .format(FORMAT)
        .map_err(|err| format!("Failed to format retained restore timestamp: {err}"))
}

fn move_whole_directory(
    staged_dir: &Path,
    target_dir: &Path,
    source_root: &Path,
    original_base_relative: &Path,
    final_base_relative: &Path,
    disposition: RestoredFileDisposition,
    report: &mut RetainedRestoreMergeReport,
) -> Result<(), String> {
    if let Some(parent) = target_dir.parent() {
        fs::create_dir_all(parent).map_err(|err| {
            format!(
                "Failed to prepare restore destination {}: {err}",
                parent.display()
            )
        })?;
    }
    fs::rename(staged_dir, target_dir).map_err(|err| {
        format!(
            "Failed to restore retained folder {}: {err}",
            original_base_relative.display()
        )
    })?;
    record_directory_files(
        target_dir,
        original_base_relative,
        final_base_relative,
        disposition,
        source_root,
        report,
    )
}

fn merge_file(
    staged_file: &Path,
    target_file: &Path,
    source_root: &Path,
    original_relative: &Path,
    stamp: &str,
    report: &mut RetainedRestoreMergeReport,
) -> Result<(), String> {
    if !target_file.exists() {
        move_file_to_path(
            staged_file,
            target_file,
            original_relative,
            original_relative,
            RestoredFileDisposition::RestoredCanonical,
            report,
        )?;
        return Ok(());
    }
    if !target_file.is_file() {
        report.had_conflicts = true;
        let fallback = timestamped_conflict_path(target_file, "recovered", stamp);
        let final_relative = source_relative(source_root, &fallback)?;
        move_file_to_path(
            staged_file,
            &fallback,
            original_relative,
            &final_relative,
            RestoredFileDisposition::RestoredTimestamped,
            report,
        )?;
        return Ok(());
    }
    if files_match(staged_file, target_file)? {
        fs::remove_file(staged_file)
            .map_err(|err| format!("Failed to discard duplicate staged file: {err}"))?;
        report.restored_files.push(RestoredFileRecord {
            original_relative: original_relative.to_path_buf(),
            final_relative: original_relative.to_path_buf(),
            disposition: RestoredFileDisposition::ReusedExisting,
        });
        return Ok(());
    }
    report.had_conflicts = true;
    resolve_different_files(
        staged_file,
        target_file,
        source_root,
        original_relative,
        stamp,
        report,
    )
}

fn resolve_different_files(
    staged_file: &Path,
    target_file: &Path,
    source_root: &Path,
    original_relative: &Path,
    stamp: &str,
    report: &mut RetainedRestoreMergeReport,
) -> Result<(), String> {
    let staged_modified = modified_nanos(staged_file)?;
    let target_modified = modified_nanos(target_file)?;
    if staged_modified > target_modified {
        let backup = timestamped_conflict_path(target_file, "replaced", stamp);
        let relocated_relative = source_relative(source_root, &backup)?;
        move_existing_file_aside(
            target_file,
            &backup,
            original_relative,
            &relocated_relative,
            report,
        )?;
        move_file_to_path(
            staged_file,
            target_file,
            original_relative,
            original_relative,
            RestoredFileDisposition::RestoredCanonical,
            report,
        )?;
        return Ok(());
    }
    let fallback = timestamped_conflict_path(target_file, "recovered", stamp);
    let final_relative = source_relative(source_root, &fallback)?;
    move_file_to_path(
        staged_file,
        &fallback,
        original_relative,
        &final_relative,
        RestoredFileDisposition::RestoredTimestamped,
        report,
    )
}

fn move_existing_file_aside(
    target_file: &Path,
    backup: &Path,
    original_relative: &Path,
    relocated_relative: &Path,
    report: &mut RetainedRestoreMergeReport,
) -> Result<(), String> {
    if let Some(parent) = backup.parent() {
        fs::create_dir_all(parent).map_err(|err| {
            format!(
                "Failed to prepare conflict backup {}: {err}",
                parent.display()
            )
        })?;
    }
    fs::rename(target_file, backup)
        .map_err(|err| format!("Failed to preserve newer conflict copy: {err}"))?;
    report.existing_relocations.push(ExistingFileRelocation {
        original_relative: original_relative.to_path_buf(),
        relocated_relative: relocated_relative.to_path_buf(),
    });
    Ok(())
}

fn move_file_to_path(
    staged_file: &Path,
    target_file: &Path,
    original_relative: &Path,
    final_relative: &Path,
    disposition: RestoredFileDisposition,
    report: &mut RetainedRestoreMergeReport,
) -> Result<(), String> {
    if let Some(parent) = target_file.parent() {
        fs::create_dir_all(parent).map_err(|err| {
            format!(
                "Failed to prepare restore destination {}: {err}",
                parent.display()
            )
        })?;
    }
    fs::rename(staged_file, target_file).map_err(|err| {
        format!(
            "Failed to restore retained file {}: {err}",
            original_relative.display()
        )
    })?;
    report.restored_files.push(RestoredFileRecord {
        original_relative: original_relative.to_path_buf(),
        final_relative: final_relative.to_path_buf(),
        disposition,
    });
    Ok(())
}

fn move_non_file_conflict(
    staged_path: &Path,
    target_path: &Path,
    source_root: &Path,
    stamp: &str,
    report: &mut RetainedRestoreMergeReport,
) -> Result<(), String> {
    if !target_path.exists() {
        if let Some(parent) = target_path.parent() {
            fs::create_dir_all(parent).map_err(|err| {
                format!(
                    "Failed to prepare restore destination {}: {err}",
                    parent.display()
                )
            })?;
        }
        fs::rename(staged_path, target_path).map_err(|err| {
            format!(
                "Failed to restore retained entry {}: {err}",
                staged_path.display()
            )
        })?;
        return Ok(());
    }
    report.had_conflicts = true;
    let fallback = timestamped_conflict_path(target_path, "recovered", stamp);
    if let Some(parent) = fallback.parent() {
        fs::create_dir_all(parent).map_err(|err| {
            format!(
                "Failed to prepare restore destination {}: {err}",
                parent.display()
            )
        })?;
    }
    fs::rename(staged_path, &fallback).map_err(|err| {
        format!(
            "Failed to preserve retained conflict {}: {err}",
            staged_path.display()
        )
    })?;
    let _ = source_relative(source_root, &fallback)?;
    Ok(())
}

fn record_directory_files(
    final_dir: &Path,
    original_base_relative: &Path,
    _final_base_relative: &Path,
    disposition: RestoredFileDisposition,
    source_root: &Path,
    report: &mut RetainedRestoreMergeReport,
) -> Result<(), String> {
    for final_child in read_dir_paths(final_dir)? {
        let name = final_child
            .file_name()
            .ok_or_else(|| format!("Restored path missing file name: {}", final_child.display()))?;
        let original_child = original_base_relative.join(name);
        let final_relative = source_relative(source_root, &final_child)?;
        if final_child.is_dir() {
            record_directory_files(
                &final_child,
                &original_child,
                &final_relative,
                disposition,
                source_root,
                report,
            )?;
        } else if final_child.is_file() {
            report.restored_files.push(RestoredFileRecord {
                original_relative: original_child,
                final_relative,
                disposition,
            });
        }
    }
    Ok(())
}
