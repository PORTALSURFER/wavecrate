use radiant::prelude as ui;

use crate::native_app::app::{GuiMessage, NativeAppState};

impl NativeAppState {
    pub(super) fn apply_frame_message(&mut self, context: &mut ui::UpdateContext<GuiMessage>) {
        self.maybe_open_audio_player(context);
        self.maybe_startup_source_scan(context);
        self.maybe_run_pending_source_refresh(context);
        self.maybe_auto_load_startup_sample(context);
        self.maybe_start_waveform_cache_warm(context);
        self.maybe_start_active_folder_cache_warm(context);
        self.advance_frame();
    }
}
