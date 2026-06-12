use notify::{RecommendedWatcher, RecursiveMode, Watcher};
use std::{collections::HashSet, path::PathBuf};
use wavecrate::sample_sources::SampleSource;

pub(super) fn update_watched_roots(
    watcher: &mut RecommendedWatcher,
    watched_roots: &mut HashSet<PathBuf>,
    sources: &[SampleSource],
) {
    let desired = sources
        .iter()
        .map(|source| source.root.clone())
        .filter(|root| root.is_dir())
        .collect::<HashSet<_>>();

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
            continue;
        }
        watched_roots.insert(root);
    }
}
