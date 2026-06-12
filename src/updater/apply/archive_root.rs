//! Update archive root validation.

use std::path::{Path, PathBuf};

use super::{UpdateError, fs_ops};

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
    let root = dirs.pop().expect("single archive root directory");
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
