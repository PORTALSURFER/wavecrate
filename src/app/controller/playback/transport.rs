use super::super::formatting::format_waveform_bpm_input;
use super::*;
use crate::app::state::FocusContext;
use crate::selection::SelectionEdge;
use std::time::{Duration, Instant};

const TRANSIENT_SNAP_RADIUS: f32 = 0.01;
const SELECTION_START_SNAP_RADIUS: f32 = 0.01;
const SELECTION_START_SNAP_VIEW_FRACTION: f32 = 0.03;
const SELECTION_START_SNAP_SECONDS: f32 = 0.1;
/// Debounce window before persisting live volume slider updates.
const VOLUME_PERSIST_DEBOUNCE: Duration = Duration::from_millis(120);
/// Debounce window for committing queued waveform seek playback updates.
///
/// This keeps drag-heavy seek interactions cheap by applying the final replay
/// seek shortly after pointer activity settles.
const WAVEFORM_SEEK_COMMIT_DEBOUNCE: Duration = Duration::from_millis(24);

pub(crate) fn start_selection_drag(controller: &mut AppController, position: f32) {
    controller.selection_state.bpm_scale_beats = None;
    controller.begin_selection_undo("Selection");
    let start = snap_selection_start(controller, position)
        .or_else(|| snap_to_transient(controller, position))
        .unwrap_or(position);
    let range = controller.selection_state.range.begin_new(start);
    controller.apply_selection(Some(range));
}

pub(crate) fn start_edit_selection_drag(controller: &mut AppController, position: f32) {
    let _ = controller.commit_edit_selection_fades();
    let start = snap_to_transient(controller, position).unwrap_or(position);
    let range = controller.selection_state.edit_range.begin_new(start);
    controller.apply_edit_selection(Some(range));
}

pub(crate) fn start_selection_edge_drag(
    controller: &mut AppController,
    edge: SelectionEdge,
    bpm_scale: bool,
) -> bool {
    if !controller.selection_state.range.begin_edge_drag(edge) {
        return false;
    }
    controller.begin_selection_undo("Selection");
    controller.selection_state.bpm_scale_beats = if bpm_scale {
        selection_scale_beats(controller)
    } else {
        None
    };
    controller.apply_selection(controller.selection_state.range.range());
    true
}

pub(crate) fn update_selection_drag(
    controller: &mut AppController,
    position: f32,
    snap_override: bool,
) {
    let range = if controller.selection_state.bpm_scale_beats.is_some() || snap_override {
        controller.selection_state.range.update_drag(position)
    } else if let Some(step) = bpm_snap_step(controller) {
        controller
            .selection_state
            .range
            .update_drag_snapped(position, step)
    } else {
        let snapped = snap_to_transient(controller, position).unwrap_or(position);
        controller.selection_state.range.update_drag(snapped)
    };
    if let Some(range) = range {
        controller.apply_selection(Some(range));
        if let Some(beats) = controller.selection_state.bpm_scale_beats {
            apply_scaled_bpm(controller, beats, range);
        }
    } else if controller.selection_state.range.range().is_none() {
        controller.apply_selection(None);
    }
}

pub(crate) fn update_edit_selection_drag(
    controller: &mut AppController,
    position: f32,
    snap_override: bool,
) {
    let range = if snap_override {
        controller.selection_state.edit_range.update_drag(position)
    } else {
        let snapped = snap_to_transient(controller, position).unwrap_or(position);
        controller.selection_state.edit_range.update_drag(snapped)
    };
    if let Some(range) = range {
        controller.apply_edit_selection(Some(range));
    } else if controller.selection_state.edit_range.range().is_none() {
        controller.apply_edit_selection(None);
    }
}

pub(crate) fn finish_selection_drag(controller: &mut AppController) {
    controller.selection_state.range.finish_drag();
    controller.selection_state.bpm_scale_beats = None;
    clear_too_small_bpm_selection(controller);
    controller.commit_selection_undo();
    let is_playing = controller
        .audio
        .player
        .as_ref()
        .map(|p| p.borrow().is_playing())
        .unwrap_or(false);
    if !is_playing || !controller.ui.waveform.loop_enabled {
        return;
    }
    let Some(selection) = controller
        .selection_state
        .range
        .range()
        .or(controller.ui.waveform.selection)
        .filter(|range| super::selection_meets_bpm_min_for_playback(controller, *range))
    else {
        return;
    };
    let playhead = controller.ui.waveform.playhead.position;
    let start_override = if playhead >= selection.start() && playhead <= selection.end() {
        Some(playhead)
    } else {
        Some(selection.start())
    };
    if let Err(err) = controller.play_audio(true, start_override) {
        controller.set_status(err, StatusTone::Error);
    }
}

