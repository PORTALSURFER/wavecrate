use std::{
    fs::{self, File, OpenOptions},
    io,
    path::{Path, PathBuf},
};

#[derive(Clone, Debug, PartialEq)]
pub(in crate::native_app) struct TrashMoveOutcome {
    pub source: PathBuf,
    pub result: TrashMoveResult,
}

#[derive(Clone, Debug, PartialEq)]
pub(in crate::native_app) enum TrashMoveResult {
    Moved { destination: PathBuf },
    Missing,
    Failed { error: String },
}

pub(super) fn move_paths_to_configured_trash(
    paths: &[PathBuf],
    trash_folder: Option<&Path>,
) -> Vec<TrashMoveOutcome> {
    paths
        .iter()
        .map(|path| move_path_to_configured_trash(path, trash_folder))
        .collect()
}

pub(super) fn move_path_to_configured_trash(
    path: &Path,
    trash_folder: Option<&Path>,
) -> TrashMoveOutcome {
    let source_path = path.to_path_buf();
    let result = move_path_to_configured_trash_inner(path, trash_folder);
    TrashMoveOutcome {
        source: source_path,
        result,
    }
}

fn move_path_to_configured_trash_inner(
    path: &Path,
    trash_folder: Option<&Path>,
) -> TrashMoveResult {
    match path.try_exists() {
        Ok(true) => {}
        Ok(false) => return TrashMoveResult::Missing,
        Err(error) => {
            return TrashMoveResult::Failed {
                error: format!("Trash source is unavailable: {error}"),
            };
        }
    }
    let moved = (|| {
        let trash_folder = trash_folder.ok_or_else(|| {
            String::from("Set a trash folder in Settings > General before deleting files")
        })?;
        fs::create_dir_all(trash_folder)
            .map_err(|err| format!("Create trash folder failed: {err}"))?;
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
        let reservation = reserve_trash_path(&trash_folder, &source)?;
        move_path(&source, reservation.destination())?;
        Ok(reservation.destination().to_path_buf())
    })();
    match moved {
        Ok(destination) => TrashMoveResult::Moved { destination },
        Err(error) => TrashMoveResult::Failed { error },
    }
}

struct TrashDestinationReservation {
    destination: PathBuf,
    marker: PathBuf,
    _file: File,
}

impl TrashDestinationReservation {
    fn destination(&self) -> &Path {
        &self.destination
    }
}

impl Drop for TrashDestinationReservation {
    fn drop(&mut self) {
        let _ = fs::remove_file(&self.marker);
    }
}

fn reserve_trash_path(
    trash_folder: &Path,
    source: &Path,
) -> Result<TrashDestinationReservation, String> {
    let file_name = source
        .file_name()
        .ok_or_else(|| format!("Trash move failed: {} has no file name", source.display()))?;
    let stem = source
        .file_stem()
        .map(|stem| stem.to_string_lossy().to_string())
        .unwrap_or_else(|| file_name.to_string_lossy().to_string());
    let extension = source.extension().map(|extension| extension.to_os_string());
    for index in 1..10_000 {
        let name = if index == 1 {
            file_name.to_os_string()
        } else {
            let mut name = format!("{stem} {index}");
            if let Some(extension) = &extension {
                name.push('.');
                name.push_str(&extension.to_string_lossy());
            }
            name.into()
        };
        let candidate = trash_folder.join(name);
        if candidate.exists() {
            continue;
        }
        let marker = candidate.with_extension(format!(
            "{}wavecrate-trash-reservation",
            candidate
                .extension()
                .map(|value| format!("{}.", value.to_string_lossy()))
                .unwrap_or_default()
        ));
        match OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&marker)
        {
            Ok(file) => {
                if candidate.exists() {
                    drop(file);
                    let _ = fs::remove_file(marker);
                    continue;
                }
                return Ok(TrashDestinationReservation {
                    destination: candidate,
                    marker,
                    _file: file,
                });
            }
            Err(error) if error.kind() == io::ErrorKind::AlreadyExists => continue,
            Err(error) => return Err(format!("Reserve trash destination failed: {error}")),
        }
    }
    Err(String::from(
        "Trash folder contains too many matching names",
    ))
}

fn move_path(source: &Path, destination: &Path) -> Result<(), String> {
    match fs::rename(source, destination) {
        Ok(()) => Ok(()),
        Err(rename_error) => fallback_move_path(source, destination, &rename_error),
    }
}

