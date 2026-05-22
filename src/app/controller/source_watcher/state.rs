use super::{
    SourceId, SourceWatchCause, SourceWatchCommand, SourceWatchEntry, SourceWatchEvent,
    combine_source_watch_causes, path_is_candidate, select_source_entry_for_path,
    source_watch_cause_for_path, update_watched_sources,
};
use notify::{Event, RecommendedWatcher};
use std::{
    collections::{HashMap, HashSet},
    path::PathBuf,
    time::{Duration, Instant},
};

#[derive(Default)]
pub(super) struct SourceWatcherState {
    watched_roots: HashSet<PathBuf>,
    sources: Vec<SourceWatchEntry>,
    pub(super) pending: HashMap<SourceId, PendingSourceWatch>,
    controller_file_ops: HashMap<SourceId, HashSet<PathBuf>>,
    pub(super) scan_in_progress: bool,
}

impl SourceWatcherState {
    pub(super) fn handle_command(
        &mut self,
        command: SourceWatchCommand,
        watcher: &mut RecommendedWatcher,
    ) -> bool {
        match command {
            SourceWatchCommand::ReplaceSources(next_sources) => {
                update_watched_sources(watcher, &mut self.watched_roots, &next_sources);
                self.sources = next_sources;
                self.prune_pending_sources();
            }
            SourceWatchCommand::SetScanInProgress { in_progress } => {
                self.scan_in_progress = in_progress;
            }
            SourceWatchCommand::BeginControllerFileOp {
                source_id,
                relative_paths,
            } => {
                self.controller_file_ops
                    .entry(source_id)
                    .or_default()
                    .extend(relative_paths);
            }
            SourceWatchCommand::FinishControllerFileOp {
                source_id,
                relative_paths,
            } => self.finish_controller_file_op(source_id, relative_paths),
            SourceWatchCommand::Shutdown => return false,
        }
        true
    }

    fn finish_controller_file_op(&mut self, source_id: SourceId, relative_paths: Vec<PathBuf>) {
        if let Some(paths) = self.controller_file_ops.get_mut(&source_id) {
            for path in relative_paths {
                paths.remove(&path);
            }
            if paths.is_empty() {
                self.controller_file_ops.remove(&source_id);
            }
        }
    }

    pub(super) fn collect_event(&mut self, event: Event, now: Instant) {
        let mut impacted = HashMap::new();
        for path in &event.paths {
            if !path_is_candidate(path) {
                continue;
            }
            if let Some(source) = select_source_entry_for_path(&self.sources, path) {
                let cause = source_watch_cause_for_path(&self.controller_file_ops, source, path);
                impacted
                    .entry(source.source_id.clone())
                    .and_modify(|pending_cause| {
                        *pending_cause = combine_source_watch_causes(*pending_cause, cause);
                    })
                    .or_insert(cause);
            }
        }
        for (source_id, cause) in impacted {
            self.update_pending_watch(source_id, cause, now);
        }
    }

    pub(super) fn update_pending_watch(
        &mut self,
        source_id: SourceId,
        cause: SourceWatchCause,
        now: Instant,
    ) {
        self.pending
            .entry(source_id)
            .and_modify(|entry| {
                entry.last_event = now;
                entry.cause = combine_source_watch_causes(entry.cause, cause);
            })
            .or_insert(PendingSourceWatch {
                last_event: now,
                cause,
            });
    }

    pub(super) fn drain_ready_sources(
        &mut self,
        now: Instant,
        debounce: Duration,
    ) -> Vec<SourceWatchEvent> {
        if self.scan_in_progress {
            return Vec::new();
        }
        let ready: Vec<SourceWatchEvent> = self
            .pending
            .iter()
            .filter(|&(_source_id, entry)| {
                now.saturating_duration_since(entry.last_event) >= debounce
            })
            .map(|(source_id, entry)| SourceWatchEvent {
                source_id: source_id.clone(),
                cause: entry.cause,
            })
            .collect();
        for event in &ready {
            self.pending.remove(&event.source_id);
        }
        ready
    }

    fn prune_pending_sources(&mut self) {
        let allowed: HashSet<&SourceId> =
            self.sources.iter().map(|entry| &entry.source_id).collect();
        self.pending
            .retain(|source_id, _| allowed.contains(source_id));
    }
}

#[derive(Debug, Copy, Clone)]
pub(super) struct PendingSourceWatch {
    last_event: Instant,
    cause: SourceWatchCause,
}