pub(crate) fn finish_edit_selection_drag(controller: &mut AppController) {
    controller.selection_state.edit_range.finish_drag();
}

pub(crate) fn set_selection_range(controller: &mut AppController, range: SelectionRange) {
    controller.selection_state.range.set_range(Some(range));
    controller.apply_selection(Some(range));

    // If playing in non-looped mode, restart playback from the new selection start
    let is_playing = controller
        .audio
        .player
        .as_ref()
        .map(|p| p.borrow().is_playing())
        .unwrap_or(false);

    if is_playing
        && !controller.ui.waveform.loop_enabled
        && let Err(err) = controller.play_audio(false, Some(range.start()))
    {
        controller.set_status(err, StatusTone::Error);
    }
}

pub(crate) fn set_edit_selection_range(controller: &mut AppController, range: SelectionRange) {
    controller.selection_state.edit_range.set_range(Some(range));
    controller.apply_edit_selection(Some(range));
}

pub(crate) fn is_selection_dragging(controller: &AppController) -> bool {
    controller.selection_state.range.is_dragging()
}

pub(crate) fn is_edit_selection_dragging(controller: &AppController) -> bool {
    controller.selection_state.edit_range.is_dragging()
}

pub(crate) fn clear_selection(controller: &mut AppController) {
    let before = controller
        .selection_state
        .range
        .range()
        .or(controller.ui.waveform.selection);
    let cleared = controller.selection_state.range.clear();
    if cleared || controller.ui.waveform.selection.is_some() {
        controller.apply_selection(None);
        controller.push_selection_undo("Selection", before, None);
    }
}

pub(crate) fn clear_edit_selection(controller: &mut AppController) {
    let cleared = controller.selection_state.edit_range.clear();
    if cleared || controller.ui.waveform.edit_selection.is_some() {
        controller.apply_edit_selection(None);
    }
}

pub(crate) fn toggle_loop(controller: &mut AppController) {
    let state = flip_loop_toggle_state(controller);
    persist_loop_toggle_markers(controller, state);
    apply_loop_toggle_playback_policy(controller, state);
}

/// Snapshot loop state transition produced by one toggle action.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct LoopToggleState {
    /// Loop state before toggling.
    was_looping: bool,
    /// Loop state after toggling.
    loop_enabled: bool,
}

impl LoopToggleState {
    /// Return true when the toggle changed loop from disabled to enabled.
    fn toggled_to_enabled(self) -> bool {
        self.loop_enabled && !self.was_looping
    }

    /// Return true when the toggle changed loop from enabled to disabled.
    fn toggled_to_disabled(self) -> bool {
        !self.loop_enabled && self.was_looping
    }
}

/// Loop-playback follow-up behavior after toggling loop mode.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum LoopPlaybackPolicy {
    /// Restart loop playback if the player is currently active.
    RestartIfPlaying,
    /// Defer loop disable to cycle boundary to avoid abrupt mid-cycle stop.
    DeferDisableAfterCycle,
    /// No additional playback action needed.
    None,
}

/// Flip waveform loop state and return the before/after transition snapshot.
fn flip_loop_toggle_state(controller: &mut AppController) -> LoopToggleState {
    let was_looping = controller.ui.waveform.loop_enabled;
    controller.ui.waveform.loop_enabled = !was_looping;
    LoopToggleState {
        was_looping,
        loop_enabled: controller.ui.waveform.loop_enabled,
    }
}

/// Browser action rows used for multi-sample loop/BPM metadata writes.
struct LoopActionRows {
    /// Primary browser row (loaded sample when visible).
    primary_row: Option<usize>,
    /// Action rows (selection plus primary when needed).
    rows: Vec<usize>,
}

