//! Filesystem-side retained-restore merge operations.

use super::util::{
    files_match, modified_nanos, read_dir_paths, remove_dir_if_empty, source_relative,
    timestamped_conflict_path,
};
use super::{RestoredFileDisposition, RetainedRestoreMergeReport, filesystem, reporting};
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
    _final_base_relative: &Path,
    disposition: RestoredFileDisposition,
    report: &mut RetainedRestoreMergeReport,
) -> Result<(), String> {
    filesystem::restore_retained_folder(staged_dir, target_dir, original_base_relative)?;
    reporting::record_directory_files(
        report,
        target_dir,
        original_base_relative,
        disposition,
        source_root,
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
        filesystem::discard_duplicate_staged_file(staged_file)?;
        reporting::record_restored_file(
            report,
            original_relative,
            original_relative,
            RestoredFileDisposition::ReusedExisting,
        );
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
    filesystem::preserve_existing_conflict_copy(target_file, backup)?;
    reporting::record_existing_relocation(report, original_relative, relocated_relative);
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
    filesystem::restore_retained_file(staged_file, target_file, original_relative)?;
    reporting::record_restored_file(report, original_relative, final_relative, disposition);
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
        filesystem::restore_retained_entry(staged_path, target_path)?;
        return Ok(());
    }
    report.had_conflicts = true;
    let fallback = timestamped_conflict_path(target_path, "recovered", stamp);
    filesystem::preserve_retained_conflict(staged_path, &fallback)?;
    let _ = source_relative(source_root, &fallback)?;
    Ok(())
}
