use std::{
    collections::HashMap,
    sync::{
        Arc, Mutex,
        mpsc::{Receiver, Sender},
    },
};

use radiant::prelude as ui;
use wavecrate::audio::AudioPlayer;

use crate::native_app::app::{GuiMessage, NormalizationProgress};
use crate::native_app::sample_library::folder_browser::scan::FolderVerifyResult;

pub(in crate::native_app) struct BackgroundTaskState {
    pub(in crate::native_app) worker_sender: Sender<GuiMessage>,
    pub(in crate::native_app) worker_receiver: Option<Receiver<GuiMessage>>,
    pub(in crate::native_app) next_task_id: u64,
    pub(in crate::native_app) deferred_sample_load_task: ui::LatestTask,
    pub(in crate::native_app) sample_load_task: ui::LatestTask,
    pub(in crate::native_app) sample_load_cancel: Option<ui::CancellationToken>,
    pub(in crate::native_app) audio_open: AudioOpenCompletionOwner,
    pub(in crate::native_app) startup_folder_verify_task: ui::LatestTask,
    pub(in crate::native_app) startup_folder_verify_results:
        Arc<Mutex<HashMap<ui::TaskTicket, FolderVerifyResult>>>,
    pub(in crate::native_app) normalization_progress: Option<NormalizationProgress>,
    pub(in crate::native_app) progress_tick: f32,
    pub(in crate::native_app) frame_cadence: ui::FrameCadenceMonitor,
}

impl BackgroundTaskState {
    pub(in crate::native_app) fn new(
        worker_sender: Sender<GuiMessage>,
        worker_receiver: Option<Receiver<GuiMessage>>,
    ) -> Self {
        Self {
            worker_sender,
            worker_receiver,
            next_task_id: 1,
            deferred_sample_load_task: ui::LatestTask::new(),
            sample_load_task: ui::LatestTask::new(),
            sample_load_cancel: None,
            audio_open: AudioOpenCompletionOwner::new(),
            startup_folder_verify_task: ui::LatestTask::new(),
            startup_folder_verify_results: Default::default(),
            normalization_progress: None,
            progress_tick: 0.0,
            frame_cadence: ui::FrameCadenceMonitor::new(),
        }
    }

    pub(in crate::native_app) fn next_task_id(&mut self) -> u64 {
        let task_id = self.next_task_id;
        self.next_task_id = self.next_task_id.saturating_add(1);
        task_id
    }

    #[cfg(test)]
    pub(in crate::native_app) fn for_tests() -> Self {
        Self::new(std::sync::mpsc::channel().0, None)
    }
}

pub(in crate::native_app) struct AudioOpenCompletionOwner {
    task: ui::LatestTask,
    results: Arc<Mutex<HashMap<ui::TaskTicket, Result<AudioPlayer, String>>>>,
}

impl AudioOpenCompletionOwner {
    fn new() -> Self {
        Self {
            task: ui::LatestTask::new(),
            results: Default::default(),
        }
    }

    pub(in crate::native_app) fn active(&self) -> Option<ui::TaskTicket> {
        self.task.active()
    }

    pub(in crate::native_app) fn begin(&mut self) -> ui::TaskTicket {
        self.task.begin()
    }

    pub(in crate::native_app) fn cancel(&mut self) {
        self.task.cancel();
    }

    pub(in crate::native_app) fn sink(&self) -> AudioOpenCompletionSink {
        AudioOpenCompletionSink {
            results: Arc::clone(&self.results),
        }
    }

    pub(in crate::native_app) fn finish(&mut self, ticket: ui::TaskTicket) -> AudioOpenCompletion {
        let result = self
            .results
            .lock()
            .ok()
            .and_then(|mut results| results.remove(&ticket));
        if !self.task.finish(ticket) {
            return AudioOpenCompletion::Stale;
        }
        AudioOpenCompletion::Current(
            result.unwrap_or_else(|| Err(String::from("audio output worker did not report"))),
        )
    }
}

#[derive(Clone)]
pub(in crate::native_app) struct AudioOpenCompletionSink {
    results: Arc<Mutex<HashMap<ui::TaskTicket, Result<AudioPlayer, String>>>>,
}

impl AudioOpenCompletionSink {
    pub(in crate::native_app) fn complete(
        &self,
        ticket: ui::TaskTicket,
        result: Result<AudioPlayer, String>,
    ) {
        if let Ok(mut results) = self.results.lock() {
            results.insert(ticket, result);
        }
    }
}

pub(in crate::native_app) enum AudioOpenCompletion {
    Current(Result<AudioPlayer, String>),
    Stale,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn audio_open_completion_owner_ignores_stale_ticket_results() {
        let mut owner = AudioOpenCompletionOwner::new();
        let stale_ticket = owner.begin();
        let sink = owner.sink();
        sink.complete(stale_ticket, Err(String::from("stale")));
        let current_ticket = owner.begin();

        assert!(matches!(
            owner.finish(stale_ticket),
            AudioOpenCompletion::Stale
        ));
        assert!(matches!(
            owner.finish(current_ticket),
            AudioOpenCompletion::Current(Err(_))
        ));
    }
}