/// Resolve action rows targeted by loop metadata updates.
fn loop_action_rows(controller: &mut AppController) -> LoopActionRows {
    let loaded_path = controller
        .sample_view
        .wav
        .loaded_audio
        .as_ref()
        .map(|audio| audio.relative_path.clone());
    let primary_row = loaded_path
        .as_ref()
        .and_then(|path| controller.visible_row_for_path(path));
    let rows = primary_row
        .map(|row| controller.action_rows_from_primary(row))
        .unwrap_or_default();
    LoopActionRows { primary_row, rows }
}

/// Persist loop marker state to selected browser rows or loaded-sample fallback.
fn persist_loop_toggle_markers(controller: &mut AppController, state: LoopToggleState) {
    let action_rows = loop_action_rows(controller);
    if !action_rows.rows.is_empty() {
        persist_browser_loop_markers(controller, &action_rows, state);
    } else {
        persist_loaded_sample_loop_marker(controller, state.loop_enabled);
    }
}

/// Persist loop markers (and initial BPM when enabling) across targeted browser rows.
fn persist_browser_loop_markers(
    controller: &mut AppController,
    action_rows: &LoopActionRows,
    state: LoopToggleState,
) {
    let primary_row = action_rows.primary_row.unwrap_or(0);
    if let Err(err) = controller.set_loop_marker_browser_samples(
        &action_rows.rows,
        state.loop_enabled,
        primary_row,
    ) {
        tracing::warn!("Failed to update loop markers for browser samples: {err}");
    }
    if state.toggled_to_enabled()
        && let Some(bpm) = controller.ui.waveform.bpm_value
        && bpm.is_finite()
        && bpm > 0.0
        && let Err(err) = controller.set_bpm_browser_samples(&action_rows.rows, bpm, primary_row)
    {
        tracing::warn!("Failed to save BPM to browser samples: {err}");
    }
}

/// Persist loop marker state for the loaded sample when no browser rows are actionable.
fn persist_loaded_sample_loop_marker(controller: &mut AppController, loop_enabled: bool) {
    let loop_marker_update =
        controller
            .sample_view
            .wav
            .loaded_audio
            .as_ref()
            .and_then(|loaded_audio| {
                controller
                    .library
                    .sources
                    .iter()
                    .find(|source| source.id == loaded_audio.source_id)
                    .map(|source| (source.clone(), loaded_audio.relative_path.clone()))
            });
    if let Some((source, relative_path)) = loop_marker_update
        && let Err(err) =
            controller.set_sample_looped_for_source(&source, &relative_path, loop_enabled, false)
    {
        tracing::warn!("Failed to update loop marker: {err}");
    }
}

/// Apply loop-toggle playback policy (restart looping playback or defer disable).
fn apply_loop_toggle_playback_policy(controller: &mut AppController, state: LoopToggleState) {
    match loop_playback_policy(state) {
        LoopPlaybackPolicy::RestartIfPlaying => restart_loop_playback_if_playing(controller),
        LoopPlaybackPolicy::DeferDisableAfterCycle => {
            if let Err(err) = controller.defer_loop_disable_after_cycle() {
                controller.set_status(err, StatusTone::Error);
            }
        }
        LoopPlaybackPolicy::None => {}
    }
}

/// Determine playback follow-up required by one loop toggle transition.
fn loop_playback_policy(state: LoopToggleState) -> LoopPlaybackPolicy {
    if state.toggled_to_enabled() {
        LoopPlaybackPolicy::RestartIfPlaying
    } else if state.toggled_to_disabled() {
        LoopPlaybackPolicy::DeferDisableAfterCycle
    } else {
        LoopPlaybackPolicy::None
    }
}

/// Restart loop playback from progress/cursor/playhead context when currently playing.
fn restart_loop_playback_if_playing(controller: &mut AppController) {
    controller.audio.pending_loop_disable_at = None;
    let Some(player_rc) = controller.audio.player.as_ref().cloned() else {
        return;
    };
    let (is_playing, progress) = {
        let player_ref = player_rc.borrow();
        (player_ref.is_playing(), player_ref.progress())
    };
    if !is_playing {
        return;
    }
    let start_override = loop_restart_start_override(controller, progress);
    if let Err(err) = controller.play_audio(true, start_override) {
        controller.set_status(err, StatusTone::Error);
    }
}

