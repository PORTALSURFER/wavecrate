#[cfg(test)]
use notify::EventKind;
use notify::{Config, Event, EventHandler, PollWatcher, Watcher};
use std::{
    collections::HashSet,
    path::PathBuf,
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
        mpsc::{Receiver, Sender, SyncSender, TryRecvError, TrySendError},
    },
    thread,
    time::{Duration, Instant},
};
use wavecrate::sample_sources::SampleSource;

use super::classification::retain_source_refresh_candidates;
use super::roots::{RootWatchUpdate, update_watched_roots};
use super::state::GuiSourceWatchState;
use super::{
    ROOT_REFRESH_AVAILABLE, ROOT_REFRESH_UNAVAILABLE, SOURCE_CHANGE_DEBOUNCE,
    WATCHER_EVENT_QUEUE_CAPACITY, WATCHER_POLL_INTERVAL, WATCHER_RESTART_MAX, WATCHER_RESTART_MIN,
    WATCHER_START_TIMEOUT,
};
use crate::native_app::app::GuiMessage;
use crate::native_app::sample_library::committed_file_mutations::CommittedWatcherEcho;

struct ActiveSourceWatcher {
    _watcher: Box<dyn Watcher + Send>,
    ingress_enabled: Arc<AtomicBool>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum SourceWatcherBackend {
    Native,
    Polling,
}

struct PendingSourceWatcher {
    result_rx: Receiver<Result<(ActiveSourceWatcher, RootWatchUpdate, HashSet<PathBuf>), String>>,
    ingress_enabled: Arc<AtomicBool>,
    started_at: Instant,
    backend: SourceWatcherBackend,
}

struct SourceWatcherIngress {
    event_tx: SyncSender<notify::Result<Event>>,
    overflowed: Arc<AtomicBool>,
    enabled: Arc<AtomicBool>,
}

impl EventHandler for SourceWatcherIngress {
    fn handle_event(&mut self, event: notify::Result<Event>) {
        if !self.enabled.load(Ordering::Acquire) {
            return;
        }
        let event = match event {
            Ok(mut event) => {
                if !retain_source_refresh_candidates(&mut event) {
                    return;
                }
                Ok(event)
            }
            Err(error) => Err(error),
        };
        match self.event_tx.try_send(event) {
            Ok(()) => {}
            Err(TrySendError::Full(_)) => {
                self.overflowed.store(true, Ordering::Release);
            }
            Err(TrySendError::Disconnected(_)) => {}
        }
    }
}

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

    pub(in crate::native_app) fn acknowledge_committed_paths(
        &self,
        source_id: String,
        echoes: Vec<CommittedWatcherEcho>,
        operation_id: u64,
    ) {
        let _ = self
            .command_tx
            .send(GuiSourceWatchCommand::AcknowledgeCommittedPaths {
                source_id,
                echoes,
                operation_id,
            });
    }

    #[cfg(test)]
    pub(in crate::native_app) fn force_overflow_for_tests(&self) {
        let _ = self.command_tx.send(GuiSourceWatchCommand::ForceOverflow);
    }

    #[cfg(test)]
    pub(in crate::native_app) fn force_restart_for_tests(&self) {
        let _ = self.command_tx.send(GuiSourceWatchCommand::ForceRestart);
    }

    #[cfg(test)]
    pub(in crate::native_app) fn force_root_refresh_for_tests(&self) {
        let _ = self
            .command_tx
            .send(GuiSourceWatchCommand::ForceRootRefresh);
    }

    #[cfg(test)]
    pub(in crate::native_app) fn inject_paths_for_tests(&self, paths: Vec<std::path::PathBuf>) {
        let _ = self
            .command_tx
            .send(GuiSourceWatchCommand::InjectPaths(paths));
    }

