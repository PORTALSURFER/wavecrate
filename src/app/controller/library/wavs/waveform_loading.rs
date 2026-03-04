#![allow(clippy::too_many_arguments)]

use super::*;
use crate::app::controller::playback::audio_samples::{
    decode_samples_from_bytes, wav_bytes_from_samples,
};
use crate::app::state::WaveformView;
use std::sync::Arc;
use std::time::{Duration, Instant};

/// Shared decoded payload tuple returned from audio preparation.
type PreparedLoadedAudio = (Arc<DecodedWaveform>, Arc<[u8]>, bool);
/// Debounce window for deferring loaded-duration DB metadata persistence.
const LOADED_DURATION_METADATA_DEBOUNCE: Duration = Duration::from_millis(80);

impl AppController {
    pub(crate) fn load_waveform_for_selection(
        &mut self,
        source: &SampleSource,
        relative_path: &Path,
    ) -> Result<(), String> {
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
                self.mark_sample_missing(source, relative_path);
                self.show_missing_waveform_notice(relative_path);
                return Err(err);
            }
        };
        let bytes = match self.read_waveform_bytes(source, relative_path) {
            Ok(bytes) => bytes,
            Err(err) => {
                self.mark_sample_missing(source, relative_path);
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
        }
        self.finish_waveform_load_shared(
            source,
            relative_path,
            decoded,
            bytes,
            AudioLoadIntent::Selection,
            is_refresh,
            Some(transients),
        )?;
        self.maybe_trigger_pending_playback();
        let message = Self::loaded_status_text(relative_path, duration_seconds, sample_rate);
        self.set_status(message, StatusTone::Info);
        self.refresh_similarity_sort_for_loaded_sample();
        Ok(())
    }

    /// Finish applying a loaded waveform using shared immutable payloads.
    pub(crate) fn finish_waveform_load_shared(
        &mut self,
        source: &SampleSource,
        relative_path: &Path,
        decoded: Arc<DecodedWaveform>,
        bytes: Arc<[u8]>,
        intent: AudioLoadIntent,
        preserve_selections: bool,
        transients: Option<Arc<[f32]>>,
    ) -> Result<(), String> {
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

    /// Finish applying a loaded waveform using owned payloads.
    ///
    /// This compatibility path adapts legacy call sites to the shared immutable
    /// waveform pipeline and should be removed once all callers are Arc-first.
    pub(crate) fn finish_waveform_load(
        &mut self,
        source: &SampleSource,
        relative_path: &Path,
        decoded: DecodedWaveform,
        bytes: Arc<[u8]>,
        intent: AudioLoadIntent,
        preserve_selections: bool,
        transients: Option<Vec<f32>>,
    ) -> Result<(), String> {
        self.finish_waveform_load_shared(
            source,
            relative_path,
            Arc::new(decoded),
            bytes,
            intent,
            preserve_selections,
            transients.map(Arc::from),
        )
    }

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
            // Decode the stretched bytes to get the correct duration
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

    pub(crate) fn clear_waveform_selection(&mut self) {
        self.ui.waveform.playhead = PlayheadState::default();
        self.ui.waveform.selection = None;
        self.ui.waveform.selection_duration = None;
        self.ui.waveform.edit_selection = None;
        self.selection_state.range.clear();
        self.selection_state.edit_range.clear();
    }

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

    pub(crate) fn invalidate_cached_audio(&mut self, source_id: &SourceId, relative_path: &Path) {
        let key = CacheKey::new(source_id, relative_path);
        self.audio.cache.invalidate(&key);
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

    /// Queue source-analysis duration metadata persistence outside the load hot path.
    fn defer_loaded_sample_duration_metadata_write(
        &mut self,
        source: &SampleSource,
        relative_path: &Path,
        duration_seconds: f32,
        sample_rate: u32,
    ) {
        if !duration_seconds.is_finite() || duration_seconds <= 0.0 {
            return;
        }
        self.update_cached_duration_for_path(
            &source.id,
            relative_path,
            duration_seconds,
            sample_rate,
        );
        let long_sample_mark = (self.sample_view.wav.selected_wav.as_deref()
            == Some(relative_path))
        .then_some(duration_seconds > self.long_sample_threshold_seconds());
        if let Some(long_sample_mark) = long_sample_mark {
            self.update_cached_long_mark_for_path(&source.id, relative_path, long_sample_mark);
        }
        self.runtime.pending_loaded_duration_metadata = Some(
            crate::app::controller::state::runtime::PendingLoadedDurationMetadata {
                source_id: source.id.clone(),
                source_root: source.root.clone(),
                relative_path: relative_path.to_path_buf(),
                duration_seconds,
                sample_rate,
                long_sample_mark,
            },
        );
        self.runtime.pending_loaded_duration_metadata_not_before =
            Some(Instant::now() + LOADED_DURATION_METADATA_DEBOUNCE);
    }

    /// Return true when deferred loaded-duration metadata persistence is queued.
    pub(crate) fn has_pending_loaded_duration_metadata_write(&self) -> bool {
        self.runtime.pending_loaded_duration_metadata.is_some()
    }

    /// Flush deferred loaded-duration metadata persistence after debounce.
    pub(crate) fn flush_pending_loaded_duration_metadata_write(&mut self) {
        if self
            .runtime
            .pending_loaded_duration_metadata_not_before
            .is_some_and(|deadline| Instant::now() < deadline)
        {
            return;
        }
        self.runtime.pending_loaded_duration_metadata_not_before = None;
        let Some(pending) = self.runtime.pending_loaded_duration_metadata.take() else {
            return;
        };
        self.persist_loaded_duration_metadata(pending);
    }

    /// Persist deferred loaded-duration metadata to the source analysis database.
    fn persist_loaded_duration_metadata(
        &mut self,
        pending: crate::app::controller::state::runtime::PendingLoadedDurationMetadata,
    ) {
        let source = SampleSource {
            id: pending.source_id.clone(),
            root: pending.source_root.clone(),
        };
        let metadata = match self.current_file_metadata(&source, &pending.relative_path) {
            Ok(metadata) => metadata,
            Err(err) => {
                tracing::warn!(
                    "Failed to read file metadata for deferred duration update ({}): {err}",
                    pending.relative_path.display()
                );
                return;
            }
        };
        let sample_id = analysis_jobs::build_sample_id(source.id.as_str(), &pending.relative_path);
        let content_hash =
            analysis_jobs::fast_content_hash(metadata.file_size, metadata.modified_ns);
        let mut conn = match analysis_jobs::open_source_db(&source.root) {
            Ok(conn) => conn,
            Err(err) => {
                tracing::warn!(
                    "Failed to open source DB for deferred duration update ({}): {err}",
                    pending.relative_path.display()
                );
                return;
            }
        };
        if let Err(err) = analysis_jobs::upsert_samples(
            &mut conn,
            &[analysis_jobs::SampleMetadata {
                sample_id: sample_id.clone(),
                content_hash,
                size: metadata.file_size,
                mtime_ns: metadata.modified_ns,
            }],
        ) {
            tracing::warn!(
                "Failed to ensure analysis row for {}: {err}",
                pending.relative_path.display()
            );
        }
        if let Err(err) = analysis_jobs::update_sample_duration(
            &conn,
            &sample_id,
            pending.duration_seconds,
            pending.sample_rate,
        ) {
            tracing::warn!(
                "Failed to store duration metadata for {}: {err}",
                pending.relative_path.display()
            );
        }
        if let Some(long_sample_mark) = pending.long_sample_mark
            && let Err(err) =
                analysis_jobs::update_sample_long_mark(&conn, &sample_id, long_sample_mark)
        {
            tracing::warn!(
                "Failed to store long sample mark for {}: {err}",
                pending.relative_path.display()
            );
        }
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
        if let Some(value) = bpm {
            let rounded = value.round();
            if (value - rounded).abs() < 0.01 {
                self.ui.waveform.bpm_input = format!("{rounded:.0}");
            } else {
                self.ui.waveform.bpm_input = format!("{value:.2}");
            }
        } else {
            self.ui.waveform.bpm_input.clear();
        }
    }

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
            if let Err(err) = self.load_waveform_for_selection(source, relative_path) {
                self.set_status(err, StatusTone::Warning);
            } else {
                self.ui.waveform.view = preserved_view;
                self.refresh_waveform_image();
            }
        }
    }
}

