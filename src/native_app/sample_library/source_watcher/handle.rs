use notify::Event;
use std::{
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
        mpsc::{Receiver, Sender, TrySendError},
    },
    thread,
    time::{Duration, Instant},
};
use wavecrate::sample_sources::SampleSource;

use super::classification::retain_source_refresh_candidates;
use super::state::GuiSourceWatchState;
use super::{
    ROOT_REFRESH_AVAILABLE, ROOT_REFRESH_UNAVAILABLE, SOURCE_CHANGE_DEBOUNCE,
    WATCHER_EVENT_QUEUE_CAPACITY, WATCHER_POLL_INTERVAL, WATCHER_RESTART_MAX, WATCHER_RESTART_MIN,
};
use crate::native_app::app::GuiMessage;
use crate::native_app::sample_library::committed_file_mutations::CommittedWatcherEcho;

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
    let mut state = GuiSourceWatchState::default();
    state.set_sources(initial_sources);
    let mut next_restart = Instant::now();
    let mut restart_delay = WATCHER_RESTART_MIN;
    let mut next_root_refresh = Instant::now();

    loop {
        match command_rx.recv_timeout(WATCHER_POLL_INTERVAL) {
            Ok(GuiSourceWatchCommand::ReplaceSources(sources)) => {
                state.set_sources(sources);
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
            Ok(GuiSourceWatchCommand::Shutdown) => break,
            Err(std::sync::mpsc::RecvTimeoutError::Timeout) => {}
            Err(std::sync::mpsc::RecvTimeoutError::Disconnected) => break,
        }

        let now = Instant::now();
        if watcher.is_none() && now >= next_restart {
            let callback_tx = event_tx.clone();
            let callback_overflowed = Arc::clone(&ingress_overflowed);
            match notify::recommended_watcher(move |event| {
                let event = match event {
                    Ok(mut event) => {
                        if !retain_source_refresh_candidates(&mut event) {
                            return;
                        }
                        Ok(event)
                    }
                    Err(error) => Err(error),
                };
                match callback_tx.try_send(event) {
                    Ok(()) => {}
                    Err(TrySendError::Full(_)) => {
                        callback_overflowed.store(true, Ordering::Release);
                    }
                    Err(TrySendError::Disconnected(_)) => {}
                }
            }) {
                Ok(mut restarted) => {
                    let (unavailable, watch_failed) =
                        state.refresh_watched_roots(&mut restarted, now, false);
                    if watch_failed {
                        state.reset_watches(now);
                        next_restart = now + restart_delay;
                        restart_delay = doubled_backoff(restart_delay);
                    } else {
                        watcher = Some(restarted);
                        restart_delay = WATCHER_RESTART_MIN;
                        next_root_refresh = now
                            + if unavailable {
                                ROOT_REFRESH_UNAVAILABLE
                            } else {
                                ROOT_REFRESH_AVAILABLE
                            };
                    }
                }
                Err(error) => {
                    tracing::warn!(
                        retry_ms = restart_delay.as_millis(),
                        "Failed to initialize GUI source watcher: {error}"
                    );
                    state.mark_all_overflowed(now);
                    next_restart = now + restart_delay;
                    restart_delay = doubled_backoff(restart_delay);
                }
            }
        }

        let mut root_refresh_failed = false;
        if let Some(active_watcher) = watcher.as_mut()
            && now >= next_root_refresh
        {
            let (unavailable, watch_failed) =
                state.refresh_watched_roots(active_watcher, now, true);
            root_refresh_failed = watch_failed;
            if !watch_failed {
                next_root_refresh = now
                    + if unavailable {
                        ROOT_REFRESH_UNAVAILABLE
                    } else {
                        ROOT_REFRESH_AVAILABLE
                    };
            }
        }
        if root_refresh_failed {
            watcher = None;
            state.reset_watches(now);
            next_restart = now + restart_delay;
            restart_delay = doubled_backoff(restart_delay);
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
            watcher = None;
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

pub(super) fn doubled_backoff(current: Duration) -> Duration {
    current.saturating_mul(2).min(WATCHER_RESTART_MAX)
}
