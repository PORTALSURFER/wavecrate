use notify::Event;
use std::{
    sync::mpsc::{Receiver, Sender},
    thread,
    time::Instant,
};
use wavecrate::sample_sources::SampleSource;

use super::classification::event_triggers_source_refresh;
use super::state::GuiSourceWatchState;
use super::{SOURCE_CHANGE_DEBOUNCE, WATCHER_POLL_INTERVAL};
use crate::native_app::app::GuiMessage;

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
