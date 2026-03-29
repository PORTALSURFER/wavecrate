use super::super::undo;
use super::*;

impl AppController {
    /// Return the current exact-duplicate cleanup ranges, if any.
    pub(crate) fn exact_duplicate_cleanup_ranges(&self) -> Result<Vec<SelectionRange>, String> {
        if self.loaded_waveform_slice_export_in_progress() {
            return Err("Wait for the current slice export to finish".to_string());
        }
        if self.ui.waveform.slice_batch_profile != WaveformSliceBatchProfile::ExactDuplicateBeats {
            return Err("Run Exact Dedupe before cleaning duplicates".to_string());
        }
        if self.ui.waveform.slices.is_empty() {
            return Err("No duplicate cleanup ranges to apply".to_string());
        }
        Ok(self.ui.waveform.slices.clone())
    }

    /// Remove the current exact-duplicate cleanup ranges from the loaded sample.
    pub(crate) fn clean_exact_duplicate_beats(&mut self) -> Result<(), String> {
        let cleanup_ranges = self.exact_duplicate_cleanup_ranges()?;
        let removed_ranges = cleanup_ranges.len();
        let removed_beats = self.ui.waveform.slice_batch_beat_count.max(removed_ranges);
        let audio = self
            .sample_view
            .wav
            .loaded_audio
            .as_ref()
            .ok_or_else(|| "Load a sample before cleaning duplicates".to_string())?;
        let source = self
            .library
            .sources
            .iter()
            .find(|source| source.id == audio.source_id)
            .cloned()
            .ok_or_else(|| "Source not available for loaded sample".to_string())?;
        let target = SelectionTarget {
            source: source.clone(),
            relative_path: audio.relative_path.clone(),
            absolute_path: source.root.join(&audio.relative_path),
            selection: SelectionRange::new(0.0, 1.0),
        };
        let backup = undo::OverwriteBackup::capture_before(&target.absolute_path)?;
        let db = self
            .database_for(&target.source)
            .map_err(|err| format!("Database unavailable: {err}"))?;
        let tag = self.sample_tag_for(&target.source, &target.relative_path)?;
        let last_played_at = self.sample_last_played_for(&target.source, &target.relative_path)?;
        let looped = self.sample_looped_for(&target.source, &target.relative_path)?;
        let visual = self.capture_selection_edit_visual_state();
        let playback = self.capture_playback_resume_state();
        let write_outcome = apply_selection_edit_write(
            SelectionEditWriteRequest {
                target: &target,
                db: &db,
                tag,
                last_played_at,
                looped,
            },
            |buffer| trim_cleanup_ranges_from_buffer(buffer, &cleanup_ranges),
        )?;
        backup.capture_after(&target.absolute_path)?;

        self.update_cached_entry(&target.source, &target.relative_path, write_outcome.entry);
        self.clear_loaded_waveform_after_disk_edit();
        self.refresh_waveform_for_sample(&target.source, &target.relative_path);
        self.restore_selection_edit_visuals(false, visual);
        self.queue_selection_edit_playback(&target, &playback);
        self.maybe_trigger_pending_playback();
        self.push_undo_entry(self.selection_edit_undo_entry(
            format!("Cleaned duplicate beats {}", target.relative_path.display()),
            target.source.id.clone(),
            target.relative_path.clone(),
            target.absolute_path.clone(),
            backup,
        ));
        self.clear_waveform_slices();
        self.focus_waveform_context();
        self.set_status(
            format!("Removed {removed_beats} beat(s) across {removed_ranges} cleanup range(s)"),
            StatusTone::Info,
        );
        Ok(())
    }
}

fn trim_cleanup_ranges_from_buffer(
    buffer: &mut SelectionEditBuffer,
    cleanup_ranges: &[SelectionRange],
) -> Result<(), String> {
    let total_frames = buffer.samples.len() / buffer.channels.max(1);
    let frame_ranges = cleanup_ranges
        .iter()
        .copied()
        .map(|range| selection_frame_bounds(total_frames, range))
        .collect::<Vec<_>>();
    let mut merged = Vec::<(usize, usize)>::new();
    for (start, end) in frame_ranges {
        if let Some(last) = merged.last_mut()
            && start <= last.1
        {
            last.1 = last.1.max(end);
        } else {
            merged.push((start, end));
        }
    }
    if merged.is_empty() {
        return Err("No duplicate cleanup ranges to apply".to_string());
    }

    let channels = buffer.channels.max(1);
    let mut trimmed = Vec::with_capacity(buffer.samples.len());
    let mut cursor = 0usize;
    for (start_frame, end_frame) in merged {
        if start_frame > cursor {
            let start = cursor.saturating_mul(channels);
            let end = start_frame
                .saturating_mul(channels)
                .min(buffer.samples.len());
            trimmed.extend_from_slice(&buffer.samples[start..end]);
        }
        cursor = cursor.max(end_frame);
    }
    if cursor < total_frames {
        let start = cursor.saturating_mul(channels);
        trimmed.extend_from_slice(&buffer.samples[start..]);
    }
    if trimmed.is_empty() {
        return Err("Duplicate cleanup would remove the entire file".to_string());
    }
    buffer.samples = trimmed;
    Ok(())
}
