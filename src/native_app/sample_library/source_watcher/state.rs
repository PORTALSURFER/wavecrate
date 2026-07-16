use notify::{Event, RecommendedWatcher};
use std::{
    collections::{HashMap, HashSet},
    path::PathBuf,
    time::{Duration, Instant},
};
use wavecrate::sample_sources::SampleSource;

use super::classification::path_is_source_refresh_candidate;
use super::debounce::{GuiSourceWatchEvent, PendingGuiSourceWatch};
use super::path_mapping::{source_for_path, source_relative_path};
use super::roots::update_watched_roots;

#[derive(Default)]
pub(super) struct GuiSourceWatchState {
    pub(super) watched_roots: HashSet<PathBuf>,
    pub(super) sources: Vec<SampleSource>,
    pub(super) pending: HashMap<String, PendingGuiSourceWatch>,
}

impl GuiSourceWatchState {
    pub(super) fn set_sources(&mut self, sources: Vec<SampleSource>) {
        self.sources = sources;
        let allowed = self
            .sources
            .iter()
            .map(|source| source.id.as_str().to_string())
            .collect::<HashSet<_>>();
        self.pending
            .retain(|source_id, _| allowed.contains(source_id));
    }

    pub(super) fn refresh_watched_roots(
        &mut self,
        watcher: &mut RecommendedWatcher,
        now: Instant,
    ) -> (bool, bool) {
        let update = update_watched_roots(watcher, &mut self.watched_roots, &self.sources);
        for root in update.changed_roots {
            let affected = self
                .sources
                .iter()
                .filter(|source| source.root == root)
                .map(|source| source.id.as_str().to_string())
                .collect::<Vec<_>>();
            for source_id in affected {
                self.mark_source_overflowed(&source_id, now);
            }
        }
        (update.has_unavailable_roots, update.watch_failed)
    }

    pub(super) fn reset_watches(&mut self, now: Instant) {
        self.watched_roots.clear();
        self.mark_all_overflowed(now);
    }

    pub(super) fn mark_all_overflowed(&mut self, now: Instant) {
        let source_ids = self
            .sources
            .iter()
            .map(|source| source.id.as_str().to_string())
            .collect::<Vec<_>>();
        for source_id in source_ids {
            self.mark_source_overflowed(&source_id, now);
        }
    }

    fn mark_source_overflowed(&mut self, source_id: &str, now: Instant) {
        self.pending
            .entry(source_id.to_string())
            .and_modify(|pending| {
                pending.last_event = now;
                pending.overflowed = true;
                pending.paths.clear();
            })
            .or_insert_with(|| PendingGuiSourceWatch::new(now, None));
    }

    pub(super) fn collect_event(&mut self, event: &Event, now: Instant) {
        for path in &event.paths {
            if !path_is_source_refresh_candidate(path, event.kind) {
                continue;
            }
            if let Some(source) = source_for_path(&self.sources, path) {
                // FSEvents may coalesce writes to `.wavecrate.db`, its WAL, or related source
                // metadata into an event for the watched root itself. Re-scanning that live root
                // would write the database again and create a self-sustaining watcher loop.
                // Root disappearance/reappearance is observed independently by the periodic root
                // refresh; child paths still retain normal low-latency watcher behavior.
                if path == &source.root && source.root.is_dir() {
                    tracing::debug!(
                        source_id = source.id.as_str(),
                        kind = ?event.kind,
                        "Ignoring coalesced live-root watcher event"
                    );
                    continue;
                }
                self.pending
                    .entry(source.id.as_str().to_string())
                    .and_modify(|pending| {
                        pending.last_event = now;
                        pending.add_path(source_relative_path(source, path));
                    })
                    .or_insert_with(|| {
                        PendingGuiSourceWatch::new(now, source_relative_path(source, path))
                    });
            }
        }
    }

    pub(super) fn drain_ready_sources(
        &mut self,
        now: Instant,
        debounce: Duration,
    ) -> Vec<GuiSourceWatchEvent> {
        let ready = self
            .pending
            .iter()
            .filter(|&(_source_id, pending)| {
                now.saturating_duration_since(pending.last_event) >= debounce
            })
            .filter_map(|(source_id, pending)| {
                let source = self
                    .sources
                    .iter()
                    .find(|source| source.id.as_str() == source_id)?;
                Some(GuiSourceWatchEvent {
                    source_id: source_id.clone(),
                    paths: pending.paths.iter().cloned().collect(),
                    overflowed: pending.overflowed,
                    source_root_available: source.root.is_dir(),
                })
            })
            .collect::<Vec<_>>();
        for event in &ready {
            self.pending.remove(&event.source_id);
        }
        ready
    }
}
