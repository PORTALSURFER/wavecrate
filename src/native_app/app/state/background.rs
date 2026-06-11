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
    pub(in crate::native_app) audio_open: AudioOpenTaskOwner,
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
            audio_open: AudioOpenTaskOwner::new(),
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

/// Owns audio-output open task identity and stale-completion policy.
pub(in crate::native_app) struct AudioOpenTaskOwner {
    task: ui::LatestTask,
}

impl AudioOpenTaskOwner {
    fn new() -> Self {
        Self {
            task: ui::LatestTask::new(),
        }
    }

    pub(in crate::native_app) fn active(&self) -> Option<ui::TaskTicket> {
        self.task.active()
    }

    pub(in crate::native_app) fn begin(&mut self) -> AudioOpenTaskRequest {
        AudioOpenTaskRequest {
            ticket: self.task.begin(),
        }
    }

    pub(in crate::native_app) fn cancel(&mut self) {
        self.task.cancel();
    }

    pub(in crate::native_app) fn finish(
        &mut self,
        completion: AudioOpenTaskCompletion,
    ) -> AudioOpenCompletion {
        if !self.task.finish(completion.ticket()) {
            return AudioOpenCompletion::Stale;
        }
        AudioOpenCompletion::Current(Box::new(
            completion
                .take_result()
                .unwrap_or_else(|| Err(String::from("audio output worker did not report"))),
        ))
    }
}

/// Cloneable message payload for a non-cloneable audio-open result.
#[derive(Clone)]
pub(in crate::native_app) struct AudioOpenTaskCompletion {
    ticket: ui::TaskTicket,
    result: Arc<Mutex<Option<Result<AudioPlayer, String>>>>,
}

impl AudioOpenTaskCompletion {
    pub(in crate::native_app) fn ticket(&self) -> ui::TaskTicket {
        self.ticket
    }

    fn take_result(self) -> Option<Result<AudioPlayer, String>> {
        self.result.lock().ok().and_then(|mut result| result.take())
    }
}

impl std::fmt::Debug for AudioOpenTaskCompletion {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("AudioOpenTaskCompletion")
            .field("ticket", &self.ticket)
            .finish_non_exhaustive()
    }
}

impl PartialEq for AudioOpenTaskCompletion {
    fn eq(&self, other: &Self) -> bool {
        self.ticket == other.ticket
    }
}

/// Worker-owned request token that can produce exactly one completion result.
pub(in crate::native_app) struct AudioOpenTaskRequest {
    ticket: ui::TaskTicket,
}

impl AudioOpenTaskRequest {
    pub(in crate::native_app) fn complete(
        self,
        result: Result<AudioPlayer, String>,
    ) -> AudioOpenTaskCompletion {
        AudioOpenTaskCompletion {
            ticket: self.ticket,
            result: Arc::new(Mutex::new(Some(result))),
        }
    }
}

pub(in crate::native_app) enum AudioOpenCompletion {
    Current(Box<Result<AudioPlayer, String>>),
    Stale,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn audio_open_task_owner_ignores_stale_ticket_results() {
        let mut owner = AudioOpenTaskOwner::new();
        let stale_completion = owner.begin().complete(Err(String::from("stale")));
        let current_completion = owner.begin().complete(Err(String::from("current")));

        assert!(matches!(
            owner.finish(stale_completion),
            AudioOpenCompletion::Stale
        ));
        assert!(
            matches!(owner.finish(current_completion), AudioOpenCompletion::Current(result) if result.as_ref().is_err())
        );
    }

    #[test]
    fn audio_open_task_completion_reports_missing_result_after_consumption() {
        let completion = AudioOpenTaskOwner::new()
            .begin()
            .complete(Err(String::from("reported")));
        let clone = completion.clone();

        assert!(matches!(
            completion.take_result(),
            Some(Err(error)) if error == "reported"
        ));
        assert!(clone.take_result().is_none());
    }
}