/// Compute restart start position for loop-enabled playback based on active selection and cursor state.
fn loop_restart_start_override(controller: &AppController, progress: Option<f32>) -> Option<f32> {
    if has_loop_playback_selection(controller) {
        return None;
    }
    progress.or_else(|| {
        if controller.ui.waveform.playhead.visible {
            Some(controller.ui.waveform.playhead.position)
        } else {
            controller
                .ui
                .waveform
                .cursor
                .or(controller.ui.waveform.last_start_marker)
        }
    })
}

/// Return true when a valid playback selection should drive loop restart position.
fn has_loop_playback_selection(controller: &AppController) -> bool {
    controller
        .selection_state
        .range
        .range()
        .or(controller.ui.waveform.selection)
        .filter(|range| super::selection_meets_bpm_min_for_playback(controller, *range))
        .is_some()
}

pub(crate) fn seek_to(controller: &mut AppController, position: f32) {
    let looped = controller.ui.waveform.loop_enabled;
    record_play_start(controller, position);
    if let Err(err) = controller.play_audio(looped, Some(position)) {
        controller.set_status(err, StatusTone::Error);
    }
}

/// Queue a waveform seek request and defer playback restart to frame prep.
pub(crate) fn queue_waveform_seek_milli(controller: &mut AppController, position_milli: u16) {
    let clamped = position_milli.min(1000);
    controller.set_waveform_cursor_milli(clamped);
    controller.runtime.pending_waveform_seek_milli = Some(clamped);
    controller.runtime.pending_waveform_seek_not_before =
        Some(Instant::now() + WAVEFORM_SEEK_COMMIT_DEBOUNCE);
}

/// Flush a deferred waveform seek once its debounce window has elapsed.
pub(crate) fn flush_pending_waveform_seek_commit(controller: &mut AppController) {
    if controller
        .runtime
        .pending_waveform_seek_not_before
        .is_some_and(|deadline| Instant::now() < deadline)
    {
        return;
    }
    controller.runtime.pending_waveform_seek_not_before = None;
    let Some(position_milli) = controller.runtime.pending_waveform_seek_milli.take() else {
        return;
    };
    let normalized = (f32::from(position_milli) / 1000.0).clamp(0.0, 1.0);
    seek_to(controller, normalized);
    controller.set_waveform_cursor(normalized);
    controller.focus_waveform();
}

fn bpm_snap_step(controller: &AppController) -> Option<f32> {
    if !controller.ui.waveform.bpm_snap_enabled {
        return None;
    }
    let bpm = controller.ui.waveform.bpm_value?;
    if !bpm.is_finite() || bpm <= 0.0 {
        return None;
    }
    let duration = controller
        .sample_view
        .wav
        .loaded_audio
        .as_ref()
        .map(|audio| audio.duration_seconds)?;
    if !duration.is_finite() || duration <= 0.0 {
        return None;
    }
    let step = 60.0 / bpm / duration;
    if step.is_finite() && step > 0.0 {
        Some(step)
    } else {
        None
    }
}

fn clear_too_small_bpm_selection(controller: &mut AppController) {
    let Some(range) = controller.selection_state.range.range() else {
        return;
    };
    if super::selection_meets_bpm_min_for_playback(controller, range) {
        return;
    }
    controller.selection_state.range.set_range(None);
    controller.apply_selection(None);
}

fn snap_to_transient(controller: &AppController, position: f32) -> Option<f32> {
    if !controller.ui.waveform.transient_markers_enabled
        || !controller.ui.waveform.transient_snap_enabled
    {
        return None;
    }
    let mut closest = None;
    let mut best_distance = TRANSIENT_SNAP_RADIUS;
    for marker in controller.ui.waveform.transients.iter().copied() {
        let distance = (marker - position).abs();
        if distance <= best_distance {
            best_distance = distance;
            closest = Some(marker);
        }
    }
    closest
}

fn snap_selection_start(controller: &AppController, position: f32) -> Option<f32> {
    if !controller.ui.waveform.bpm_snap_enabled {
        return None;
    }
    let radius = selection_start_snap_radius(controller);
    if position.is_finite() && radius.is_finite() && radius > 0.0 && position <= radius {
        Some(0.0)
    } else {
        None
    }
}