#[cfg(test)]
/// Deferred loaded-duration metadata persistence tests.
mod tests {
    use super::*;
    use crate::app::controller::test_support::{
        prepare_with_source_and_wav_entries, sample_entry, write_test_wav,
    };
    use std::path::Path;

    /// Query persisted analysis duration metadata for one sample path.
    fn sample_duration_seconds(source: &SampleSource, relative_path: &Path) -> Option<f64> {
        let sample_id = analysis_jobs::build_sample_id(source.id.as_str(), relative_path);
        let conn = match analysis_jobs::open_source_db(&source.root) {
            Ok(conn) => conn,
            Err(_) => return None,
        };
        conn.query_row(
            "SELECT duration_seconds FROM samples WHERE sample_id = ?1",
            rusqlite::params![sample_id],
            |row| row.get::<_, Option<f64>>(0),
        )
        .ok()
        .flatten()
    }

    #[test]
    /// Loaded duration metadata writes should queue and only persist after deferred flush.
    fn loaded_duration_metadata_write_is_deferred_until_flush() {
        let (mut controller, source) = prepare_with_source_and_wav_entries(vec![sample_entry(
            "deferred.wav",
            crate::sample_sources::Rating::NEUTRAL,
        )]);
        let relative_path = Path::new("deferred.wav");
        let wav_path = source.root.join(relative_path);
        write_test_wav(&wav_path, &[0.0, 0.2, -0.2, 0.4]);

        let loaded = controller.load_waveform_for_selection(&source, relative_path);
        assert!(loaded.is_ok(), "waveform load failed: {loaded:?}");
        assert!(controller.has_pending_loaded_duration_metadata_write());
        assert!(sample_duration_seconds(&source, relative_path).is_none());

        controller
            .runtime
            .pending_loaded_duration_metadata_not_before =
            Some(Instant::now() - Duration::from_millis(1));
        controller.flush_pending_loaded_duration_metadata_write();

        assert!(!controller.has_pending_loaded_duration_metadata_write());
        let duration = sample_duration_seconds(&source, relative_path);
        assert!(duration.is_some(), "expected deferred duration metadata");
    }

    #[test]
    /// Deferred metadata flush should wait while its debounce deadline is still active.
    fn loaded_duration_metadata_flush_respects_deadline() {
        let (mut controller, source) = prepare_with_source_and_wav_entries(vec![sample_entry(
            "deadline.wav",
            crate::sample_sources::Rating::NEUTRAL,
        )]);
        let relative_path = Path::new("deadline.wav");
        let wav_path = source.root.join(relative_path);
        write_test_wav(&wav_path, &[0.0, 0.1, 0.2, 0.3]);

        let loaded = controller.load_waveform_for_selection(&source, relative_path);
        assert!(loaded.is_ok(), "waveform load failed: {loaded:?}");
        controller
            .runtime
            .pending_loaded_duration_metadata_not_before =
            Some(Instant::now() + Duration::from_secs(60));

        controller.flush_pending_loaded_duration_metadata_write();

        assert!(controller.has_pending_loaded_duration_metadata_write());
        assert!(sample_duration_seconds(&source, relative_path).is_none());
    }
}
