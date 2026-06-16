use crate::native_app::app::{NativeAppState, PendingSamplePlayback};

impl NativeAppState {
    pub(in crate::native_app) fn reload_normalized_waveform(
        &mut self,
        reload: super::NormalizedWaveformReload<'_>,
        context: &mut radiant::prelude::UiUpdateContext<crate::native_app::app::GuiMessage>,
    ) {
        if let Some(playback) = reload.playback {
            let (_, previous_end) = playback.span.unwrap_or((0.0, 1.0));
            let start = playback.start_ratio.clamp(0.0, 1.0);
            let end = previous_end.max(start).clamp(start, 1.0);
            self.audio.pending_sample_playback =
                Some(PendingSamplePlayback::ResumeNormalized { start, end });
        }
        self.library
            .folder_browser
            .select_file(reload.path.display().to_string());
        self.load_sample_without_autoplay(reload.path.display().to_string(), context);
    }
}
