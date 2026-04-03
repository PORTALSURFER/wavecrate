use super::super::*;
use super::*;
use crate::app::controller::jobs::{FileOpMessage, FileOpResult, SelectionEditCommitResult};
use crate::app::controller::library::selection_edits::buffer::load_selection_buffer;
use crate::app::controller::library::selection_edits::duplicate_cleanup::trim_cleanup_ranges_from_buffer;
use crate::selection::SelectionRange;
use std::sync::{Arc, atomic::AtomicBool};
use std::time::Duration;

/// Background worker operations for destructive in-place selection edits.
#[derive(Clone, Debug)]
pub(super) enum SelectionEditWorkerOp {
    Crop,
    Trim,
    Reverse,
    Fade { direction: FadeDirection },
    Normalize { edge_fade: Duration },
    ShortEdgeFades { fade_duration: Duration },
    RepairClicks,
    Mute,
    ApplySelectionFades { selection: SelectionRange },
    CleanupDuplicates { cleanup_ranges: Vec<SelectionRange> },
}

impl SelectionEditWorkerOp {
    fn apply(&self, buffer: &mut SelectionEditBuffer) -> Result<(), String> {
        match self {
            Self::Crop => crop_buffer(buffer),
            Self::Trim => trim_buffer(buffer),
            Self::Reverse => reverse_buffer(buffer),
            Self::Fade { direction } => {
                apply_directional_fade(
                    &mut buffer.samples,
                    buffer.channels,
                    buffer.start_frame,
                    buffer.end_frame,
                    *direction,
                );
                Ok(())
            }
            Self::Normalize { edge_fade } => normalize_selection(buffer, *edge_fade),
            Self::ShortEdgeFades { fade_duration } => {
                let selection_frames = buffer.end_frame.saturating_sub(buffer.start_frame);
                let fade_frames = edge_fade_frame_count(
                    buffer.sample_rate.max(1),
                    selection_frames,
                    *fade_duration,
                );
                if fade_frames == 0 {
                    return Err("Selection is too short for edge fades".into());
                }
                apply_edge_fades(
                    &mut buffer.samples,
                    buffer.channels,
                    buffer.start_frame,
                    buffer.end_frame,
                    fade_frames,
                );
                Ok(())
            }
            Self::RepairClicks => repair_clicks_buffer(buffer),
            Self::Mute => ops::mute_buffer(buffer),
            Self::ApplySelectionFades { selection } => {
                apply_selection_fades(SelectionFadeRequest {
                    samples: &mut buffer.samples,
                    channels: buffer.channels,
                    sample_rate: buffer.sample_rate,
                    start_frame: buffer.start_frame,
                    end_frame: buffer.end_frame,
                    selection_gain: selection.gain(),
                    fade_in: selection.fade_in(),
                    fade_out: selection.fade_out(),
                });
                Ok(())
            }
            Self::CleanupDuplicates { cleanup_ranges } => {
                trim_cleanup_ranges_from_buffer(buffer, cleanup_ranges)
            }
        }
    }
}

impl AppController {
    pub(super) fn queue_selection_edit_commit(
        &mut self,
        action_label: impl Into<String>,
        status_message: impl Into<String>,
        preserve_selection: bool,
        clear_duplicate_cleanup: bool,
        clear_edit_fades: bool,
        op: SelectionEditWorkerOp,
    ) -> Result<(), String> {
        if self.runtime.jobs.file_ops_in_progress() {
            return Err("File operation already in progress".to_string());
        }
        let target = self.selection_target()?;
        let action_label = action_label.into();
        let status_message = status_message.into();
        let visual = self.capture_selection_edit_visual_state();
        let playback = self.capture_playback_resume_state();
        self.begin_pending_file_mutation(&target.source.id, [target.relative_path.clone()]);
        self.set_status(
            format!("{} {}...", action_label, target.relative_path.display()),
            StatusTone::Busy,
        );
        let (tx, rx) = std::sync::mpsc::channel();
        let cancel = Arc::new(AtomicBool::new(false));
        self.runtime.jobs.start_file_ops(rx, cancel.clone());
        std::thread::spawn(move || {
            let result = run_selection_edit_commit_job(
                target,
                action_label,
                status_message,
                preserve_selection,
                visual,
                playback,
                clear_duplicate_cleanup,
                clear_edit_fades,
                op,
                cancel,
            );
            let _ = tx.send(FileOpMessage::Finished(FileOpResult::SelectionEditCommit(result)));
        });
        Ok(())
    }
}

fn run_selection_edit_commit_job(
    target: SelectionTarget,
    action_label: String,
    status_message: String,
    preserve_selection: bool,
    visual: SelectionEditVisualState,
    playback: PlaybackResumeState,
    clear_duplicate_cleanup: bool,
    clear_edit_fades: bool,
    op: SelectionEditWorkerOp,
    cancel: Arc<AtomicBool>,
) -> SelectionEditCommitResult {
    let cancelled = || SelectionEditCommitResult {
        source_id: target.source.id.clone(),
        relative_path: target.relative_path.clone(),
        absolute_path: target.absolute_path.clone(),
        action_label: action_label.clone(),
        status_message: status_message.clone(),
        preserve_selection,
        visual: visual.clone(),
        playback: playback.clone(),
        clear_duplicate_cleanup,
        clear_edit_fades,
        entry: None,
        backup: None,
        result: Err(String::from("Edit cancelled")),
    };
    if cancel.load(std::sync::atomic::Ordering::Relaxed) {
        return cancelled();
    }
    let result = (|| {
        let backup =
            crate::app::controller::undo::OverwriteBackup::capture_before(&target.absolute_path)?;
        let db = crate::sample_sources::SourceDatabase::open(&target.source.root)
            .map_err(|err| format!("Database unavailable: {err}"))?;
        let tag = db
            .tag_for_path(&target.relative_path)
            .map_err(|err| format!("Failed to read tag: {err}"))?
            .ok_or_else(|| "Sample not found in database".to_string())?;
        let last_played_at = db
            .last_played_at_for_path(&target.relative_path)
            .map_err(|err| format!("Failed to read playback age: {err}"))?;
        let looped = db
            .looped_for_path(&target.relative_path)
            .map_err(|err| format!("Failed to read loop marker: {err}"))?
            .unwrap_or(false);
        let mut buffer = load_selection_buffer(&target.absolute_path, target.selection)?;
        op.apply(&mut buffer)?;
        if buffer.samples.is_empty() {
            return Err("No audio data after edit".into());
        }
        write_service::write_buffer_to_path(&target.absolute_path, &buffer)?;
        let entry = write_service::sync_sample_entry(
            &db,
            &target.relative_path,
            &target.absolute_path,
            tag,
            last_played_at,
            looped,
        )?;
        backup.capture_after(&target.absolute_path)?;
        Ok((entry, backup))
    })();
    match result {
        Ok((entry, backup)) => SelectionEditCommitResult {
            source_id: target.source.id,
            relative_path: target.relative_path,
            absolute_path: target.absolute_path,
            action_label,
            status_message,
            preserve_selection,
            visual,
            playback,
            clear_duplicate_cleanup,
            clear_edit_fades,
            entry: Some(entry),
            backup: Some(backup),
            result: Ok(()),
        },
        Err(err) => SelectionEditCommitResult {
            source_id: target.source.id,
            relative_path: target.relative_path,
            absolute_path: target.absolute_path,
            action_label,
            status_message,
            preserve_selection,
            visual,
            playback,
            clear_duplicate_cleanup,
            clear_edit_fades,
            entry: None,
            backup: None,
            result: Err(err),
        },
    }
}
