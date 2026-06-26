use super::*;

impl AppController {
    /// Capture a worker-safe selection export snapshot from the currently loaded sample.
    pub(crate) fn capture_selection_export_snapshot(
        &self,
        bounds: SelectionRange,
        target_tag: Option<crate::sample_sources::Rating>,
    ) -> Result<SelectionExportSnapshot, String> {
        let audio = self
            .sample_view
            .wav
            .loaded_audio
            .as_ref()
            .ok_or_else(|| "Load a sample first".to_string())?;
        let (looped, bpm) = self.selection_export_metadata();
        Ok(SelectionExportSnapshot {
            source_id: audio.source_id.clone(),
            source_root: audio.root.clone(),
            relative_path: audio.relative_path.clone(),
            bounds,
            source_duration_seconds: audio.duration_seconds,
            audio: build_selection_export_audio_payload(
                self.sample_view.waveform.decoded.as_ref(),
                Arc::clone(&audio.bytes),
            ),
            apply_edge_fades: self.settings.controls.auto_edge_fades_on_selection_exports,
            edge_fade_ms: self.settings.controls.anti_clip_fade_ms.max(0.0),
            write_format: self.settings.audio_write_format.clone(),
            target_tag,
            looped,
            bpm,
        })
    }

    pub(super) fn selection_export_metadata(&self) -> (bool, Option<f32>) {
        let looped = self.ui.waveform.loop_enabled;
        let bpm = self
            .ui
            .waveform
            .bpm_value
            .filter(|value| value.is_finite() && *value > 0.0);
        (looped, if looped { bpm } else { None })
    }

    pub(super) fn apply_auto_edge_fades_to_selection_export(
        &self,
        samples: &mut [f32],
        sample_rate: u32,
        channels: u16,
    ) {
        if !self.settings.controls.auto_edge_fades_on_selection_exports {
            return;
        }
        let fade_ms = self.settings.controls.anti_clip_fade_ms.max(0.0);
        let fade_duration = Duration::from_secs_f32(fade_ms / 1000.0);
        apply_short_edge_fades_to_clip(samples, channels as usize, sample_rate, fade_duration);
    }

    pub(super) fn next_selection_path_in_dir(&self, root: &Path, original: &Path) -> PathBuf {
        helpers::next_selection_path_in_dir(root, original)
    }

    /// Return the active waveform selection span suitable for clip export.
    ///
    /// Export should accept any non-empty active selection, even when it is
    /// narrower than the global normalized editing threshold used for drag and
    /// paint affordances on very long files.
    pub(super) fn active_waveform_selection_for_export(&self) -> Result<SelectionRange, String> {
        self.selection_state
            .range
            .range()
            .or(self.ui.waveform.selection)
            .filter(|range| !range.is_empty())
            .ok_or_else(|| "Create a selection first".to_string())
    }
}
