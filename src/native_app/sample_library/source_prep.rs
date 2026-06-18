use std::time::Instant;

use radiant::prelude as ui;

use crate::native_app::app::{GuiMessage, NativeAppState, emit_gui_action};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(in crate::native_app) enum SourcePrepTrigger {
    SourceSelected,
    FolderActivated,
    SourceVerified,
    SourceScanFinished,
    FilesystemChanged,
}

impl SourcePrepTrigger {
    fn action_label(self) -> &'static str {
        match self {
            Self::SourceSelected => "source_selected",
            Self::FolderActivated => "folder_activated",
            Self::SourceVerified => "source_verified",
            Self::SourceScanFinished => "source_scan_finished",
            Self::FilesystemChanged => "filesystem_changed",
        }
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
        self.refresh_persisted_metadata_tags_for_source(&source_id);
        self.prepare_similarity_for_source_automatically(&source_id, context);
        if selected_source {
            self.schedule_persisted_waveform_cache_indicator_refresh(context);
            self.schedule_active_folder_cache_warm(context);
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
