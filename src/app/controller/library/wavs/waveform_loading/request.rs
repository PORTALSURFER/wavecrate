use super::*;

impl AppController {
    /// Load the waveform for the active browser selection.
    ///
    /// A fresh selection load rebuilds waveform state from cache or disk and clears
    /// transient selection state. A same-path refresh is treated as a no-op reuse of
    /// the currently loaded sample so existing view and selection state stay intact.
    pub(crate) fn load_waveform_for_selection(
        &mut self,
        source: &SampleSource,
        relative_path: &Path,
    ) -> Result<(), String> {
        if self.warn_if_retained_delete_path_busy(&source.id, relative_path, "loading") {
            return Ok(());
        }
        let is_refresh = self.sample_view.wav.loaded_wav.as_deref() == Some(relative_path);
        if self.sample_view.wav.selected_wav.as_deref() != Some(relative_path) {
            self.sample_view.wav.selected_wav = Some(relative_path.to_path_buf());
        }
        if is_refresh {
            let message = self
                .loaded_audio_for(source, relative_path)
                .map(|audio| {
                    Self::loaded_status_text(
                        relative_path,
                        audio.duration_seconds,
                        audio.sample_rate,
                    )
                })
                .unwrap_or_else(|| format!("Loaded {}", relative_path.display()));
            self.set_status(message, StatusTone::Info);
            return Ok(());
        }
        if self.try_use_cached_audio(source, relative_path, AudioLoadIntent::Selection)? {
            return Ok(());
        }
        let metadata = match self.current_file_metadata(source, relative_path) {
            Ok(meta) => meta,
            Err(err) => {
                let _ = self.prune_missing_sample(source, relative_path);
                self.show_missing_waveform_notice(relative_path);
                return Err(err);
            }
        };
        let bytes = match self.read_waveform_bytes(source, relative_path) {
            Ok(bytes) => bytes,
            Err(err) => {
                let _ = self.prune_missing_sample(source, relative_path);
                self.show_missing_waveform_notice(relative_path);
                return Err(err);
            }
        };
        let (decoded, bytes, stretched) = self.prepare_loaded_audio(
            relative_path,
            None,
            bytes.into(),
            AudioLoadIntent::Selection,
        )?;
        let duration_seconds = decoded.duration_seconds;
        let sample_rate = decoded.sample_rate;
        let transients: Arc<[f32]> = crate::waveform::transients::detect_transients(
            decoded.as_ref(),
            super::waveform_rendering::DEFAULT_TRANSIENT_SENSITIVITY,
        )
        .into();
        let cache_key = CacheKey::new(&source.id, relative_path);
        if !stretched {
            self.audio.cache.insert(
                cache_key,
                metadata,
                decoded.clone(),
                bytes.clone(),
                transients.clone(),
            );
            self.persist_waveform_cache(&source.id, relative_path, metadata, &decoded, &transients);
        }
        self.finish_waveform_load_shared(FinishWaveformLoadShared {
            source,
            relative_path,
            decoded,
            bytes,
            intent: AudioLoadIntent::Selection,
            preserve_selections: is_refresh,
            transients: Some(transients),
        })?;
        self.maybe_trigger_pending_playback();
        let message = Self::loaded_status_text(relative_path, duration_seconds, sample_rate);
        self.set_status(message, StatusTone::Info);
        self.refresh_similarity_sort_for_loaded_sample();
        Ok(())
    }

    /// Decode bytes and optionally stretch them for the active waveform load request.
    pub(crate) fn prepare_loaded_audio(
        &mut self,
        relative_path: &Path,
        decoded: Option<Arc<DecodedWaveform>>,
        bytes: Arc<[u8]>,
        intent: AudioLoadIntent,
    ) -> Result<PreparedLoadedAudio, String> {
        let original_decoded = match decoded {
            Some(decoded) => decoded,
            None => Arc::new(
                self.sample_view
                    .renderer
                    .decode_from_bytes(&bytes)
                    .map_err(|err| err.to_string())?,
            ),
        };

        if matches!(intent, AudioLoadIntent::Selection)
            && let Some(ratio) = self.stretch_ratio_for_sample(relative_path)
        {
            let stretched = self.stretch_wav_bytes(&bytes, ratio)?;
            let stretched: Arc<[u8]> = stretched.into();
            let stretched_decoded = Arc::new(
                self.sample_view
                    .renderer
                    .decode_from_bytes(&stretched)
                    .map_err(|err| err.to_string())?,
            );
            return Ok((stretched_decoded, stretched, true));
        }

        Ok((original_decoded, bytes, false))
    }

