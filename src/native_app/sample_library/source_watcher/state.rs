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
    pub(super) fn replace_sources(
        &mut self,
        sources: Vec<SampleSource>,
        watcher: &mut RecommendedWatcher,
    ) {
        update_watched_roots(watcher, &mut self.watched_roots, &sources);
        self.sources = sources;
        let allowed = self
            .sources
            .iter()
            .map(|source| source.id.as_str().to_string())
            .collect::<HashSet<_>>();
        self.pending
            .retain(|source_id, _| allowed.contains(source_id));
    }

    pub(super) fn collect_event(&mut self, event: &Event, now: Instant) {
        for path in &event.paths {
            if !path_is_source_refresh_candidate(path, event.kind) {
                continue;
            }
            if let Some(source) = source_for_path(&self.sources, path) {
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
            .map(|(source_id, pending)| GuiSourceWatchEvent {
                source_id: source_id.clone(),
                paths: pending.paths.iter().cloned().collect(),
                overflowed: pending.overflowed,
            })
            .collect::<Vec<_>>();
        for event in &ready {
            self.pending.remove(&event.source_id);
        }
        ready
    }
}
