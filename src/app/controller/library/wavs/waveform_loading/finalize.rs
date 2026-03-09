use super::*;

/// Immutable payload and context required to finalize one shared waveform load.
pub(crate) struct FinishWaveformLoadShared<'a> {
    pub(crate) source: &'a SampleSource,
    pub(crate) relative_path: &'a Path,
    pub(crate) decoded: Arc<DecodedWaveform>,
    pub(crate) bytes: Arc<[u8]>,
    pub(crate) intent: AudioLoadIntent,
    pub(crate) preserve_selections: bool,
    pub(crate) transients: Option<Arc<[f32]>>,
}

impl AppController {
    /// Finish applying a loaded waveform using shared immutable payloads.
    pub(crate) fn finish_waveform_load_shared(
        &mut self,
        params: FinishWaveformLoadShared<'_>,
    ) -> Result<(), String> {
        let FinishWaveformLoadShared {
            source,
            relative_path,
            decoded,
            bytes,
            intent,
            preserve_selections,
            transients,
        } = params;
        let duration_seconds = decoded.duration_seconds;
        let sample_rate = decoded.sample_rate;
        self.apply_waveform_image_shared(decoded, transients);
        if !preserve_selections {
            self.ui.waveform.view = WaveformView::default();
            self.ui.waveform.cursor = Some(0.0);
            self.clear_waveform_selection();
        }
        self.ui.waveform.notice = None;
        self.ui.waveform.loading = None;
        self.clear_waveform_slices();
        self.runtime.jobs.set_pending_audio(None);
        self.sample_view.wav.loaded_wav = Some(relative_path.to_path_buf());
        self.set_ui_loaded_wav(Some(relative_path.to_path_buf()));
        self.sync_loaded_audio(source, relative_path, duration_seconds, sample_rate, bytes)?;
        if matches!(intent, AudioLoadIntent::Selection) {
            self.apply_loaded_sample_bpm(relative_path);
            self.apply_loaded_sample_loop_marker(source, relative_path);
        }
        Ok(())
    }

    /// Clear waveform selection state after a new load replaces the current sample.
    pub(crate) fn clear_waveform_selection(&mut self) {
        self.ui.waveform.playhead = PlayheadState::default();
        self.ui.waveform.selection = None;
        self.ui.waveform.selection_duration = None;
        self.ui.waveform.edit_selection = None;
        self.selection_state.range.clear();
        self.selection_state.edit_range.clear();
    }

    fn sync_loaded_audio(
        &mut self,
        source: &SampleSource,
        relative_path: &Path,
        duration_seconds: f32,
        sample_rate: u32,
        bytes: Arc<[u8]>,
    ) -> Result<(), String> {
        self.sample_view.wav.loaded_audio = Some(LoadedAudio {
            source_id: source.id.clone(),
            root: source.root.clone(),
            relative_path: relative_path.to_path_buf(),
            bytes: Arc::clone(&bytes),
            duration_seconds,
            sample_rate,
        });
        match self.ensure_player() {
            Ok(Some(player)) => {
                let mut player = player.borrow_mut();
                player.stop();
                player.set_audio(bytes, duration_seconds);
            }
            Ok(None) => {}
            Err(err) => self.set_status(err, StatusTone::Warning),
        }
        self.defer_loaded_sample_duration_metadata_write(
            source,
            relative_path,
            duration_seconds,
            sample_rate,
        );
        Ok(())
    }

    /// Apply cached source BPM metadata to waveform controls when locks allow updates.
    fn apply_loaded_sample_bpm(&mut self, relative_path: &Path) {
        if self.ui.waveform.bpm_lock_enabled || self.ui.waveform.bpm_stretch_enabled {
            return;
        }
        if let Some(bpm) = self.bpm_value_for_path(relative_path) {
            self.set_waveform_bpm_input(Some(bpm));
        }
    }

    fn apply_loaded_sample_loop_marker(&mut self, source: &SampleSource, relative_path: &Path) {
        if self.ui.waveform.loop_lock_enabled {
            return;
        }
        let looped = match self.database_for(source) {
            Ok(db) => match db.looped_for_path(relative_path) {
                Ok(Some(looped)) => looped,
                Ok(None) => false,
                Err(err) => {
                    tracing::warn!(
                        "Failed to load loop marker for {}: {err}",
                        relative_path.display()
                    );
                    return;
                }
            },
            Err(err) => {
                tracing::warn!("Failed to access database for loop marker: {err}");
                return;
            }
        };
        self.ui.waveform.loop_enabled = looped;
    }

    /// Update the waveform BPM input fields to match stored metadata.
    pub(crate) fn set_waveform_bpm_input(&mut self, bpm: Option<f32>) {
        let bpm = bpm.filter(|value| value.is_finite() && *value > 0.0);
        self.ui.waveform.bpm_value = bpm;
        self.ui.waveform.bpm_input = bpm
            .and_then(crate::app::controller::formatting::format_waveform_bpm_input)
            .unwrap_or_default();
    }

    /// Clear loaded audio state and the rendered waveform artifacts together.
    pub(crate) fn clear_loaded_audio_and_waveform_visuals(&mut self) {
        self.sample_view.wav.loaded_audio = None;
        self.sample_view.waveform.decoded = None;
        self.ui.waveform.image = None;
        self.ui.waveform.waveform_image_signature = None;
        self.projected_waveform_image_signature = None;
        self.projected_waveform_image = None;
        self.ui.waveform.playhead = PlayheadState::default();
        self.ui.waveform.selection = None;
        self.ui.waveform.selection_duration = None;
        self.ui.waveform.edit_selection = None;
        self.selection_state.range.clear();
        self.selection_state.edit_range.clear();
        self.clear_waveform_slices();
    }
}
