/// Background worker implementation for non-blocking selection exports.
mod background;
/// Worker-side entry registration helpers for background selection exports.
mod background_recording;
/// UI-thread completion handlers for finished selection exports.
mod completion;
/// Helper routines shared by the selection-export workflow.
mod helpers;
/// Persistence and cache-registration steps for newly exported clips.
mod recording;
/// Typed export/registration request objects used by the selection-export workflow.
mod requests;
/// Waveform slice-batch export orchestration.
mod slice_batch;

pub(crate) use self::background::run_selection_export_job;
pub(crate) use self::background::run_slice_batch_export_job;
use self::helpers::{crop_selection_samples, next_selection_path_in_dir};
pub(crate) use self::requests::{SelectionClipExportRequest, SelectionEntryRecordRequest};
use super::selection_edits::apply_short_edge_fades_to_clip;
use super::*;
use std::time::Duration;

use crate::app::controller::jobs::{
    SelectionClipDestination, SelectionClipExportSuccess, SelectionCropExportSuccess,
    SelectionExportJob, SelectionExportSnapshot, SelectionExportTimings,
    build_selection_export_audio_payload,
};
use crate::app::controller::playback::audio_samples::write_wav;

impl AppController {
    /// Save the current waveform selection or accepted slices into the browser.
    ///
    /// This shares the same export path used by waveform drag-drop so keyboard and
    /// pointer workflows produce identical files and status updates.
    pub(crate) fn save_waveform_selection_or_slices_to_browser(
        &mut self,
        keep_source_focused: bool,
    ) -> Result<(), String> {
        match self.commit_edit_selection_fades() {
            Ok(true) => return Ok(()),
            Ok(false) => {}
            Err(err) => return Err(err),
        }
        if !self.ui.waveform.slices.is_empty() {
            self.start_waveform_slice_batch_export()?;
            return Ok(());
        }
        self.save_waveform_selection_to_browser(keep_source_focused)
    }

    /// Save the current waveform selection or slices and surface any failure via status UI.
    pub(crate) fn save_waveform_selection_or_slices_to_browser_action(
        &mut self,
        keep_source_focused: bool,
    ) {
        if let Err(err) = self.save_waveform_selection_or_slices_to_browser(keep_source_focused) {
            self.set_error_status(err);
        }
    }

    pub(crate) fn export_selection_clip(
        &mut self,
        request: SelectionClipExportRequest<'_>,
    ) -> Result<WavEntry, String> {
        let audio = self.selection_audio(request.source_id, request.relative_path)?;
        let source = self
            .library
            .sources
            .iter()
            .find(|s| &s.id == request.source_id)
            .cloned()
            .ok_or_else(|| "Source not available".to_string())?;
        let target_rel = self.next_selection_path_in_dir(&source.root, &audio.relative_path);
        let target_abs = source.root.join(&target_rel);
        let (mut samples, spec) = crop_selection_samples(&audio, request.bounds)?;
        self.apply_auto_edge_fades_to_selection_export(
            &mut samples,
            spec.sample_rate,
            spec.channels,
        );
        write_wav(&target_abs, &samples, spec.sample_rate, spec.channels)?;
        let (looped, bpm) = self.selection_export_metadata();
        self.record_selection_entry(SelectionEntryRecordRequest {
            source: &source,
            relative_path: target_rel,
            target_tag: request.target_tag,
            add_to_browser: request.add_to_browser,
            register_in_source: request.register_in_source,
            looped,
            bpm,
        })
    }

    pub(crate) fn export_selection_clip_in_folder(
        &mut self,
        request: SelectionClipExportRequest<'_>,
        folder: &Path,
    ) -> Result<WavEntry, String> {
        let audio = self.selection_audio(request.source_id, request.relative_path)?;
        let source = self
            .library
            .sources
            .iter()
            .find(|s| &s.id == request.source_id)
            .cloned()
            .ok_or_else(|| "Source not available".to_string())?;
        let name_hint = folder.join(
            audio
                .relative_path
                .file_name()
                .map(PathBuf::from)
                .unwrap_or_else(|| PathBuf::from("selection.wav")),
        );
        let target_rel = self.next_selection_path_in_dir(&source.root, &name_hint);
        let target_abs = source.root.join(&target_rel);
        let (mut samples, spec) = crop_selection_samples(&audio, request.bounds)?;
        self.apply_auto_edge_fades_to_selection_export(
            &mut samples,
            spec.sample_rate,
            spec.channels,
        );
        write_wav(&target_abs, &samples, spec.sample_rate, spec.channels)?;
        let (looped, bpm) = self.selection_export_metadata();
        self.record_selection_entry(SelectionEntryRecordRequest {
            source: &source,
            relative_path: target_rel,
            target_tag: request.target_tag,
            add_to_browser: request.add_to_browser,
            register_in_source: request.register_in_source,
            looped,
            bpm,
        })
    }

