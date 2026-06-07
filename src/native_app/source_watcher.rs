use notify::{Event, EventKind, RecommendedWatcher, RecursiveMode, Watcher};
use std::{
    collections::{BTreeSet, HashMap, HashSet},
    path::{Path, PathBuf},
    sync::mpsc::{Receiver, Sender},
    thread,
    time::{Duration, Instant},
};
use wavecrate::sample_sources::SampleSource;

use super::GuiMessage;

const WATCHER_POLL_INTERVAL: Duration = Duration::from_millis(200);
const SOURCE_CHANGE_DEBOUNCE: Duration = Duration::from_millis(400);
const MAX_PENDING_PATHS_PER_SOURCE: usize = 512;

#[derive(Debug)]
pub(in crate::native_app) struct GuiSourceWatcherHandle {
    command_tx: Sender<GuiSourceWatchCommand>,
    join_handle: Option<thread::JoinHandle<()>>,
}

impl GuiSourceWatcherHandle {
    pub(in crate::native_app) fn spawn(
        sources: Vec<SampleSource>,
        message_tx: Sender<GuiMessage>,
    ) -> Self {
        let (command_tx, command_rx) = std::sync::mpsc::channel();
        let handle = thread::spawn(move || run_source_watcher(command_rx, message_tx, sources));
        Self {
            command_tx,
            join_handle: Some(handle),
        }
    }

    pub(in crate::native_app) fn replace_sources(&self, sources: Vec<SampleSource>) {
        let _ = self
            .command_tx
            .send(GuiSourceWatchCommand::ReplaceSources(sources));
    }
}

impl Drop for GuiSourceWatcherHandle {
    fn drop(&mut self) {
        let _ = self.command_tx.send(GuiSourceWatchCommand::Shutdown);
        if let Some(handle) = self.join_handle.take() {
            let _ = handle.join();
        }
    }
}

#[derive(Debug)]
enum GuiSourceWatchCommand {
    ReplaceSources(Vec<SampleSource>),
    Shutdown,
}

#[derive(Default)]
struct GuiSourceWatchState {
    watched_roots: HashSet<PathBuf>,
    sources: Vec<SampleSource>,
    pending: HashMap<String, PendingGuiSourceWatch>,
}

fn run_source_watcher(
    command_rx: Receiver<GuiSourceWatchCommand>,
    message_tx: Sender<GuiMessage>,
    initial_sources: Vec<SampleSource>,
) {
    let (event_tx, event_rx) = std::sync::mpsc::channel();
    let mut watcher = match notify::recommended_watcher(move |event| {
        let _ = event_tx.send(event);
    }) {
        Ok(watcher) => watcher,
        Err(error) => {
            tracing::warn!("Failed to initialize GUI source watcher: {error}");
            return;
        }
    };
    let mut state = GuiSourceWatchState::default();
    state.replace_sources(initial_sources, &mut watcher);

    loop {
        match command_rx.recv_timeout(WATCHER_POLL_INTERVAL) {
            Ok(GuiSourceWatchCommand::ReplaceSources(sources)) => {
                state.replace_sources(sources, &mut watcher);
            }
            Ok(GuiSourceWatchCommand::Shutdown) => break,
            Err(std::sync::mpsc::RecvTimeoutError::Timeout) => {}
            Err(std::sync::mpsc::RecvTimeoutError::Disconnected) => break,
        }

        while let Ok(event) = event_rx.try_recv() {
            let event: notify::Result<Event> = event;
            match event {
                Ok(event) if event_triggers_source_refresh(&event) => {
                    state.collect_event(&event, Instant::now());
                }
                Ok(_) => {}
                Err(error) => {
                    tracing::warn!("GUI source watcher error: {error}");
                }
            }
        }

        for event in state.drain_ready_sources(Instant::now(), SOURCE_CHANGE_DEBOUNCE) {
            let _ = message_tx.send(GuiMessage::SourceFilesystemChanged {
                source_id: event.source_id,
                paths: event.paths,
                overflowed: event.overflowed,
            });
        }
    }
}

