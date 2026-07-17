use std::path::{Path, PathBuf};

#[derive(Clone, Debug, PartialEq, Eq)]
pub(in crate::native_app) enum CommittedWatcherPathState {
    Missing,
    Metadata { len: u64, modified_ns: u128 },
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
        Ok(metadata) => {
            let modified_ns = metadata
                .modified()
                .ok()?
                .duration_since(std::time::UNIX_EPOCH)
                .ok()?
                .as_nanos();
            Some(CommittedWatcherPathState::Metadata {
                len: metadata.len(),
                modified_ns,
            })
        }
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            Some(CommittedWatcherPathState::Missing)
        }
        Err(_) => None,
    }
}
