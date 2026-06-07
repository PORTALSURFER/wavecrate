//! File system watcher for source roots that reports audio-relevant changes.

use crate::app::controller::jobs::{JobMessage, JobMessageSender};
use crate::sample_sources::{
    SourceId,
    db::{DB_FILE_NAME, LEGACY_DB_FILE_NAME},
    is_supported_audio,
};
use notify::{
    Event, EventKind, RecommendedWatcher, RecursiveMode, Result as NotifyResult, Watcher,
};
use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::sync::mpsc::{Receiver, Sender};
use std::thread;
use std::time::{Duration, Instant};

mod state;

use state::SourceWatcherState;

const COMMAND_POLL_INTERVAL: Duration = Duration::from_millis(200);
const SOURCE_WATCH_DEBOUNCE: Duration = Duration::from_millis(400);

/// Input used to configure which source roots are actively watched.
#[derive(Clone, Debug)]
pub(crate) struct SourceWatchEntry {
    pub(crate) source_id: SourceId,
    pub(crate) root: PathBuf,
}

impl SourceWatchEntry {
    /// Create a watch entry for a source root.
    pub(crate) fn new(source_id: SourceId, root: PathBuf) -> Self {
        Self { source_id, root }
    }
}

/// Commands sent to the watcher thread to update its configuration.
#[derive(Debug)]
pub(crate) enum SourceWatchCommand {
    /// Replace the watched sources with a new list of source roots.
    ReplaceSources(Vec<SourceWatchEntry>),
    /// Tell the watcher whether a scan job is currently running.
    SetScanInProgress { in_progress: bool },
    /// Mark source-local paths currently owned by controller-dispatched file operations.
    BeginControllerFileOp {
        source_id: SourceId,
        relative_paths: Vec<PathBuf>,
    },
    /// Clear source-local paths after controller-dispatched file operations finish.
    FinishControllerFileOp {
        source_id: SourceId,
        relative_paths: Vec<PathBuf>,
    },
    /// Signal the watcher thread to exit.
    Shutdown,
}

/// Why the source watcher believes a source changed.
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub(crate) enum SourceWatchCause {
    /// Disk changed outside a controller-owned file operation.
    ExternalFileChange,
    /// The watched change matches a controller-owned file operation.
    ControllerFileOp,
}

/// Event emitted when a watched source sees an on-disk change worth syncing.
#[derive(Debug, Clone)]
pub(crate) struct SourceWatchEvent {
    pub(crate) source_id: SourceId,
    pub(crate) cause: SourceWatchCause,
    pub(crate) paths: Vec<PathBuf>,
    pub(crate) overflowed: bool,
}

/// Join handle and command sender for the source watcher thread.
pub(crate) struct SourceWatcherHandle {
    command_tx: Sender<SourceWatchCommand>,
    join_handle: Option<thread::JoinHandle<()>>,
}

impl SourceWatcherHandle {
    /// Send a command to the watcher thread.
    pub(crate) fn send(&self, command: SourceWatchCommand) {
        let _ = self.command_tx.send(command);
    }

    /// Signal the watcher thread to exit and wait for it to finish.
    pub(crate) fn shutdown(&mut self) {
        let _ = self.command_tx.send(SourceWatchCommand::Shutdown);
        if let Some(handle) = self.join_handle.take() {
            let _ = handle.join();
        }
    }

    /// Signal the watcher thread to exit without waiting for it to finish.
    pub(crate) fn request_shutdown_detached(&mut self) {
        let _ = self.command_tx.send(SourceWatchCommand::Shutdown);
        let _ = self.join_handle.take();
    }
}

/// Spawn the watcher thread and return a handle used to update watched sources.
pub(crate) fn spawn_source_watcher(message_tx: JobMessageSender) -> SourceWatcherHandle {
    let (command_tx, command_rx) = std::sync::mpsc::channel();
    let handle = thread::spawn(move || run_source_watcher(command_rx, message_tx));
    SourceWatcherHandle {
        command_tx,
        join_handle: Some(handle),
    }
}

