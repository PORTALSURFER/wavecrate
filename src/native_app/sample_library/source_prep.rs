use std::time::Instant;

use radiant::prelude as ui;

use crate::native_app::app::{GuiMessage, NativeAppState, emit_gui_action};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(in crate::native_app) enum SourcePrepTrigger {
    UserRequested,
    SourceSelected,
    FolderActivated,
    SourceVerified,
    SourceScanFinished,
    FilesystemChanged,
}

impl SourcePrepTrigger {
    fn action_label(self) -> &'static str {
        match self {
            Self::UserRequested => "user_requested",
            Self::SourceSelected => "source_selected",
            Self::FolderActivated => "folder_activated",
            Self::SourceVerified => "source_verified",
            Self::SourceScanFinished => "source_scan_finished",
            Self::FilesystemChanged => "filesystem_changed",
        }
    }

    fn schedules_source_cache_warm(self) -> bool {
        matches!(self, Self::UserRequested)
    }

    fn force_metadata_refresh(self) -> bool {
        matches!(
            self,
            Self::UserRequested | Self::SourceScanFinished | Self::FilesystemChanged
        )
    }

    fn invalidates_running_source_work(self) -> bool {
        matches!(self, Self::FilesystemChanged)
    }
}

impl NativeAppState {
    pub(in crate::native_app) fn queue_selected_source_prep(
        &mut self,
        trigger: SourcePrepTrigger,
        context: &mut ui::UiUpdateContext<GuiMessage>,
    ) {
        let source_id = self.library.folder_browser.selected_source_id().to_string();
        self.queue_source_prep(source_id, trigger, context);
    }

    pub(in crate::native_app) fn queue_source_prep(
        &mut self,
        source_id: String,
        trigger: SourcePrepTrigger,
        context: &mut ui::UiUpdateContext<GuiMessage>,
    ) {
        let started_at = Instant::now();
        let selected_source = source_id == self.library.folder_browser.selected_source_id();
        if trigger == SourcePrepTrigger::UserRequested {
            self.background
                .source_processing
                .request_source_reanalysis(&source_id, trigger.action_label());
        } else if trigger.invalidates_running_source_work() {
            self.background
                .source_processing
                .wake_source(&source_id, trigger.action_label());
        } else {
            self.background
                .source_processing
                .request_source_processing(&source_id, trigger.action_label());
        }
        if selected_source {
            self.background
                .source_processing
                .set_selected_source(Some(&source_id));
        }
        self.schedule_persisted_metadata_tags_refresh_for_source(
            &source_id,
            trigger.force_metadata_refresh(),
            context,
        );
        if selected_source {
            self.schedule_persisted_waveform_cache_indicator_refresh(context);
        }
        let cache_scheduled = if trigger.schedules_source_cache_warm() {
            if selected_source {
                self.schedule_active_folder_cache_warm(context)
            } else {
                self.schedule_source_cache_warm(&source_id, context)
            }
        } else {
            false
        };
        if trigger == SourcePrepTrigger::UserRequested && !cache_scheduled {
            self.ui.status.sample = String::from("Source processing queued");
        }
        emit_gui_action(
            "source_prep.queue",
            Some("background"),
            Some(&source_id),
            trigger.action_label(),
            started_at,
            None,
        );
    }
}