fn fallback_move_path(
    source: &Path,
    destination: &Path,
    rename_error: &io::Error,
) -> Result<(), String> {
    let is_directory = source.is_dir();
    let copy_result = if is_directory {
        copy_dir_all(source, destination)
    } else {
        copy_file_exclusive(source, destination)
    };
    if let Err(error) = copy_result {
        remove_partial_destination(destination);
        let kind = if is_directory { "folder" } else { "file" };
        return Err(format!(
            "Move {kind} to trash failed: {rename_error}; fallback failed: {error}"
        ));
    }
    let cleanup_result = if is_directory {
        fs::remove_dir_all(source)
    } else {
        fs::remove_file(source)
    };
    if let Err(error) = cleanup_result {
        let kind = if is_directory { "folder" } else { "file" };
        return Err(format!(
            "Move {kind} to trash copied successfully, but source cleanup failed: {error}; the trash copy was preserved"
        ));
    }
    Ok(())
}

fn copy_dir_all(source: &Path, destination: &Path) -> std::io::Result<()> {
    fs::create_dir(destination)?;
    for entry in fs::read_dir(source)? {
        let entry = entry?;
        let target = destination.join(entry.file_name());
        if entry.file_type()?.is_dir() {
            copy_dir_all(&entry.path(), &target)?;
        } else {
            copy_file_exclusive(&entry.path(), &target)?;
        }
    }
    Ok(())
}

fn copy_file_exclusive(source: &Path, destination: &Path) -> io::Result<()> {
    let mut input = File::open(source)?;
    let mut output = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(destination)?;
    io::copy(&mut input, &mut output)?;
    output.sync_all()
}

fn remove_partial_destination(destination: &Path) {
    if destination.is_dir() {
        let _ = fs::remove_dir_all(destination);
    } else {
        let _ = fs::remove_file(destination);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::{Arc, Barrier};
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

        let destination = reserve_trash_path(&trash, &source).unwrap();

        assert_eq!(destination.destination(), trash.join("kick 3.wav"));
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

    #[test]
    fn batch_retains_successes_when_a_later_source_is_missing() {
        let temp = tempdir().unwrap();
        let trash = temp.path().join("trash");
        let source_root = temp.path().join("source");
        fs::create_dir_all(&source_root).unwrap();
        let first = source_root.join("first.wav");
        let missing = source_root.join("missing.wav");
        fs::write(&first, b"first").unwrap();

        let outcomes =
            move_paths_to_configured_trash(&[first.clone(), missing.clone()], Some(&trash));

        assert_eq!(outcomes.len(), 2);
        assert!(matches!(outcomes[0].result, TrashMoveResult::Moved { .. }));
        assert_eq!(outcomes[1].result, TrashMoveResult::Missing);
        assert!(!first.exists());
        assert_eq!(fs::read(trash.join("first.wav")).unwrap(), b"first");
    }

    #[test]
    fn concurrent_same_name_trash_moves_preserve_both_payloads() {
        let temp = tempdir().unwrap();
        let trash = temp.path().join("trash");
        let left_root = temp.path().join("left");
        let right_root = temp.path().join("right");
        fs::create_dir_all(&left_root).unwrap();
        fs::create_dir_all(&right_root).unwrap();
        let left = left_root.join("kick.wav");
        let right = right_root.join("kick.wav");
        fs::write(&left, b"left").unwrap();
        fs::write(&right, b"right").unwrap();
        let barrier = Arc::new(Barrier::new(2));

        let handles = [left, right].map(|source| {
            let barrier = Arc::clone(&barrier);
            let trash = trash.clone();
            std::thread::spawn(move || {
                barrier.wait();
                move_path_to_configured_trash(&source, Some(&trash))
            })
        });
        let outcomes = handles.map(|handle| handle.join().unwrap());

        assert!(
            outcomes
                .iter()
                .all(|outcome| matches!(outcome.result, TrashMoveResult::Moved { .. }))
        );
        let mut payloads = fs::read_dir(&trash)
            .unwrap()
            .map(|entry| fs::read(entry.unwrap().path()).unwrap())
            .collect::<Vec<_>>();
        payloads.sort();
        assert_eq!(payloads, vec![b"left".to_vec(), b"right".to_vec()]);
    }

    #[cfg(unix)]
    #[test]
    fn failed_fallback_copy_removes_partial_destination() {
        use std::os::unix::fs::symlink;

        let temp = tempdir().unwrap();
        let source = temp.path().join("source");
        let destination = temp.path().join("destination");
        fs::create_dir(&source).unwrap();
        fs::write(source.join("copied.wav"), b"copied").unwrap();
        symlink(source.join("missing.wav"), source.join("broken.wav")).unwrap();

        let error = fallback_move_path(
            &source,
            &destination,
            &io::Error::from(io::ErrorKind::CrossesDevices),
        )
        .unwrap_err();

        assert!(error.contains("fallback failed"));
        assert!(source.exists());
        assert!(!destination.exists());
    }
}
