use std::path::{Path, PathBuf};

/// Validate and sanitize a renamed file while preserving its extension.
pub(crate) fn name_with_preserved_extension(
    current_relative: &Path,
    new_name: &str,
) -> Result<String, String> {
    let trimmed = new_name.trim();
    if trimmed.is_empty() {
        return Err("Name cannot be empty".into());
    }
    let Some(ext) = current_relative.extension().and_then(|ext| ext.to_str()) else {
        return Ok(trimmed.to_string());
    };
    let ext_lower = ext.to_ascii_lowercase();
    let should_strip_suffix = |suffix: &str| -> bool {
        let suffix_lower = suffix.to_ascii_lowercase();
        suffix_lower == ext_lower
            || matches!(
                suffix_lower.as_str(),
                "wav" | "wave" | "flac" | "aif" | "aiff" | "mp3" | "ogg" | "opus"
            )
    };
    let stem = if let Some((stem, suffix)) = trimmed.rsplit_once('.') {
        if !stem.is_empty() && should_strip_suffix(suffix) {
            stem
        } else {
            trimmed
        }
    } else {
        trimmed
    };
    let stem = stem.trim_end_matches('.');
    if stem.trim().is_empty() {
        return Err("Name cannot be empty".into());
    }
    Ok(format!("{stem}.{ext}"))
}

/// Validate that a new file name is safe and available in its parent folder.
pub(crate) fn validate_new_sample_name_in_parent(
    relative_path: &Path,
    root: &Path,
    new_name: &str,
) -> Result<PathBuf, String> {
    let trimmed = new_name.trim();
    if trimmed.is_empty() {
        return Err("Name cannot be empty".into());
    }
    if trimmed.contains(['/', '\\']) {
        return Err("Name cannot contain path separators".into());
    }
    let parent = relative_path.parent().unwrap_or(Path::new(""));
    let new_relative = parent.join(trimmed);
    let new_absolute = root.join(&new_relative);
    if new_absolute.exists() {
        return Err(format!(
            "A file named {} already exists",
            new_relative.display()
        ));
    }
    Ok(new_relative)
}

/// Validate that a moved file name is safe and available in the target folder.
pub(crate) fn validate_new_sample_name_in_folder(
    current_relative: &Path,
    root: &Path,
    target_folder: &Path,
    new_name: &str,
) -> Result<PathBuf, String> {
    let trimmed = new_name.trim();
    if trimmed.is_empty() {
        return Err("Name cannot be empty".into());
    }
    if trimmed.contains(['/', '\\']) {
        return Err("Name cannot contain path separators".into());
    }
    let file_name = name_with_preserved_extension(current_relative, trimmed)?;
    let new_relative = folder_relative_path(target_folder, &file_name);
    if root.join(&new_relative).exists() && new_relative != current_relative {
        return Err(format!(
            "A file named {} already exists",
            new_relative.display()
        ));
    }
    Ok(new_relative)
}

/// Suggest the next available numbered name for moving a file into a target folder.
pub(crate) fn suggest_numbered_sample_name_in_folder(
    current_relative: &Path,
    root: &Path,
    target_folder: &Path,
) -> Result<String, String> {
    let stem = current_relative
        .file_stem()
        .and_then(|name| name.to_str())
        .or_else(|| current_relative.file_name().and_then(|name| name.to_str()))
        .unwrap_or("sample");
    for index in 1..=999 {
        let candidate = format!("{stem}_{index:03}");
        let file_name = name_with_preserved_extension(current_relative, &candidate)?;
        let target_relative = folder_relative_path(target_folder, &file_name);
        if !root.join(target_relative).exists() {
            return Ok(candidate);
        }
    }
    Err("Unable to find a unique destination name".into())
}

fn folder_relative_path(target_folder: &Path, file_name: &str) -> PathBuf {
    if target_folder.as_os_str().is_empty() {
        PathBuf::from(file_name)
    } else {
        target_folder.join(file_name)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::tempdir;

    #[test]
    fn numbered_folder_move_name_starts_at_001() {
        let temp = tempdir().unwrap();
        let root = temp.path();
        fs::create_dir_all(root.join("dest")).unwrap();
        fs::write(root.join("dest/one.wav"), b"taken").unwrap();

        let suggested =
            suggest_numbered_sample_name_in_folder(Path::new("one.wav"), root, Path::new("dest"))
                .unwrap();

        assert_eq!(suggested, "one_001");
    }

    #[test]
    fn numbered_folder_move_name_skips_taken_suffixes() {
        let temp = tempdir().unwrap();
        let root = temp.path();
        fs::create_dir_all(root.join("dest")).unwrap();
        fs::write(root.join("dest/one.wav"), b"taken").unwrap();
        fs::write(root.join("dest/one_001.wav"), b"taken").unwrap();

        let suggested =
            suggest_numbered_sample_name_in_folder(Path::new("one.wav"), root, Path::new("dest"))
                .unwrap();

        assert_eq!(suggested, "one_002");
    }

    #[test]
    fn folder_move_name_validation_preserves_extension() {
        let temp = tempdir().unwrap();
        let root = temp.path();
        fs::create_dir_all(root.join("dest")).unwrap();

        let validated = validate_new_sample_name_in_folder(
            Path::new("one.wav"),
            root,
            Path::new("dest"),
            "one_001",
        )
        .unwrap();

        assert_eq!(validated, PathBuf::from("dest/one_001.wav"));
    }

    #[test]
    fn numbered_folder_move_name_handles_extensionless_files() {
        let temp = tempdir().unwrap();
        let root = temp.path();
        fs::create_dir_all(root.join("dest")).unwrap();
        fs::write(root.join("dest/loop"), b"taken").unwrap();

        let suggested =
            suggest_numbered_sample_name_in_folder(Path::new("loop"), root, Path::new("dest"))
                .unwrap();

        assert_eq!(suggested, "loop_001");
    }
}