fn run_source_watcher(command_rx: Receiver<SourceWatchCommand>, message_tx: JobMessageSender) {
    let (event_tx, event_rx) = std::sync::mpsc::channel::<NotifyResult<Event>>();
    let mut watcher = match notify::recommended_watcher(move |event| {
        let _ = event_tx.send(event);
    }) {
        Ok(watcher) => watcher,
        Err(err) => {
            tracing::warn!("Failed to initialize source watcher: {err}");
            return;
        }
    };
    let mut state = SourceWatcherState::default();

    loop {
        match command_rx.recv_timeout(COMMAND_POLL_INTERVAL) {
            Ok(command) => {
                if !state.handle_command(command, &mut watcher) {
                    break;
                }
            }
            Err(std::sync::mpsc::RecvTimeoutError::Timeout) => {}
            Err(std::sync::mpsc::RecvTimeoutError::Disconnected) => break,
        }

        while let Ok(event) = event_rx.try_recv() {
            let event = match event {
                Ok(event) => event,
                Err(err) => {
                    tracing::warn!("Source watcher error: {err}");
                    continue;
                }
            };
            if !event_triggers_sync(&event) {
                continue;
            }
            state.collect_event(event, Instant::now());
        }

        for event in state.drain_ready_sources(Instant::now(), SOURCE_WATCH_DEBOUNCE) {
            let _ = message_tx.send(JobMessage::SourceWatch(event));
        }
    }
}

fn update_watched_sources(
    watcher: &mut RecommendedWatcher,
    watched_roots: &mut HashSet<PathBuf>,
    sources: &[SourceWatchEntry],
) {
    let desired: HashSet<PathBuf> = sources
        .iter()
        .map(|entry| entry.root.clone())
        .filter(|root| root.is_dir())
        .collect();
    for root in watched_roots
        .difference(&desired)
        .cloned()
        .collect::<Vec<_>>()
    {
        if let Err(err) = watcher.unwatch(&root) {
            tracing::warn!("Failed to unwatch source root {}: {err}", root.display());
        }
        watched_roots.remove(&root);
    }
    for root in desired
        .difference(watched_roots)
        .cloned()
        .collect::<Vec<_>>()
    {
        if let Err(err) = watcher.watch(&root, RecursiveMode::Recursive) {
            tracing::warn!("Failed to watch source root {}: {err}", root.display());
            continue;
        }
        watched_roots.insert(root);
    }
}

fn event_triggers_sync(event: &Event) -> bool {
    matches!(
        event.kind,
        EventKind::Create(_) | EventKind::Modify(_) | EventKind::Remove(_) | EventKind::Any
    )
}

fn select_source_for_path(sources: &[SourceWatchEntry], path: &Path) -> Option<SourceId> {
    select_source_entry_for_path(sources, path).map(|entry| entry.source_id.clone())
}

fn select_source_entry_for_path<'a>(
    sources: &'a [SourceWatchEntry],
    path: &Path,
) -> Option<&'a SourceWatchEntry> {
    sources
        .iter()
        .filter(|entry| path.starts_with(&entry.root))
        .max_by_key(|entry| entry.root.as_os_str().len())
}

fn source_watch_cause_for_path(
    controller_file_ops: &HashMap<SourceId, HashSet<PathBuf>>,
    source: &SourceWatchEntry,
    path: &Path,
) -> SourceWatchCause {
    let Some(owned_paths) = controller_file_ops.get(&source.source_id) else {
        return SourceWatchCause::ExternalFileChange;
    };
    let Ok(relative_path) = path.strip_prefix(&source.root) else {
        return SourceWatchCause::ExternalFileChange;
    };
    if owned_paths
        .iter()
        .any(|owned| relative_path == owned || relative_path.starts_with(owned))
    {
        SourceWatchCause::ControllerFileOp
    } else {
        SourceWatchCause::ExternalFileChange
    }
}

fn combine_source_watch_causes(
    current: SourceWatchCause,
    next: SourceWatchCause,
) -> SourceWatchCause {
    match (current, next) {
        (SourceWatchCause::ExternalFileChange, _) | (_, SourceWatchCause::ExternalFileChange) => {
            SourceWatchCause::ExternalFileChange
        }
        (SourceWatchCause::ControllerFileOp, SourceWatchCause::ControllerFileOp) => {
            SourceWatchCause::ControllerFileOp
        }
    }
}

fn path_is_candidate(path: &Path, kind: EventKind) -> bool {
    if path_is_ignored(path) {
        return false;
    }
    if matches!(kind, EventKind::Remove(_) | EventKind::Any) {
        return true;
    }
    if is_supported_audio(path) {
        return true;
    }
    path_extensionless_is_directory(path)
}

fn path_is_ignored(path: &Path) -> bool {
    let Some(name) = path.file_name().and_then(|name| name.to_str()) else {
        return false;
    };
    name.starts_with(DB_FILE_NAME) || name.starts_with(LEGACY_DB_FILE_NAME)
}

fn path_extensionless_is_directory(path: &Path) -> bool {
    if path.extension().is_some() {
        return false;
    }
    match path.metadata() {
        Ok(metadata) => metadata.is_dir(),
        Err(_) => true,
    }
}

#[cfg(test)]
mod tests;
