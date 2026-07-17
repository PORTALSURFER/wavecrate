use std::path::{Path, PathBuf};

const MAX_WATCHER_ECHO_HASH_BYTES: u64 = 8 * 1024 * 1024;

#[derive(Clone, Debug, PartialEq, Eq)]
pub(in crate::native_app) enum CommittedWatcherPathState {
    Missing,
    ContentHash([u8; 32]),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(in crate::native_app) struct CommittedWatcherEcho {
    pub(in crate::native_app) relative_path: PathBuf,
    pub(in crate::native_app) expected_state: CommittedWatcherPathState,
}

pub(super) fn capture_watcher_echoes(
    root: &Path,
    relative_paths: &[PathBuf],
) -> Vec<CommittedWatcherEcho> {
    relative_paths
        .iter()
        .filter_map(|relative_path| {
            observed_watcher_path_state(&root.join(relative_path)).map(|expected_state| {
                CommittedWatcherEcho {
                    relative_path: relative_path.clone(),
                    expected_state,
                }
            })
        })
        .collect()
}

pub(in crate::native_app) fn observed_watcher_path_state(
    path: &Path,
) -> Option<CommittedWatcherPathState> {
    match std::fs::metadata(path) {
        Ok(metadata) if metadata.is_file() && metadata.len() <= MAX_WATCHER_ECHO_HASH_BYTES => {
            let bytes = std::fs::read(path).ok()?;
            Some(CommittedWatcherPathState::ContentHash(
                *blake3::hash(&bytes).as_bytes(),
            ))
        }
        Ok(_) => None,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            Some(CommittedWatcherPathState::Missing)
        }
        Err(_) => None,
    }
}