    /// Resolve the active BPM-stretch ratio for one sample when stretch is enabled.
    pub(crate) fn stretch_ratio_for_sample(&mut self, relative_path: &Path) -> Option<f64> {
        if !self.ui.waveform.bpm_stretch_enabled {
            return None;
        }
        let target_bpm = self.ui.waveform.bpm_value?;
        if !target_bpm.is_finite() || target_bpm <= 0.0 {
            return None;
        }
        let source_bpm = self.bpm_value_for_path(relative_path)?;
        if !source_bpm.is_finite() || source_bpm <= 0.0 {
            return None;
        }
        let ratio = target_bpm as f64 / source_bpm as f64;
        if !ratio.is_finite() || (ratio - 1.0).abs() < 1e-3 {
            return None;
        }
        Some(ratio.clamp(0.5, 2.0))
    }

    fn stretch_wav_bytes(&self, bytes: &[u8], ratio: f64) -> Result<Vec<u8>, String> {
        let decoded = decode_samples_from_bytes(bytes)?;
        let channels = decoded.channels.max(1) as usize;
        let wsola = crate::audio::Wsola::new(decoded.sample_rate);
        let stretched = wsola.stretch(&decoded.samples, channels, ratio);
        wav_bytes_from_samples(&stretched, decoded.sample_rate, decoded.channels)
    }

    /// Format the status line shown after a waveform finishes loading.
    pub(crate) fn loaded_status_text(
        relative_path: &Path,
        duration_seconds: f32,
        sample_rate: u32,
    ) -> String {
        let duration_label = Self::format_duration(duration_seconds);
        let rate_label = Self::format_sample_rate(sample_rate);
        format!(
            "Loaded {} ({duration_label} @ {rate_label})",
            relative_path.display()
        )
    }

    fn loaded_audio_for(
        &self,
        source: &SampleSource,
        relative_path: &Path,
    ) -> Option<&LoadedAudio> {
        self.sample_view
            .wav
            .loaded_audio
            .as_ref()
            .filter(|audio| audio.source_id == source.id && audio.relative_path == relative_path)
    }

    fn format_duration(duration_seconds: f32) -> String {
        if !duration_seconds.is_finite() || duration_seconds <= 0.0 {
            return "0.00s".into();
        }
        if duration_seconds < 1.0 {
            return format!("{:.0} ms", duration_seconds * 1_000.0);
        }
        if duration_seconds < 60.0 {
            return format!("{:.2} s", duration_seconds);
        }
        let minutes = (duration_seconds / 60.0).floor() as u32;
        let seconds = duration_seconds - minutes as f32 * 60.0;
        format!("{minutes}m {seconds:05.2}s")
    }

    fn format_sample_rate(sample_rate: u32) -> String {
        if sample_rate == 0 {
            return "unknown".into();
        }
        if sample_rate >= 1_000 {
            return format!("{:.1} kHz", sample_rate as f32 / 1_000.0);
        }
        format!("{sample_rate} Hz")
    }

    /// Reload the active waveform after external content changes invalidate caches.
    ///
    /// This path preserves the current view window so edits and background refreshes
    /// do not snap the user back to the default zoom after a successful reload.
    pub(crate) fn reload_waveform_for_selection_if_active(
        &mut self,
        source: &SampleSource,
        relative_path: &Path,
    ) {
        self.invalidate_cached_audio(&source.id, relative_path);
        let loaded_matches = self
            .sample_view
            .wav
            .loaded_audio
            .as_ref()
            .is_some_and(|audio| {
                audio.source_id == source.id && audio.relative_path == relative_path
            });
        let selected_matches = self.selection_state.ctx.selected_source.as_ref()
            == Some(&source.id)
            && self.sample_view.wav.selected_wav.as_deref() == Some(relative_path);
        if selected_matches || loaded_matches {
            let preserved_view = self.ui.waveform.view;
            self.sample_view.wav.loaded_wav = None;
            self.set_ui_loaded_wav(None);
            if let Err(err) =
                self.queue_audio_load_for(source, relative_path, AudioLoadIntent::Selection, None)
            {
                self.set_status(err, StatusTone::Warning);
            } else {
                self.ui.waveform.view = preserved_view;
            }
        }
    }
}