    pub(crate) fn save_waveform_selection_to_browser(
        &mut self,
        keep_source_focused: bool,
    ) -> Result<(), String> {
        let selection = self.active_waveform_selection_for_export()?;
        let folder_override = self
            .selection_state
            .ctx
            .selected_source
            .as_ref()
            .zip(self.sample_view.wav.loaded_audio.as_ref())
            .is_some_and(|(selected, audio)| selected == &audio.source_id)
            .then(|| {
                self.ui.sources.folders.focused.and_then(|idx| {
                    self.ui
                        .sources
                        .folders
                        .rows
                        .get(idx)
                        .map(|row| row.path.clone())
                })
            })
            .flatten()
            .filter(|path| !path.as_os_str().is_empty());
        let request_id = self.runtime.jobs.next_selection_export_request_id();
        self.begin_pending_sample_creation_transaction(
            crate::app::controller::history::PendingHistoryTransactionKey::SelectionExport {
                request_id,
            },
            "Saved selection clip",
        );
        self.runtime
            .jobs
            .begin_selection_export(SelectionExportJob::Clip {
                request_id,
                snapshot: self.capture_selection_export_snapshot(selection, None)?,
                destination: SelectionClipDestination::Browser {
                    keep_source_focused,
                    folder_override,
                },
            });
        self.record_waveform_selection_export_flash();
        self.set_status("Saving selection clip...", StatusTone::Busy);
        Ok(())
    }

    pub(crate) fn export_selection_clip_to_root(
        &mut self,
        request: SelectionClipExportRequest<'_>,
        clip_root: &Path,
        name_hint: &Path,
    ) -> Result<WavEntry, String> {
        let audio = self.selection_audio(request.source_id, request.relative_path)?;
        let target_rel = self.next_selection_path_in_dir(clip_root, name_hint);
        let target_abs = clip_root.join(&target_rel);
        let (mut samples, spec) = crop_selection_samples(&audio, request.bounds)?;
        self.apply_auto_edge_fades_to_selection_export(
            &mut samples,
            spec.sample_rate,
            spec.channels,
        );
        write_wav(&target_abs, &samples, spec.sample_rate, spec.channels)?;
        let source = SampleSource {
            id: SourceId::new(),
            root: clip_root.to_path_buf(),
        };
        // Clips saved outside sources are not inserted into browser or source DB.
        let (looped, bpm) = self.selection_export_metadata();
        self.record_selection_entry(SelectionEntryRecordRequest {
            source: &source,
            relative_path: target_rel,
            target_tag: request.target_tag,
            add_to_browser: false,
            register_in_source: false,
            looped,
            bpm,
        })
    }

    pub(crate) fn selection_audio(
        &self,
        source_id: &SourceId,
        relative_path: &Path,
    ) -> Result<LoadedAudio, String> {
        let Some(audio) = self.sample_view.wav.loaded_audio.as_ref() else {
            return Err("Selection audio not available; load a sample first".into());
        };
        if &audio.source_id != source_id || audio.relative_path != relative_path {
            return Err("Selection no longer matches the loaded sample".into());
        }
        Ok(audio.clone())
    }

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
            audio: build_selection_export_audio_payload(
                self.sample_view.waveform.decoded.as_ref(),
                Arc::clone(&audio.bytes),
            ),
            apply_edge_fades: self.settings.controls.auto_edge_fades_on_selection_exports,
            edge_fade_ms: self.settings.controls.anti_clip_fade_ms.max(0.0),
            target_tag,
            looped,
            bpm,
        })
    }

    fn selection_export_metadata(&self) -> (bool, Option<f32>) {
        let looped = self.ui.waveform.loop_enabled;
        let bpm = self
            .ui
            .waveform
            .bpm_value
            .filter(|value| value.is_finite() && *value > 0.0);
        (looped, if looped { bpm } else { None })
    }

    fn apply_auto_edge_fades_to_selection_export(
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

    fn next_selection_path_in_dir(&self, root: &Path, original: &Path) -> PathBuf {
        next_selection_path_in_dir(root, original)
    }

    /// Return the active waveform selection span suitable for clip export.
    ///
    /// Export should accept any non-empty active selection, even when it is
    /// narrower than the global normalized editing threshold used for drag and
    /// paint affordances on very long files.
    fn active_waveform_selection_for_export(&self) -> Result<SelectionRange, String> {
        self.selection_state
            .range
            .range()
            .or(self.ui.waveform.selection)
            .filter(|range| !range.is_empty())
            .ok_or_else(|| "Create a selection first".to_string())
    }

    /// Emit one optimistic submit token so native shells can blink the selection immediately.
    fn record_waveform_selection_export_flash(&mut self) {
        self.ui.waveform.selection_export_flash_nonce = self
            .ui
            .waveform
            .selection_export_flash_nonce
            .wrapping_add(1);
    }

    /// Emit one failure token so native shells can repaint the selection in an error color.
    pub(crate) fn record_waveform_selection_export_failure_flash(&mut self) {
        self.ui.waveform.selection_export_failure_flash_nonce = self
            .ui
            .waveform
            .selection_export_failure_flash_nonce
            .wrapping_add(1);
    }

    fn record_selection_export_timings(
        &self,
        action: &str,
        relative_path: &Path,
        timings: SelectionExportTimings,
    ) {
        tracing::debug!(
            "selection_export action={} path={} prepare_us={} write_us={} register_us={} total_us={}",
            action,
            relative_path.display(),
            timings.prepare.as_micros(),
            timings.write.as_micros(),
            timings.register.as_micros(),
            timings.total.as_micros()
        );
    }
}

#[cfg(test)]
mod selection_export_tests;
