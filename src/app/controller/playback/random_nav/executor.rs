use super::super::*;
use super::planner::RandomVisibleTarget;
use crate::app::controller::state::audio::PendingPlayback;
use crate::app::controller::state::history::RandomHistoryEntry;

pub(super) fn play_visible_target(
    controller: &mut AppController,
    target: RandomVisibleTarget,
    start_playback: bool,
) {
    controller.focus_browser_row_only(target.visible_row);
    if start_playback
        && let Err(err) = controller.play_audio(controller.ui.waveform.loop_enabled, None)
    {
        controller.set_status(err, StatusTone::Error);
    }
}

pub(super) fn play_history_entry(controller: &mut AppController, entry: RandomHistoryEntry) {
    if controller.selection_state.ctx.selected_source.as_ref() != Some(&entry.source_id) {
        controller
            .runtime
            .jobs
            .set_pending_playback(Some(PendingPlayback {
                source_id: entry.source_id.clone(),
                relative_path: entry.relative_path.clone(),
                looped: controller.ui.waveform.loop_enabled,
                start_override: None,
                force_loaded_audio: false,
            }));
        controller
            .runtime
            .jobs
            .set_pending_select_path(Some(entry.relative_path.clone()));
        controller.select_source_internal(Some(entry.source_id), Some(entry.relative_path));
        return;
    }
    if let Some(row) = controller.visible_row_for_path(&entry.relative_path) {
        controller.focus_browser_row_only(row);
    } else {
        controller.select_wav_by_path(&entry.relative_path);
    }
    if let Err(err) = controller.play_audio(controller.ui.waveform.loop_enabled, None) {
        controller.set_status(err, StatusTone::Error);
    }
}
