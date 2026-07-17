use notify::{RecommendedWatcher, RecursiveMode, Watcher};
use std::{collections::HashSet, path::PathBuf};
use wavecrate::sample_sources::SampleSource;

pub(super) use crate::native_app::sample_library::committed_file_mutations::observed_watcher_path_state;

pub(super) fn source_root_is_available(source: &SampleSource) -> bool {
    source.root.is_dir()
}

pub(super) struct RootWatchUpdate {
    pub(super) changed_roots: Vec<PathBuf>,
    pub(super) has_unavailable_roots: bool,
    pub(super) watch_failed: bool,
}

pub(super) fn update_watched_roots(
    watcher: &mut RecommendedWatcher,
    watched_roots: &mut HashSet<PathBuf>,
    sources: &[SampleSource],
) -> RootWatchUpdate {
    let desired = sources
        .iter()
        .map(|source| source.root.clone())
        .filter(|root| root.is_dir())
        .collect::<HashSet<_>>();

    let mut changed_roots = Vec::new();
    let mut watch_failed = false;
    for root in watched_roots
        .difference(&desired)
        .cloned()
        .collect::<Vec<_>>()
    {
        if let Err(error) = watcher.unwatch(&root) {
            tracing::warn!(
                "Failed to unwatch GUI source root {}: {error}",
                root.display()
            );
        }
        watched_roots.remove(&root);
        changed_roots.push(root);
    }

    for root in desired
        .difference(watched_roots)
        .cloned()
        .collect::<Vec<_>>()
    {
        if let Err(error) = watcher.watch(&root, RecursiveMode::Recursive) {
            tracing::warn!(
                "Failed to watch GUI source root {}: {error}",
                root.display()
            );
            changed_roots.push(root);
            watch_failed = true;
            continue;
        }
        watched_roots.insert(root.clone());
        changed_roots.push(root);
    }
    RootWatchUpdate {
        changed_roots,
        has_unavailable_roots: sources.iter().any(|source| !source.root.is_dir()),
        watch_failed,
    }
}