    #[cfg(test)]
    pub(in crate::native_app) fn wait_until_ready_for_tests(&self) {
        let (ready_tx, ready_rx) = std::sync::mpsc::channel();
        self.command_tx
            .send(GuiSourceWatchCommand::AwaitReady(ready_tx))
            .expect("request source watcher readiness");
        ready_rx
            .recv_timeout(Duration::from_secs(30))
            .expect("source watcher should become ready");
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
    AcknowledgeCommittedPaths {
        source_id: String,
        echoes: Vec<CommittedWatcherEcho>,
        operation_id: u64,
    },
    #[cfg(test)]
    ForceOverflow,
    #[cfg(test)]
    ForceRestart,
    #[cfg(test)]
    ForceRootRefresh,
    #[cfg(test)]
    AwaitReady(Sender<()>),
    #[cfg(test)]
    InjectPaths(Vec<std::path::PathBuf>),
    Shutdown,
}

fn run_source_watcher(
    command_rx: Receiver<GuiSourceWatchCommand>,
    message_tx: Sender<GuiMessage>,
    initial_sources: Vec<SampleSource>,
) {
    let (event_tx, event_rx) = std::sync::mpsc::sync_channel(WATCHER_EVENT_QUEUE_CAPACITY);
    let ingress_overflowed = Arc::new(AtomicBool::new(false));
    let mut watcher = None;
    let mut pending_watcher = None;
    let mut state = GuiSourceWatchState::default();
    state.set_sources(initial_sources);
    let mut next_restart = Instant::now();
    let mut restart_delay = WATCHER_RESTART_MIN;
    let mut next_root_refresh = Instant::now();
    let mut watcher_has_been_ready = false;
    #[cfg(test)]
    let mut readiness_waiters = Vec::<Sender<()>>::new();

    loop {
        match command_rx.recv_timeout(WATCHER_POLL_INTERVAL) {
            Ok(GuiSourceWatchCommand::ReplaceSources(sources)) => {
                let roots_changed =
                    desired_watched_roots(&sources) != desired_watched_roots(&state.sources);
                state.set_sources(sources);
                if roots_changed {
                    retire_source_watcher(&mut watcher);
                    cancel_pending_source_watcher(&mut pending_watcher);
                    state.reset_watches(Instant::now());
                    next_restart = Instant::now();
                    restart_delay = WATCHER_RESTART_MIN;
                }
                next_root_refresh = Instant::now();
            }
            Ok(GuiSourceWatchCommand::AcknowledgeCommittedPaths {
                source_id,
                echoes,
                operation_id,
            }) => {
                state.acknowledge_committed_paths(
                    &source_id,
                    &echoes,
                    operation_id,
                    Instant::now(),
                );
            }
            #[cfg(test)]
            Ok(GuiSourceWatchCommand::ForceOverflow) => {
                state.mark_all_overflowed(Instant::now());
            }
            #[cfg(test)]
            Ok(GuiSourceWatchCommand::ForceRestart) => {
                retire_source_watcher(&mut watcher);
                cancel_pending_source_watcher(&mut pending_watcher);
                state.reset_watches(Instant::now());
                next_restart = Instant::now();
                restart_delay = WATCHER_RESTART_MIN;
            }
            #[cfg(test)]
            Ok(GuiSourceWatchCommand::ForceRootRefresh) => {
                next_root_refresh = Instant::now();
            }
            #[cfg(test)]
            Ok(GuiSourceWatchCommand::AwaitReady(ready_tx)) => {
                readiness_waiters.push(ready_tx);
            }
            #[cfg(test)]
            Ok(GuiSourceWatchCommand::InjectPaths(paths)) => {
                let event = paths
                    .into_iter()
                    .fold(Event::new(EventKind::Any), Event::add_path);
                match event_tx.try_send(Ok(event)) {
                    Ok(()) => {}
                    Err(TrySendError::Full(_)) => {
                        ingress_overflowed.store(true, Ordering::Release);
                    }
                    Err(TrySendError::Disconnected(_)) => {}
                }
            }
            Ok(GuiSourceWatchCommand::Shutdown) => {
                cancel_pending_source_watcher(&mut pending_watcher);
                retire_source_watcher(&mut watcher);
                break;
            }
            Err(std::sync::mpsc::RecvTimeoutError::Timeout) => {}
            Err(std::sync::mpsc::RecvTimeoutError::Disconnected) => break,
        }

        let now = Instant::now();
        if watcher.is_none() && pending_watcher.is_none() && now >= next_restart {
            pending_watcher = Some(spawn_source_watcher(
                state.sources.clone(),
                event_tx.clone(),
                Arc::clone(&ingress_overflowed),
                SourceWatcherBackend::Native,
            ));
        }
        if let Some(pending) = pending_watcher.take() {
            match pending.result_rx.try_recv() {
                Ok(Ok((restarted, update, watched_roots))) => {
                    state.watched_roots = watched_roots;
                    let (unavailable, watch_failed) =
                        state.apply_root_watch_update(update, now, false);
                    if watch_failed {
                        retire_source_watcher_value(restarted);
                        if watcher_has_been_ready {
                            state.reset_watches(now);
                        }
                        if pending.backend == SourceWatcherBackend::Native {
                            tracing::warn!(
                                "Native GUI source watcher could not register every root; \
                                 falling back to polling"
                            );
                            pending_watcher = Some(spawn_source_watcher(
                                state.sources.clone(),
                                event_tx.clone(),
                                Arc::clone(&ingress_overflowed),
                                SourceWatcherBackend::Polling,
                            ));
                        } else {
                            next_restart = now + restart_delay;
                            restart_delay = doubled_backoff(restart_delay);
                        }
                    } else {
                        restarted.ingress_enabled.store(true, Ordering::Release);
                        let first_ready = !watcher_has_been_ready;
                        watcher_has_been_ready = true;
                        watcher = Some(restarted);
                        if first_ready {
                            // Registration callbacks were fenced while every root was installed.
                            // Re-arm the authoritative startup audit after ingress is live so it
                            // closes that short construction window without queuing foreground
                            // scans for every source.
                            let _ = message_tx.send(GuiMessage::SourceWatcherReady);
                        }
                        restart_delay = WATCHER_RESTART_MIN;
                        next_root_refresh = now
                            + if unavailable {
                                ROOT_REFRESH_UNAVAILABLE
                            } else {
                                ROOT_REFRESH_AVAILABLE
                            };
                    }
                }
                Ok(Err(error)) => {
                    tracing::warn!(
                        backend = ?pending.backend,
                        retry_ms = restart_delay.as_millis(),
                        "Failed to initialize GUI source watcher: {error}"
                    );
                    if watcher_has_been_ready {
                        state.mark_all_overflowed(now);
                    }
                    if pending.backend == SourceWatcherBackend::Native {
                        pending_watcher = Some(spawn_source_watcher(
                            state.sources.clone(),
                            event_tx.clone(),
                            Arc::clone(&ingress_overflowed),
                            SourceWatcherBackend::Polling,
                        ));
                    } else {
                        next_restart = now + restart_delay;
                        restart_delay = doubled_backoff(restart_delay);
                    }
                }
                Err(TryRecvError::Empty)
                    if now.saturating_duration_since(pending.started_at)
                        < WATCHER_START_TIMEOUT =>
                {
                    pending_watcher = Some(pending);
                }
                Err(TryRecvError::Empty) => {
                    pending.ingress_enabled.store(false, Ordering::Release);
                    tracing::warn!(
                        backend = ?pending.backend,
                        timeout_ms = WATCHER_START_TIMEOUT.as_millis(),
                        retry_ms = restart_delay.as_millis(),
                        "Timed out initializing GUI source watcher"
                    );
                    if watcher_has_been_ready {
                        state.mark_all_overflowed(now);
                    }
                    if pending.backend == SourceWatcherBackend::Native {
                        pending_watcher = Some(spawn_source_watcher(
                            state.sources.clone(),
                            event_tx.clone(),
                            Arc::clone(&ingress_overflowed),
                            SourceWatcherBackend::Polling,
                        ));
                    } else {
                        next_restart = now + restart_delay;
                        restart_delay = doubled_backoff(restart_delay);
                    }
                }
                Err(TryRecvError::Disconnected) => {
                    tracing::warn!(
                        backend = ?pending.backend,
                        retry_ms = restart_delay.as_millis(),
                        "GUI source watcher initializer exited without a result"
                    );
                    if watcher_has_been_ready {
                        state.mark_all_overflowed(now);
                    }
                    if pending.backend == SourceWatcherBackend::Native {
                        pending_watcher = Some(spawn_source_watcher(
                            state.sources.clone(),
                            event_tx.clone(),
                            Arc::clone(&ingress_overflowed),
                            SourceWatcherBackend::Polling,
                        ));
                    } else {
                        next_restart = now + restart_delay;
                        restart_delay = doubled_backoff(restart_delay);
                    }
                }
            }
        }
        #[cfg(test)]
        if watcher.is_some() {
            for ready in readiness_waiters.drain(..) {
                let _ = ready.send(());
            }
        }

        if now >= next_root_refresh
            && watcher.is_some()
            && pending_watcher.is_none()
            && desired_watched_roots(&state.sources) != state.watched_roots
        {
            retire_source_watcher(&mut watcher);
            cancel_pending_source_watcher(&mut pending_watcher);
            state.reset_watches(now);
            next_restart = now;
            restart_delay = WATCHER_RESTART_MIN;
        }
        if now >= next_root_refresh {
            next_root_refresh = now
                + if state.sources.iter().any(|source| !source.root.is_dir()) {
                    ROOT_REFRESH_UNAVAILABLE
                } else {
                    ROOT_REFRESH_AVAILABLE
                };
        }

        if ingress_overflowed.swap(false, Ordering::AcqRel) {
            tracing::warn!("GUI source watcher event queue overflowed; reconciling every source");
            state.mark_all_overflowed(now);
        }

        let mut watcher_failed = false;
        while let Ok(event) = event_rx.try_recv() {
            let event: notify::Result<Event> = event;
            match event {
                Ok(event) => state.collect_event(&event, Instant::now()),
                Err(error) => {
                    tracing::warn!("GUI source watcher error: {error}");
                    watcher_failed = true;
                }
            }
        }

        if watcher_failed {
            retire_source_watcher(&mut watcher);
            cancel_pending_source_watcher(&mut pending_watcher);
            state.reset_watches(now);
            next_restart = now + restart_delay;
            restart_delay = doubled_backoff(restart_delay);
        }

        for event in state.drain_ready_sources(now, SOURCE_CHANGE_DEBOUNCE) {
            tracing::debug!(
                source_id = %event.source_id,
                overflowed = event.overflowed,
                source_root_available = event.source_root_available,
                paths = ?event.paths,
                "Publishing debounced GUI source watcher event"
            );
            let _ = message_tx.send(GuiMessage::SourceFilesystemChanged {
                source_id: event.source_id,
                paths: event.paths,
                overflowed: event.overflowed,
                source_root_available: event.source_root_available,
            });
        }
    }
}

fn spawn_source_watcher(
    sources: Vec<SampleSource>,
    event_tx: SyncSender<notify::Result<Event>>,
    ingress_overflowed: Arc<AtomicBool>,
    backend: SourceWatcherBackend,
) -> PendingSourceWatcher {
    let (result_tx, result_rx) = std::sync::mpsc::channel();
    // Native backends can emit callbacks while roots are being registered.
    // Fence those callbacks and open ingress only once the complete watcher is
    // installed; the watcher-ready audit covers the construction interval.
    let ingress_enabled = Arc::new(AtomicBool::new(false));
    let watcher_enabled = Arc::clone(&ingress_enabled);
    let _ = thread::Builder::new()
        .name("wavecrate-source-watcher-start".to_string())
        .spawn(move || {
            let ingress = SourceWatcherIngress {
                event_tx,
                overflowed: ingress_overflowed,
                enabled: Arc::clone(&watcher_enabled),
            };
            let watcher: Result<Box<dyn Watcher + Send>, String> = match backend {
                SourceWatcherBackend::Native => notify::recommended_watcher(ingress)
                    .map(|watcher| Box::new(watcher) as Box<dyn Watcher + Send>)
                    .map_err(|error| error.to_string()),
                SourceWatcherBackend::Polling => PollWatcher::new(
                    ingress,
                    Config::default().with_poll_interval(Duration::from_secs(1)),
                )
                .map(|watcher| Box::new(watcher) as Box<dyn Watcher + Send>)
                .map_err(|error| error.to_string()),
            };
            let result = watcher.map(|mut watcher| {
                let mut watched_roots = HashSet::new();
                let update = update_watched_roots(watcher.as_mut(), &mut watched_roots, &sources);
                (
                    ActiveSourceWatcher {
                        _watcher: watcher,
                        ingress_enabled: watcher_enabled,
                    },
                    update,
                    watched_roots,
                )
            });
            let _ = result_tx.send(result);
        });
    PendingSourceWatcher {
        result_rx,
        ingress_enabled,
        started_at: Instant::now(),
        backend,
    }
}

fn desired_watched_roots(sources: &[SampleSource]) -> HashSet<PathBuf> {
    sources
        .iter()
        .map(|source| source.root.clone())
        .filter(|root| root.is_dir())
        .collect()
}

fn cancel_pending_source_watcher(watcher: &mut Option<PendingSourceWatcher>) {
    if let Some(watcher) = watcher.take() {
        watcher.ingress_enabled.store(false, Ordering::Release);
    }
}

/// Stop accepting callbacks before dropping the macOS FSEvents watcher off the coordinator.
///
/// `notify` waits for the Core Foundation run loop to become idle during `Drop`. A busy source can
/// keep that wait inside the watcher coordinator for an unbounded interval, preventing restart,
/// recovery events, and shutdown. Fencing callback ingress makes the old stream quiescent while a
/// short-lived reaper performs the backend-specific blocking teardown.
fn retire_source_watcher(watcher: &mut Option<ActiveSourceWatcher>) {
    if let Some(watcher) = watcher.take() {
        retire_source_watcher_value(watcher);
    }
}

fn retire_source_watcher_value(watcher: ActiveSourceWatcher) {
    watcher.ingress_enabled.store(false, Ordering::Release);
    let _ = thread::Builder::new()
        .name("wavecrate-source-watcher-reaper".to_string())
        .spawn(move || drop(watcher));
}

pub(super) fn doubled_backoff(current: Duration) -> Duration {
    current.saturating_mul(2).min(WATCHER_RESTART_MAX)
}