fn selection_start_snap_radius(controller: &AppController) -> f32 {
    let mut radius = SELECTION_START_SNAP_RADIUS;
    let view_width = controller.ui.waveform.view.width();
    if view_width.is_finite() && view_width > 0.0 {
        radius = radius.min((view_width * SELECTION_START_SNAP_VIEW_FRACTION as f64) as f32);
    }
    if let Some(duration) = controller.loaded_audio_duration_seconds()
        && duration.is_finite()
        && duration > 0.0
    {
        radius = radius.min(SELECTION_START_SNAP_SECONDS / duration);
    }
    radius
}

fn selection_scale_beats(controller: &AppController) -> Option<f32> {
    if !controller.ui.waveform.bpm_snap_enabled {
        return None;
    }
    let bpm = controller.ui.waveform.bpm_value?;
    if !bpm.is_finite() || bpm <= 0.0 {
        return None;
    }
    let duration = controller.loaded_audio_duration_seconds()?;
    if !duration.is_finite() || duration <= 0.0 {
        return None;
    }
    let range = controller
        .selection_state
        .range
        .range()
        .or(controller.ui.waveform.selection)?;
    let seconds = range.width() * duration;
    if !seconds.is_finite() || seconds <= 0.0 {
        return None;
    }
    let beats = seconds * bpm / 60.0;
    if !beats.is_finite() || beats <= 0.0 {
        return None;
    }
    let rounded = beats.round();
    if (beats - rounded).abs() < 1.0e-3 {
        Some(rounded)
    } else {
        Some(beats)
    }
}

fn apply_scaled_bpm(controller: &mut AppController, beats: f32, range: SelectionRange) {
    if !beats.is_finite() || beats <= 0.0 {
        return;
    }
    let duration = match controller.loaded_audio_duration_seconds() {
        Some(duration) if duration.is_finite() && duration > 0.0 => duration,
        _ => return,
    };
    let seconds = range.width() * duration;
    if !seconds.is_finite() || seconds <= 0.0 {
        return;
    }
    let bpm = beats * 60.0 / seconds;
    if !bpm.is_finite() || bpm <= 0.0 {
        return;
    }
    controller.set_bpm_value(bpm);
    if let Some(input) = format_waveform_bpm_input(bpm) {
        controller.ui.waveform.bpm_input = input;
    }
}

pub(crate) fn replay_from_last_start(controller: &mut AppController) -> bool {
    if let Some(position) = controller.ui.waveform.last_start_marker {
        seek_to(controller, position);
        return true;
    }
    if let Some(cursor) = controller.ui.waveform.cursor {
        seek_to(controller, cursor);
        return true;
    }
    if controller.ui.waveform.playhead.visible {
        seek_to(controller, controller.ui.waveform.playhead.position);
        return true;
    }
    false
}

pub(crate) fn play_from_cursor(controller: &mut AppController) -> bool {
    if !controller.waveform_ready() {
        return false;
    }
    let cursor_from_navigation = match (
        controller.ui.waveform.cursor_last_hover_at,
        controller.ui.waveform.cursor_last_navigation_at,
    ) {
        (_, None) => false,
        (None, Some(_)) => true,
        (Some(hover), Some(nav)) => nav >= hover,
    };
    if cursor_from_navigation && let Some(cursor) = controller.ui.waveform.cursor {
        seek_to(controller, cursor);
        return true;
    }
    replay_from_last_start(controller)
}

pub(crate) fn record_play_start(controller: &mut AppController, position: f32) {
    let clamped = position.clamp(0.0, 1.0);
    controller.ui.waveform.last_start_marker = Some(clamped);
    controller.set_waveform_cursor(clamped);
}

/// Apply volume immediately and mark the setting dirty for deferred persistence.
pub(crate) fn set_volume_live(controller: &mut AppController, volume: f32) {
    let previous_milli = volume_to_milli(controller.ui.volume);
    controller.apply_volume(volume);
    let current_milli = volume_to_milli(controller.ui.volume);
    if previous_milli == current_milli {
        return;
    }
    if controller.runtime.last_persisted_volume_milli == Some(current_milli) {
        controller.runtime.volume_persist_dirty = false;
        controller.runtime.volume_persist_deadline = None;
        return;
    }
    controller.runtime.volume_persist_dirty = true;
    controller.runtime.volume_persist_deadline = Some(Instant::now() + VOLUME_PERSIST_DEBOUNCE);
}