impl GuiSourceWatchState {
    fn replace_sources(&mut self, sources: Vec<SampleSource>, watcher: &mut RecommendedWatcher) {
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

    fn collect_event(&mut self, event: &Event, now: Instant) {
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

    fn drain_ready_sources(
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

#[derive(Debug)]
struct PendingGuiSourceWatch {
    last_event: Instant,
    paths: BTreeSet<PathBuf>,
    overflowed: bool,
}

impl PendingGuiSourceWatch {
    fn new(last_event: Instant, path: Option<PathBuf>) -> Self {
        let mut pending = Self {
            last_event,
            paths: BTreeSet::new(),
            overflowed: false,
        };
        pending.add_path(path);
        pending
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

#[derive(Debug)]
struct GuiSourceWatchEvent {
    source_id: String,
    paths: Vec<PathBuf>,
    overflowed: bool,
}

fn update_watched_roots(
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

fn event_triggers_source_refresh(event: &Event) -> bool {
    matches!(
        event.kind,
        EventKind::Create(_) | EventKind::Modify(_) | EventKind::Remove(_) | EventKind::Any
    )
}

fn path_is_source_refresh_candidate(path: &Path, kind: EventKind) -> bool {
    if is_wavecrate_metadata_file(path) {
        return false;
    }
    matches!(kind, EventKind::Remove(_) | EventKind::Any)
        || path_has_supported_audio_extension(path)
        || path.extension().is_none()
        || path.is_dir()
}

fn path_has_supported_audio_extension(path: &Path) -> bool {
    path.extension()
        .and_then(|extension| extension.to_str())
        .is_some_and(|extension| {
            matches!(
                extension.to_ascii_lowercase().as_str(),
                "wav" | "wave" | "aif" | "aiff"
            )
        })
}

fn is_wavecrate_metadata_file(path: &Path) -> bool {
    let Some(name) = path.file_name().and_then(|name| name.to_str()) else {
        return false;
    };
    name.starts_with(wavecrate::sample_sources::db::DB_FILE_NAME)
        || name.starts_with(wavecrate::sample_sources::db::LEGACY_DB_FILE_NAME)
}

fn source_for_path<'a>(sources: &'a [SampleSource], path: &Path) -> Option<&'a SampleSource> {
    sources
        .iter()
        .filter(|source| path.starts_with(&source.root))
        .max_by_key(|source| source.root.components().count())
}

fn source_relative_path(source: &SampleSource, path: &Path) -> Option<PathBuf> {
    let relative = path.strip_prefix(&source.root).ok()?;
    (!relative.as_os_str().is_empty()).then(|| relative.to_path_buf())
}

#[cfg(test)]
mod tests {
    use super::*;
    use notify::{EventKind, event::RemoveKind};
    use wavecrate::sample_sources::SourceId;

    #[test]
    fn removed_extension_named_folder_triggers_source_refresh() {
        let root = PathBuf::from(r"C:\samples");
        let source =
            SampleSource::new_with_id(SourceId::from_string("source_id::samples"), root.clone());
        let mut state = GuiSourceWatchState {
            sources: vec![source],
            ..Default::default()
        };
        let event = Event {
            kind: EventKind::Remove(RemoveKind::Folder),
            paths: vec![root.join("Drum.Loops")],
            attrs: Default::default(),
        };

        state.collect_event(&event, Instant::now());

        let pending = state.pending.get("source_id::samples").unwrap();
        assert!(pending.paths.contains(&PathBuf::from("Drum.Loops")));
        assert!(!pending.overflowed);
    }

    #[test]
    fn wavecrate_metadata_files_do_not_trigger_source_refresh() {
        let root = PathBuf::from(r"C:\samples");
        assert!(!path_is_source_refresh_candidate(
            &root.join(wavecrate::sample_sources::db::DB_FILE_NAME),
            EventKind::Modify(notify::event::ModifyKind::Data(
                notify::event::DataChange::Any
            )),
        ));
    }

    #[test]
    fn source_root_event_overflows_to_full_refresh() {
        let root = PathBuf::from(r"C:\samples");
        let source =
            SampleSource::new_with_id(SourceId::from_string("source_id::samples"), root.clone());
        let mut state = GuiSourceWatchState {
            sources: vec![source],
            ..Default::default()
        };
        let event = Event {
            kind: EventKind::Any,
            paths: vec![root],
            attrs: Default::default(),
        };

        state.collect_event(&event, Instant::now());

        let pending = state.pending.get("source_id::samples").unwrap();
        assert!(pending.paths.is_empty());
        assert!(pending.overflowed);
    }
}
