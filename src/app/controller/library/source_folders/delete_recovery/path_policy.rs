//! Path validation for source-local delete recovery state.

use crate::sample_sources::normalize_relative_path;
use std::{
    fs, io,
    path::{Path, PathBuf},
};

pub(super) fn validate_relative_path(path: &Path, field: &str) -> Result<PathBuf, String> {
    normalize_relative_path(path)
        .map(PathBuf::from)
        .map_err(|err| format!("Invalid delete recovery {field} {}: {err}", path.display()))
}

pub(super) fn validate_journal_relative(raw: &str, field: &str) -> Result<PathBuf, String> {
    validate_relative_path(Path::new(raw), field)
}

pub(super) fn source_root_for_staging_root(staging_root: &Path) -> Result<PathBuf, String> {
    staging_root.parent().map(Path::to_path_buf).ok_or_else(|| {
        format!(
            "Delete staging root has no source parent: {}",
            staging_root.display()
        )
    })
}

pub(super) fn contained_child(
    root: &Path,
    relative: &Path,
    field: &str,
) -> Result<PathBuf, String> {
    Ok(root.join(validate_relative_path(relative, field)?))
}

pub(super) fn path_exists_no_follow(path: &Path) -> Result<bool, String> {
    match fs::symlink_metadata(path) {
        Ok(_) => Ok(true),
        Err(err) if err.kind() == io::ErrorKind::NotFound => Ok(false),
        Err(err) => Err(format!(
            "Failed to inspect delete recovery path {}: {err}",
            path.display()
        )),
    }
}

pub(super) fn ensure_existing_dir_under(
    root: &Path,
    path: &Path,
    context: &str,
) -> Result<(), String> {
    let metadata = fs::symlink_metadata(path)
        .map_err(|err| format!("Failed to inspect {context} {}: {err}", path.display()))?;
    if metadata.file_type().is_symlink() {
        return Err(format!(
            "{context} must not be a symlink: {}",
            path.display()
        ));
    }
    if !metadata.file_type().is_dir() {
        return Err(format!("{context} is not a directory: {}", path.display()));
    }
    ensure_existing_path_under(root, path, context)
}

pub(super) fn ensure_existing_path_under(
    root: &Path,
    path: &Path,
    context: &str,
) -> Result<(), String> {
    let canonical_root = canonicalize_root(root, context)?;
    let canonical_path = fs::canonicalize(path)
        .map_err(|err| format!("Failed to resolve {context} {}: {err}", path.display()))?;
    ensure_canonical_under(&canonical_root, &canonical_path, context, path)
}

pub(super) fn ensure_creatable_path_under(
    root: &Path,
    path: &Path,
    context: &str,
) -> Result<(), String> {
    let canonical_root = canonicalize_root(root, context)?;
    if path_exists_no_follow(path)? {
        reject_symlink(path, context)?;
        let canonical_path = fs::canonicalize(path)
            .map_err(|err| format!("Failed to resolve {context} {}: {err}", path.display()))?;
        return ensure_canonical_under(&canonical_root, &canonical_path, context, path);
    }
    let ancestor = nearest_existing_ancestor(path)?;
    reject_symlink(&ancestor, context)?;
    let canonical_ancestor = fs::canonicalize(&ancestor)
        .map_err(|err| format!("Failed to resolve {context} {}: {err}", ancestor.display()))?;
    ensure_canonical_under(&canonical_root, &canonical_ancestor, context, path)
}

pub(super) fn ensure_staging_root(source_root: &Path, staging_root: &Path) -> Result<(), String> {
    ensure_existing_dir_under(source_root, staging_root, "Delete staging root")
}

pub(super) fn reject_symlink(path: &Path, context: &str) -> Result<(), String> {
    let metadata = fs::symlink_metadata(path)
        .map_err(|err| format!("Failed to inspect {context} {}: {err}", path.display()))?;
    if metadata.file_type().is_symlink() {
        return Err(format!(
            "{context} must not be a symlink: {}",
            path.display()
        ));
    }
    Ok(())
}

fn canonicalize_root(root: &Path, context: &str) -> Result<PathBuf, String> {
    reject_symlink(root, context)?;
    fs::canonicalize(root)
        .map_err(|err| format!("Failed to resolve {context} root {}: {err}", root.display()))
}

fn nearest_existing_ancestor(path: &Path) -> Result<PathBuf, String> {
    let mut current = path.parent().unwrap_or_else(|| Path::new("."));
    loop {
        match fs::symlink_metadata(current) {
            Ok(_) => return Ok(current.to_path_buf()),
            Err(err) if err.kind() == io::ErrorKind::NotFound => {
                current = current.parent().ok_or_else(|| {
                    format!(
                        "No existing parent found for delete recovery path {}",
                        path.display()
                    )
                })?;
            }
            Err(err) => {
                return Err(format!(
                    "Failed to inspect delete recovery parent {}: {err}",
                    current.display()
                ));
            }
        }
    }
}

fn ensure_canonical_under(
    canonical_root: &Path,
    canonical_path: &Path,
    context: &str,
    original: &Path,
) -> Result<(), String> {
    if canonical_path.starts_with(canonical_root) {
        return Ok(());
    }
    Err(format!(
        "{context} escapes delete recovery root: {}",
        original.display()
    ))
}
