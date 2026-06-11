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
    pub(in crate::native_app) audio_open_task: ui::LatestTask,
    pub(in crate::native_app) audio_open_results:
        Arc<Mutex<HashMap<ui::TaskTicket, Result<AudioPlayer, String>>>>,
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
            audio_open_task: ui::LatestTask::new(),
            audio_open_results: Default::default(),
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
