use std::{
    fs,
    path::{Path, PathBuf},
};

pub(super) fn move_paths_to_configured_trash(
    paths: &[PathBuf],
    trash_folder: Option<&Path>,
) -> Result<Vec<PathBuf>, String> {
    let mut moved = Vec::with_capacity(paths.len());
    for path in paths {
        moved.push(move_path_to_configured_trash(path, trash_folder)?);
    }
    Ok(moved)
}

pub(super) fn move_path_to_configured_trash(
    path: &Path,
    trash_folder: Option<&Path>,
) -> Result<PathBuf, String> {
    let trash_folder = trash_folder.ok_or_else(|| {
        String::from("Set a trash folder in Settings > General before deleting files")
    })?;
    if !path.exists() {
        return Err(format!("Trash move failed: {} is missing", path.display()));
    }
    fs::create_dir_all(trash_folder).map_err(|err| format!("Create trash folder failed: {err}"))?;
    let trash_folder = trash_folder
        .canonicalize()
        .map_err(|err| format!("Trash folder is unavailable: {err}"))?;
    let source = path
        .canonicalize()
        .map_err(|err| format!("Trash source is unavailable: {err}"))?;
    if source.starts_with(&trash_folder) {
        return Err(String::from(
            "Selected item is already inside the trash folder",
        ));
    }
    if trash_folder.starts_with(&source) {
        return Err(String::from(
            "Trash folder cannot be inside the item being deleted",
        ));
    }
    let destination = next_available_trash_path(&trash_folder, &source)?;
    move_path(&source, &destination)?;
    Ok(destination)
}

fn next_available_trash_path(trash_folder: &Path, source: &Path) -> Result<PathBuf, String> {
    let file_name = source
        .file_name()
        .ok_or_else(|| format!("Trash move failed: {} has no file name", source.display()))?;
    let candidate = trash_folder.join(file_name);
    if !candidate.exists() {
        return Ok(candidate);
    }
    let stem = source
        .file_stem()
        .map(|stem| stem.to_string_lossy().to_string())
        .unwrap_or_else(|| file_name.to_string_lossy().to_string());
    let extension = source.extension().map(|extension| extension.to_os_string());
    for index in 2..10_000 {
        let mut name = format!("{stem} {index}");
        if let Some(extension) = &extension {
            name.push('.');
            name.push_str(&extension.to_string_lossy());
        }
        let candidate = trash_folder.join(name);
        if !candidate.exists() {
            return Ok(candidate);
        }
    }
    Err(String::from(
        "Trash folder contains too many matching names",
    ))
}

fn move_path(source: &Path, destination: &Path) -> Result<(), String> {
    match fs::rename(source, destination) {
        Ok(()) => Ok(()),
        Err(rename_error) => {
            if source.is_dir() {
                copy_dir_all(source, destination)
                    .and_then(|()| fs::remove_dir_all(source))
                    .map_err(|err| {
                        format!(
                            "Move folder to trash failed: {rename_error}; fallback failed: {err}"
                        )
                    })
            } else {
                fs::copy(source, destination)
                    .and_then(|_| fs::remove_file(source))
                    .map_err(|err| {
                        format!("Move file to trash failed: {rename_error}; fallback failed: {err}")
                    })
            }
        }
    }
}

fn copy_dir_all(source: &Path, destination: &Path) -> std::io::Result<()> {
    fs::create_dir_all(destination)?;
    for entry in fs::read_dir(source)? {
        let entry = entry?;
        let target = destination.join(entry.file_name());
        if entry.file_type()?.is_dir() {
            copy_dir_all(&entry.path(), &target)?;
        } else {
            fs::copy(entry.path(), target)?;
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn trash_destination_numbers_existing_names() {
        let temp = tempdir().unwrap();
        let trash = temp.path().join("trash");
        let source_root = temp.path().join("source");
        fs::create_dir_all(&trash).unwrap();
        fs::create_dir_all(&source_root).unwrap();
        let source = source_root.join("kick.wav");
        fs::write(&source, b"wav").unwrap();
        fs::write(trash.join("kick.wav"), b"old").unwrap();
        fs::write(trash.join("kick 2.wav"), b"old").unwrap();

        let destination = next_available_trash_path(&trash, &source).unwrap();

        assert_eq!(destination, trash.join("kick 3.wav"));
    }

    #[test]
    fn recursive_copy_preserves_nested_files() {
        let temp = tempdir().unwrap();
        let source = temp.path().join("source");
        let destination = temp.path().join("destination");
        fs::create_dir_all(source.join("nested")).unwrap();
        fs::write(source.join("root.wav"), b"root").unwrap();
        fs::write(source.join("nested").join("child.wav"), b"child").unwrap();

        copy_dir_all(&source, &destination).unwrap();

        assert_eq!(fs::read(destination.join("root.wav")).unwrap(), b"root");
        assert_eq!(
            fs::read(destination.join("nested").join("child.wav")).unwrap(),
            b"child"
        );
    }
}
