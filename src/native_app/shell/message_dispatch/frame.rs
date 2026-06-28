use radiant::prelude as ui;

use crate::native_app::app::{GuiMessage, NativeAppState};

impl NativeAppState {
    pub(super) fn apply_frame_message(&mut self, context: &mut ui::UiUpdateContext<GuiMessage>) {
        self.maybe_install_application_icon();
        self.maybe_open_audio_player(context);
        self.maybe_startup_source_scan(context);
        self.maybe_run_pending_source_refresh(context);
        self.maybe_auto_load_startup_sample(context);
        self.maybe_start_release_update_check(context);
        self.maybe_start_waveform_cache_warm(context);
        self.maybe_start_active_folder_cache_warm(context);
        self.maybe_prepare_starmap_similarity_layout(context);
        self.flush_pending_play_selection_playback_retarget();
        self.advance_frame(context);
    }

    fn maybe_install_application_icon(&mut self) {
        if !self.ui.startup.app_icon_install_pending {
            return;
        }
        self.ui.startup.app_icon_install_pending = false;
        crate::native_app::shell::macos_app_icon::install_wavecrate_application_icon();
    }
}
