use super::super::*;

impl AppController {
    /// Queue background audio loading for browser-preview playback.
    ///
    /// The waveform panel now clears immediately for every new selection, but
    /// playback still waits for the newest background result only.
    pub(crate) fn queue_browser_preview_audio_load(
        &mut self,
        source: &SampleSource,
        relative_path: &Path,
        looped: bool,
    ) -> Result<(), String> {
        let pending_playback = PendingPlayback {
            source_id: source.id.clone(),
            relative_path: relative_path.to_path_buf(),
            looped,
            start_override: None,
            force_loaded_audio: false,
        };
        if self
            .runtime
            .jobs
            .pending_audio()
            .as_ref()
            .is_some_and(|pending| {
                pending.source_id == source.id && pending.relative_path == relative_path
            })
        {
            self.runtime
                .jobs
                .set_pending_playback(Some(pending_playback));
            return Ok(());
        }
        self.queue_audio_load_for(
            source,
            relative_path,
            AudioLoadIntent::Selection,
            Some(pending_playback),
        )
    }

    pub(crate) fn queue_audio_load_for(
        &mut self,
        source: &SampleSource,
        relative_path: &Path,
        intent: AudioLoadIntent,
        pending_playback: Option<PendingPlayback>,
    ) -> Result<(), String> {
        self.begin_audio_load_transition(relative_path, pending_playback);
        self.dispatch_audio_load_for(source, relative_path, intent)
    }

    /// Publish immediate loading-state changes for a newly selected sample.
    ///
    /// This clears stale waveform/audio state synchronously so the next frame
    /// shows the new loading target even when job dispatch is deferred.
    pub(crate) fn begin_audio_load_transition(
        &mut self,
        relative_path: &Path,
        pending_playback: Option<PendingPlayback>,
    ) {
        self.runtime.jobs.set_pending_audio(None);
        self.runtime.jobs.set_pending_playback(pending_playback);
        self.runtime.pending_waveform_render = None;
        self.runtime.pending_waveform_transient_compute = None;
        self.runtime.jobs.invalidate_waveform_render_requests();
        self.runtime.jobs.invalidate_waveform_transient_requests();
        self.ui.waveform.loading = Some(relative_path.to_path_buf());
        self.ui.waveform.waveform_image_signature = None;
        self.projected_waveform_image_signature = None;
        self.projected_waveform_image = None;
        self.ui.waveform.notice = None;
        self.sample_view.waveform.render_meta = None;
        self.sample_view.waveform.decoded = None;
        self.ui.waveform.image = None;
        self.ui.waveform.transients = Arc::from([]);
        self.ui.waveform.transient_cache_token = None;
        self.sample_view.wav.loaded_audio = None;
        self.sample_view.wav.loaded_wav = None;
        self.set_ui_loaded_wav(None);
        self.ui.waveform.last_bpm_grid_origin = 0.0;
        self.stop_playback_if_active();
        self.clear_waveform_selection();
        self.mark_waveform_projection_dirty();
        self.set_status(
            format!("Loading {}", relative_path.display()),
            StatusTone::Busy,
        );
    }

    /// Dispatch the heavy half of one prepared audio load request.
    pub(crate) fn dispatch_audio_load_for(
        &mut self,
        source: &SampleSource,
        relative_path: &Path,
        intent: AudioLoadIntent,
    ) -> Result<(), String> {
        let request_id = self.runtime.jobs.next_audio_request_id();
        let stretch_ratio = self.stretch_ratio_for_sample(relative_path);
        let pending = PendingAudio {
            request_id,
            source_id: source.id.clone(),
            root: source.root.clone(),
            relative_path: relative_path.to_path_buf(),
            intent,
        };
        let job = AudioLoadJob {
            request_id,
            source_id: source.id.clone(),
            root: source.root.clone(),
            relative_path: relative_path.to_path_buf(),
            stretch_ratio,
            render_spec: self.initial_waveform_render_spec(),
            prepared: None,
        };
        if self.try_queue_cached_audio_load(source, relative_path, intent)? {
            return Ok(());
        }
        if self.runtime.jobs.send_audio_job(job).is_err() {
            self.runtime.jobs.set_pending_audio(None);
            self.runtime.jobs.set_pending_playback(None);
            self.ui.waveform.loading = None;
            return Err("Failed to queue audio load".to_string());
        }
        self.runtime.jobs.set_pending_audio(Some(pending));
        Ok(())
    }
}
