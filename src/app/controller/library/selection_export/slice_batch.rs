//! Waveform slice-batch export orchestration and completion handling.

use super::*;
use crate::app::controller::jobs::{
    PendingSliceBatchExport, SelectionExportJob, SelectionSliceBatchExportSnapshot,
    SelectionSliceBatchExportSuccess, build_selection_export_audio_payload,
};
use crate::app::state::{ProgressTaskKind, WaveformSliceBatchProfile};
use crate::sample_sources::Rating;

impl AppController {
    /// Return whether the currently loaded waveform owns the active slice-batch export.
    pub(crate) fn loaded_waveform_slice_export_in_progress(&self) -> bool {
        let Some(audio) = self.sample_view.wav.loaded_audio.as_ref() else {
            return false;
        };
        self.runtime
            .jobs
            .pending_slice_batch_export()
            .is_some_and(|pending| {
                pending.source_id == audio.source_id && pending.relative_path == audio.relative_path
            })
    }

    /// Queue the current waveform slice batch for background export.
    pub(crate) fn start_waveform_slice_batch_export(&mut self) -> Result<(), String> {
        self.start_waveform_slice_batch_export_with_tag(None)
    }

    pub(super) fn start_waveform_slice_batch_export_with_tag(
        &mut self,
        target_tag: Option<Rating>,
    ) -> Result<(), String> {
        if self.runtime.jobs.pending_slice_batch_export().is_some() {
            self.set_status("Slice export already in progress", StatusTone::Info);
            return Ok(());
        }
        let snapshot = self.capture_slice_batch_export_snapshot_with_tag(target_tag)?;
        let total = snapshot.slices.len();
        let request_id = self.runtime.jobs.next_selection_export_request_id();
        self.runtime
            .jobs
            .set_pending_slice_batch_export(Some(PendingSliceBatchExport {
                request_id,
                source_id: snapshot.source_id.clone(),
                relative_path: snapshot.relative_path.clone(),
            }));
        self.runtime
            .jobs
            .begin_selection_slice_batch_export(SelectionExportJob::SliceBatch {
                request_id,
                snapshot,
            });
        self.show_status_progress(
            ProgressTaskKind::SelectionExport,
            "Saving slices",
            total,
            false,
        );
        self.set_status("Saving slices...", StatusTone::Busy);
        Ok(())
    }

    /// Capture a worker-safe slice-batch export snapshot from the loaded waveform.
    pub(crate) fn capture_slice_batch_export_snapshot(
        &self,
    ) -> Result<SelectionSliceBatchExportSnapshot, String> {
        self.capture_slice_batch_export_snapshot_with_tag(None)
    }

    pub(super) fn capture_slice_batch_export_snapshot_with_tag(
        &self,
        target_tag: Option<Rating>,
    ) -> Result<SelectionSliceBatchExportSnapshot, String> {
        let audio = self
            .sample_view
            .wav
            .loaded_audio
            .as_ref()
            .ok_or_else(|| "Load a sample first".to_string())?;
        if self.ui.waveform.slices.is_empty() {
            return Err("No slices to export".to_string());
        }
        let slices = self.waveform_slice_export_ranges()?;
        Ok(SelectionSliceBatchExportSnapshot {
            source_id: audio.source_id.clone(),
            source_root: audio.root.clone(),
            relative_path: audio.relative_path.clone(),
            slices,
            profile: self.ui.waveform.slice_batch_profile,
            target_tag,
            audio: build_selection_export_audio_payload(
                self.sample_view.waveform.decoded.as_ref(),
                Arc::clone(&audio.bytes),
            ),
            apply_edge_fades: self.settings.controls.auto_edge_fades_on_selection_exports,
            edge_fade_ms: self.settings.controls.anti_clip_fade_ms.max(0.0),
        })
    }

    /// Apply one completed slice-batch export on the UI thread.
    pub(crate) fn apply_selection_slice_batch_export_success(
        &mut self,
        success: SelectionSliceBatchExportSuccess,
    ) {
        self.record_selection_export_timings(
            "slice_batch",
            &success.source_relative_path,
            success.timings,
        );
        let source = SampleSource {
            id: success.source_id.clone(),
            root: success.source_root.clone(),
        };
        for entry in &success.entries {
            self.insert_cached_entry(&source, entry.clone());
            self.enqueue_similarity_for_new_sample(
                &source,
                &entry.relative_path,
                entry.file_size,
                entry.modified_ns,
            );
        }

        if success.errors.is_empty() && self.loaded_waveform_matches_slice_batch(&success) {
            self.clear_waveform_slices();
        }

        let total = success.entries.len() + success.errors.len();
        if success.errors.is_empty() {
            self.set_status(
                format!("Saved {} slices", success.entries.len()),
                StatusTone::Info,
            );
        } else {
            let tone = if success.entries.is_empty() {
                StatusTone::Error
            } else {
                StatusTone::Warning
            };
            self.set_status(
                format!(
                    "Saved {} of {} slices ({} errors)",
                    success.entries.len(),
                    total,
                    success.errors.len()
                ),
                tone,
            );
        }
    }

    fn loaded_waveform_matches_slice_batch(
        &self,
        success: &SelectionSliceBatchExportSuccess,
    ) -> bool {
        self.sample_view
            .wav
            .loaded_audio
            .as_ref()
            .is_some_and(|audio| {
                audio.source_id == success.source_id
                    && audio.relative_path == success.source_relative_path
            })
    }
}

/// Resolve the next available slice-export path under the provided source root.
pub(super) fn next_slice_path_in_dir_for_root(
    root: &Path,
    original: &Path,
    profile: WaveformSliceBatchProfile,
    counter: &mut usize,
) -> PathBuf {
    let parent = original.parent().unwrap_or_else(|| Path::new(""));
    let stem = original
        .file_stem()
        .and_then(|s| s.to_str())
        .filter(|s| !s.is_empty())
        .unwrap_or("slice");
    let stem = match profile {
        WaveformSliceBatchProfile::Manual => strip_numbered_suffix(stem, "slice"),
        WaveformSliceBatchProfile::SilenceSplit => strip_numbered_suffix(stem, "silence_split"),
    };
    loop {
        let suffix = match profile {
            WaveformSliceBatchProfile::Manual => format!("slice{:03}", counter),
            WaveformSliceBatchProfile::SilenceSplit => format!("silence_split_{:03}", counter),
        };
        let candidate = parent.join(format!("{stem}_{suffix}.wav"));
        if !root.join(&candidate).exists() {
            *counter = counter.saturating_add(1);
            return candidate;
        }
        *counter = counter.saturating_add(1);
    }
}

fn strip_numbered_suffix<'a>(stem: &'a str, suffix: &str) -> &'a str {
    if let Some((prefix, tail)) = stem.rsplit_once(&format!("_{suffix}")) {
        if !prefix.is_empty() && !tail.is_empty() && tail.chars().all(|c| c.is_ascii_digit()) {
            return prefix;
        }
    }
    stem
}
