use super::super::*;

impl AppController {
    pub(crate) fn maybe_trigger_pending_playback(&mut self) {
        let Some(pending) = self.runtime.jobs.pending_playback() else {
            return;
        };
        let Some(audio) = self.sample_view.wav.loaded_audio.as_ref() else {
            return;
        };
        if audio.source_id != pending.source_id || audio.relative_path != pending.relative_path {
            return;
        }
        self.runtime.jobs.set_pending_playback(None);
        let playback = if pending.force_loaded_audio {
            crate::app::controller::playback::play_loaded_audio_for_path(
                self,
                &pending.source_id,
                &pending.relative_path,
                pending.looped,
                pending.start_override,
            )
        } else {
            self.play_audio(pending.looped, pending.start_override)
        };
        if let Err(err) = playback {
            self.set_status(err, StatusTone::Error);
        }
    }

    pub(crate) fn initial_waveform_render_spec(
        &self,
    ) -> crate::app::controller::library::wavs::waveform_rendering::InitialWaveformRenderSpec {
        crate::app::controller::library::wavs::waveform_rendering::InitialWaveformRenderSpec {
            size: self.sample_view.waveform.size,
            channel_view: self.ui.waveform.channel_view,
            transient_markers_enabled: self.ui.waveform.transient_markers_enabled,
        }
    }
}
