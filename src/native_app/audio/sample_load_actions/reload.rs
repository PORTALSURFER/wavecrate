use std::time::Instant;

use radiant::prelude as ui;

use crate::native_app::{
    app::{GuiMessage, NativeAppState, PendingSamplePlayback},
    audio::sample_load_actions::{foreground_sample_load_priority, types::SampleLoadStrategy},
};

impl NativeAppState {
    pub(in crate::native_app) fn reload_normalized_waveform(
        &mut self,
        reload: super::NormalizedWaveformReload<'_>,
        context: &mut ui::UiUpdateContext<GuiMessage>,
    ) {
        let reload_resumes_playback = reload.playback.is_some();
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
        self.log_sample_identity_checkpoint(
            "browser.normalize.reload_select",
            "reload_normalized_waveform",
            Some(reload.path),
            Some(if reload_resumes_playback {
                "resume_playback"
            } else {
                "reload_without_playback"
            }),
        );
        self.start_normalized_waveform_reload(reload.path.display().to_string(), context);
    }

    fn start_normalized_waveform_reload(
        &mut self,
        path: String,
        context: &mut ui::UiUpdateContext<GuiMessage>,
    ) {
        let started_at = Instant::now();
        self.log_sample_identity_checkpoint(
            "browser.normalize.reload_start",
            "start_normalized_waveform_reload",
            Some(std::path::Path::new(&path)),
            None,
        );
        self.yield_sample_cache_warm_for_foreground_load(context);
        self.cancel_inflight_sample_load();
        self.prepare_uncached_sample_load(&path, "normalization_reload_queued", started_at);
        self.start_sample_load_with_priority(
            path,
            false,
            context,
            foreground_sample_load_priority(),
            SampleLoadStrategy::Decode,
        );
    }
}