/// Persist a dirty volume setting immediately.
pub(crate) fn commit_volume_setting(controller: &mut AppController) {
    if !controller.runtime.volume_persist_dirty {
        return;
    }
    let current_milli = volume_to_milli(controller.ui.volume);
    if controller.runtime.last_persisted_volume_milli == Some(current_milli) {
        controller.runtime.volume_persist_dirty = false;
        controller.runtime.volume_persist_deadline = None;
        return;
    }
    if let Err(err) = controller.persist_config("Failed to save volume") {
        controller.set_status(err, StatusTone::Error);
        controller.runtime.volume_persist_deadline = Some(Instant::now() + VOLUME_PERSIST_DEBOUNCE);
        return;
    }
    controller.runtime.volume_persist_dirty = false;
    controller.runtime.volume_persist_deadline = None;
    controller.runtime.last_persisted_volume_milli = Some(current_milli);
}

/// Flush deferred volume persistence once the debounce deadline elapses.
pub(crate) fn flush_pending_volume_setting(controller: &mut AppController) {
    if !controller.runtime.volume_persist_dirty {
        return;
    }
    let Some(deadline) = controller.runtime.volume_persist_deadline else {
        return;
    };
    if Instant::now() >= deadline {
        commit_volume_setting(controller);
    }
}

/// Convert normalized volume into stable milli units for equality checks.
fn volume_to_milli(volume: f32) -> u16 {
    (volume.clamp(0.0, 1.0) * 1000.0).round() as u16
}

pub(crate) fn toggle_play_pause(controller: &mut AppController) {
    let player_rc = match controller.ensure_player() {
        Ok(Some(p)) => p,
        Ok(None) => {
            controller.set_status("Audio unavailable", StatusTone::Error);
            return;
        }
        Err(err) => {
            controller.set_status(err, StatusTone::Error);
            return;
        }
    };
    let _is_playing = player_rc.borrow().is_playing();
    drop(player_rc);
    let _ = controller.play_audio(controller.ui.waveform.loop_enabled, None);
}

pub(crate) fn stop_playback_if_active(controller: &mut AppController) -> bool {
    controller.audio.pending_loop_disable_at = None;
    let Some(player_rc) = controller.audio.player.as_ref() else {
        return false;
    };
    let stopped = {
        let mut player = player_rc.borrow_mut();
        if player.is_playing() {
            player.stop();
            true
        } else {
            false
        }
    };
    if stopped {
        controller.hide_waveform_playhead();
    }
    stopped
}

pub(crate) fn handle_escape(controller: &mut AppController) {
    if controller.cancel_edit_selection_fades() {
        return;
    }
    let selection_active = controller.selection_state.range.range().is_some()
        || controller.ui.waveform.selection.is_some()
        || controller.selection_state.edit_range.range().is_some()
        || controller.ui.waveform.edit_selection.is_some();
    let stopped_playback = stop_playback_if_active(controller);
    if !(selection_active && stopped_playback) {
        clear_selection(controller);
        clear_edit_selection(controller);
    }
    let had_cursor = controller.ui.waveform.cursor.take().is_some();
    if had_cursor {
        controller.ui.waveform.cursor_last_hover_at = None;
        controller.ui.waveform.cursor_last_navigation_at = None;
        controller.ui.waveform.last_start_marker = Some(0.0);
    }
    if !controller.ui.browser.selected_paths.is_empty() {
        controller.clear_browser_selection();
    }
    if matches!(controller.ui.focus.context, FocusContext::SourceFolders) {
        controller.clear_folder_selection();
    }
}

#[cfg(test)]
/// Transport-focused regression tests for selection, loop, and seek behavior.
mod tests {
    use super::*;
    use crate::app::controller::test_support;

