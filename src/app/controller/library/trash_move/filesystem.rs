use std::{
    fs,
    path::{Path, PathBuf},
};

use super::super::{SampleSource, WavEntry};

fn unique_destination(root: &Path, relative: &Path) -> Result<PathBuf, String> {
    let mut candidate = root.join(relative);
    if !candidate.exists() {
        return Ok(candidate);
    }
    let parent = candidate
        .parent()
        .map(Path::to_path_buf)
        .unwrap_or_else(|| root.to_path_buf());
    let stem = relative
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("file");
    let ext = relative.extension().and_then(|e| e.to_str()).unwrap_or("");
    for idx in 1..=1000 {
        let mut name = format!("{stem}_{idx}");
        if !ext.is_empty() {
            name.push('.');
            name.push_str(ext);
        }
        candidate = parent.join(name);
        if !candidate.exists() {
            return Ok(candidate);
        }
    }
    Err("Could not create unique trash destination".into())
}

pub(crate) fn move_to_trash(
    source: &SampleSource,
    entry: &WavEntry,
    trash_root: &Path,
) -> Result<(), String> {
    let absolute = source.root.join(&entry.relative_path);
    if !absolute.is_file() {
        return Err(format!("File not found for trash: {}", absolute.display()));
    }
    let destination = unique_destination(trash_root, &entry.relative_path)?;
    if let Some(parent) = destination.parent() {
        fs::create_dir_all(parent)
            .map_err(|err| format!("Unable to prepare trash folder {}: {err}", parent.display()))?;
    }
    if let Err(err) = fs::rename(&absolute, &destination) {
        fs::copy(&absolute, &destination).map_err(|copy_err| {
            format!(
                "Failed to move {} to trash: rename error {err}; copy error {copy_err}",
                absolute.display()
            )
        })?;
        fs::remove_file(&absolute).map_err(|remove_err| {
            format!(
                "Failed to remove original {} after copy: {remove_err}",
                absolute.display()
            )
        })?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn unique_destination_preserves_relative_folder_and_numbers_conflicts() {
        let temp = tempdir().unwrap();
        let root = temp.path().join("trash");
        fs::create_dir_all(root.join("drums")).unwrap();
        fs::write(root.join("drums").join("kick.wav"), b"old").unwrap();
        fs::write(root.join("drums").join("kick_1.wav"), b"old").unwrap();

        let destination = unique_destination(&root, Path::new("drums/kick.wav")).unwrap();

        assert_eq!(destination, root.join("drums").join("kick_2.wav"));
    }
}
