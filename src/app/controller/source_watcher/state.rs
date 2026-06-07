use super::{
    SourceId, SourceWatchCause, SourceWatchCommand, SourceWatchEntry, SourceWatchEvent,
    combine_source_watch_causes, path_is_candidate, select_source_entry_for_path,
    source_watch_cause_for_path, update_watched_sources,
};
use notify::{Event, RecommendedWatcher};
use std::{
    collections::{BTreeSet, HashMap, HashSet},
    path::PathBuf,
    time::{Duration, Instant},
};

const MAX_PENDING_PATHS_PER_SOURCE: usize = 512;

#[derive(Default)]
pub(super) struct SourceWatcherState {
    pub(super) watched_roots: HashSet<PathBuf>,
    pub(super) sources: Vec<SourceWatchEntry>,
    pub(super) pending: HashMap<SourceId, PendingSourceWatch>,
    pub(super) controller_file_ops: HashMap<SourceId, HashSet<PathBuf>>,
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
        let mut impacted: HashMap<SourceId, PendingSourceEvent> = HashMap::new();
        for path in &event.paths {
            if !path_is_candidate(path, event.kind) {
                continue;
            }
            if let Some(source) = select_source_entry_for_path(&self.sources, path) {
                let cause = source_watch_cause_for_path(&self.controller_file_ops, source, path);
                impacted
                    .entry(source.source_id.clone())
                    .and_modify(|pending| {
                        pending.cause = combine_source_watch_causes(pending.cause, cause);
                        pending.add_path(source_relative_path(source, path));
                    })
                    .or_insert_with(|| {
                        PendingSourceEvent::new(cause, source_relative_path(source, path))
                    });
            }
        }
        for (source_id, event) in impacted {
            self.update_pending_watch(source_id, event, now);
        }
    }

    pub(super) fn update_pending_watch(
        &mut self,
        source_id: SourceId,
        event: PendingSourceEvent,
        now: Instant,
    ) {
        self.pending
            .entry(source_id)
            .and_modify(|entry| {
                entry.last_event = now;
                entry.cause = combine_source_watch_causes(entry.cause, event.cause);
                entry.merge_paths(event.paths.clone(), event.overflowed);
            })
            .or_insert(PendingSourceWatch {
                last_event: now,
                cause: event.cause,
                paths: event.paths,
                overflowed: event.overflowed,
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
                paths: entry.paths.iter().cloned().collect(),
                overflowed: entry.overflowed,
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

#[derive(Debug, Clone)]
pub(super) struct PendingSourceWatch {
    last_event: Instant,
    cause: SourceWatchCause,
    pub(super) paths: BTreeSet<PathBuf>,
    pub(super) overflowed: bool,
}

impl PendingSourceWatch {
    fn merge_paths(&mut self, paths: BTreeSet<PathBuf>, overflowed: bool) {
        if overflowed {
            self.overflowed = true;
            self.paths.clear();
            return;
        }
        if self.overflowed {
            return;
        }
        for path in paths {
            if self.paths.len() >= MAX_PENDING_PATHS_PER_SOURCE {
                self.overflowed = true;
                self.paths.clear();
                return;
            }
            self.paths.insert(path);
        }
    }
}

pub(super) struct PendingSourceEvent {
    cause: SourceWatchCause,
    paths: BTreeSet<PathBuf>,
    overflowed: bool,
}

impl PendingSourceEvent {
    pub(super) fn new(cause: SourceWatchCause, path: Option<PathBuf>) -> Self {
        let mut event = Self {
            cause,
            paths: BTreeSet::new(),
            overflowed: false,
        };
        event.add_path(path);
        event
    }

    fn add_path(&mut self, path: Option<PathBuf>) {
        let Some(path) = path else {
            self.overflowed = true;
            self.paths.clear();
            return;
        };
        if self.paths.len() >= MAX_PENDING_PATHS_PER_SOURCE {
            self.overflowed = true;
            self.paths.clear();
            return;
        }
        self.paths.insert(path);
    }
}

fn source_relative_path(source: &SourceWatchEntry, path: &std::path::Path) -> Option<PathBuf> {
    let relative = path.strip_prefix(&source.root).ok()?;
    (!relative.as_os_str().is_empty()).then(|| relative.to_path_buf())
}