    #[test]
    /// Selection drags near zero should snap to exact start when BPM snapping is enabled.
    fn start_selection_drag_snaps_to_zero_with_bpm_snap() {
        let (mut controller, _source) = test_support::dummy_controller();
        controller.ui.waveform.bpm_snap_enabled = true;

        start_selection_drag(&mut controller, 0.005);

        let range = if let Some(range) = controller.selection_state.range.range() {
            range
        } else {
            panic!("selection range should be initialized");
        };
        assert!((range.start() - 0.0).abs() <= f32::EPSILON);
    }

    #[test]
    /// Enabling loop should clear any pending deferred loop-disable deadline.
    fn toggle_loop_enable_clears_pending_disable_deadline() {
        let (mut controller, _source) = test_support::dummy_controller();
        controller.audio.pending_loop_disable_at = Some(Instant::now() + Duration::from_secs(1));
        controller.ui.waveform.loop_enabled = false;

        toggle_loop(&mut controller);

        assert!(controller.ui.waveform.loop_enabled);
        assert!(controller.audio.pending_loop_disable_at.is_none());
    }

    #[test]
    /// Loop toggle playback policy should map to restart/disable/no-op transitions.
    fn loop_playback_policy_maps_toggle_transitions() {
        assert_eq!(
            loop_playback_policy(LoopToggleState {
                was_looping: false,
                loop_enabled: true,
            }),
            LoopPlaybackPolicy::RestartIfPlaying
        );
        assert_eq!(
            loop_playback_policy(LoopToggleState {
                was_looping: true,
                loop_enabled: false,
            }),
            LoopPlaybackPolicy::DeferDisableAfterCycle
        );
        assert_eq!(
            loop_playback_policy(LoopToggleState {
                was_looping: false,
                loop_enabled: false,
            }),
            LoopPlaybackPolicy::None
        );
    }

    #[test]
    /// Loop restart should keep span-based replay when a valid playback selection exists.
    fn loop_restart_start_override_uses_selection_span_when_available() {
        let (mut controller, _source) = test_support::dummy_controller();
        controller.ui.waveform.selection = Some(SelectionRange::new(0.2, 0.6));

        let start_override = loop_restart_start_override(&controller, Some(0.4));

        assert_eq!(start_override, None);
    }

    #[test]
    /// Loop restart start override should prefer progress then playhead then cursor/last-marker.
    fn loop_restart_start_override_priority_chain_is_stable() {
        let (mut controller, _source) = test_support::dummy_controller();

        let with_progress = loop_restart_start_override(&controller, Some(0.42));
        assert_eq!(with_progress, Some(0.42));

        controller.ui.waveform.playhead.visible = true;
        controller.ui.waveform.playhead.position = 0.37;
        let with_playhead = loop_restart_start_override(&controller, None);
        assert_eq!(with_playhead, Some(0.37));

        controller.ui.waveform.playhead.visible = false;
        controller.ui.waveform.cursor = Some(0.55);
        controller.ui.waveform.last_start_marker = Some(0.19);
        let with_cursor = loop_restart_start_override(&controller, None);
        assert_eq!(with_cursor, Some(0.55));

        controller.ui.waveform.cursor = None;
        let with_last_marker = loop_restart_start_override(&controller, None);
        assert_eq!(with_last_marker, Some(0.19));
    }

    #[test]
    /// Queued waveform seek requests should clamp milli input to the normalized range.
    fn queue_waveform_seek_milli_clamps_input() {
        let (mut controller, _source) = test_support::dummy_controller();

        queue_waveform_seek_milli(&mut controller, 1500);

        assert_eq!(controller.runtime.pending_waveform_seek_milli, Some(1000));
        assert!(
            controller
                .runtime
                .pending_waveform_seek_not_before
                .is_some()
        );
    }

    #[test]
    /// Deferred waveform seek commits should wait until the debounce deadline.
    fn flush_pending_waveform_seek_commit_waits_for_deadline() {
        let (mut controller, _source) = test_support::dummy_controller();
        queue_waveform_seek_milli(&mut controller, 500);
        controller.runtime.pending_waveform_seek_not_before =
            Some(Instant::now() + Duration::from_millis(50));

        flush_pending_waveform_seek_commit(&mut controller);

        assert_eq!(controller.runtime.pending_waveform_seek_milli, Some(500));
    }
}
