use super::*;
pub(crate) use super::{AppController, BPM_MIN_SELECTION_DIVISOR, StatusTone};
pub(crate) use crate::sample_sources::*;
pub(crate) use crate::selection::SelectionRange;

use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

pub(crate) mod audio_cache;
pub(crate) mod audio_loader;
pub(crate) mod audio_options;
pub(crate) mod audio_samples;
pub(crate) mod loop_crossfade;
pub(crate) mod recording;

mod browser_nav;
mod formatting;
mod player;
mod playhead_trail;
mod random_nav;
mod tagging;
mod transport;

#[cfg(test)]
mod audio_options_tests;

use formatting::{format_selection_duration, format_timestamp_hms_ms};
use tracing::warn;

#[cfg(test)]
const SHOULD_PLAY_RANDOM_SAMPLE: bool = false;
#[cfg(not(test))]
const SHOULD_PLAY_RANDOM_SAMPLE: bool = true;
const PLAYHEAD_COMPLETION_EPSILON: f32 = 0.001;
/// Equality epsilon used for normalized waveform cursor no-op detection.
const WAVEFORM_CURSOR_NOOP_EPSILON: f32 = 1.0e-6;
/// Equality epsilon used for waveform view no-op detection.
const WAVEFORM_VIEW_NOOP_EPSILON: f64 = 1.0e-9;
/// Debounce duration for deferred playback-age database writes.
const DEFERRED_PLAYBACK_AGE_COMMIT_DELAY: Duration = Duration::from_millis(160);

fn selection_meets_bpm_min(controller: &AppController, range: SelectionRange) -> bool {
    if !controller.ui.waveform.bpm_snap_enabled {
        return true;
    }
    let Some(min_seconds) = bpm_min_selection_seconds(controller) else {
        return true;
    };
    let Some(duration) = controller.loaded_audio_duration_seconds() else {
        return true;
    };
    if !min_seconds.is_finite() || min_seconds <= 0.0 {
        return true;
    }
    if !duration.is_finite() || duration <= 0.0 {
        return true;
    }
    let selection_seconds = range.width() * duration;
    let epsilon = min_seconds * 1.0e-3;
    selection_seconds + epsilon >= min_seconds
}

pub(crate) fn bpm_min_selection_seconds(controller: &AppController) -> Option<f32> {
    if !controller.ui.waveform.bpm_snap_enabled {
        return None;
    }
    let bpm = controller.ui.waveform.bpm_value?;
    if !bpm.is_finite() || bpm <= 0.0 {
        return None;
    }
    let beat = 60.0 / bpm;
    let min_seconds = beat / BPM_MIN_SELECTION_DIVISOR;
    if min_seconds.is_finite() && min_seconds > 0.0 {
        Some(min_seconds)
    } else {
        None
    }
}

pub(crate) fn selection_meets_bpm_min_for_playback(
    controller: &AppController,
    range: SelectionRange,
) -> bool {
    selection_meets_bpm_min(controller, range)
}

fn now_epoch_seconds() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs() as i64
}

impl AppController {
    /// Begin a selection drag gesture at the given position.
    pub fn start_selection_drag(&mut self, position: f32) {
        transport::start_selection_drag(self, position);
    }

    /// Begin a right-click edit selection drag on the waveform.
    pub fn start_edit_selection_drag(&mut self, position: f32) {
        transport::start_edit_selection_drag(self, position);
    }

    /// Begin dragging a selection edge, optionally scaling for BPM.
    pub fn start_selection_edge_drag(
        &mut self,
        edge: crate::selection::SelectionEdge,
        bpm_scale: bool,
    ) -> bool {
        transport::start_selection_edge_drag(self, edge, bpm_scale)
    }

    /// Update the active selection drag with the latest position.
    pub fn update_selection_drag(&mut self, position: f32, snap_override: bool) {
        transport::update_selection_drag(self, position, snap_override);
    }

    /// Update the in-progress edit selection drag with the latest cursor position.
    pub fn update_edit_selection_drag(&mut self, position: f32, snap_override: bool) {
        transport::update_edit_selection_drag(self, position, snap_override);
    }

    /// Finish the active selection drag gesture.
    pub fn finish_selection_drag(&mut self) {
        transport::finish_selection_drag(self);
    }

    /// Finish the edit selection drag and keep the edit selection active.
    pub fn finish_edit_selection_drag(&mut self) {
        transport::finish_edit_selection_drag(self);
    }

    /// Set the active selection range.
    pub fn set_selection_range(&mut self, range: SelectionRange) {
        transport::set_selection_range(self, range);
    }

    /// Replace the edit selection without a drag gesture.
    pub fn set_edit_selection_range(&mut self, range: SelectionRange) {
        transport::set_edit_selection_range(self, range);
    }

    /// True while a selection drag gesture is active.
    pub fn is_selection_dragging(&self) -> bool {
        transport::is_selection_dragging(self)
    }

    /// True while an edit selection drag gesture is active.
    pub fn is_edit_selection_dragging(&self) -> bool {
        transport::is_edit_selection_dragging(self)
    }

    /// Clear the active selection.
    pub fn clear_selection(&mut self) {
        transport::clear_selection(self);
    }

    /// Clear the edit selection while leaving playback selection intact.
    pub fn clear_edit_selection(&mut self) {
        transport::clear_edit_selection(self);
    }

    /// Toggle loop playback for the current selection.
    pub fn toggle_loop(&mut self) {
        transport::toggle_loop(self);
    }

    /// Seek playback to the given normalized position.
    pub fn seek_to(&mut self, position: f32) {
        transport::seek_to(self, position);
    }

    /// Seek waveform/playback using a 0..=1000 milli position from UI actions.
    pub fn seek_waveform_milli(&mut self, position_milli: u16) {
        let normalized = normalized_from_milli(position_milli);
        self.seek_to(normalized);
        self.set_waveform_cursor(normalized);
        self.focus_waveform();
    }

    /// Queue a waveform seek from UI actions and defer commit-side playback work.
    pub fn queue_waveform_seek_milli(&mut self, position_milli: u16) {
        transport::queue_waveform_seek_milli(self, position_milli);
    }

    /// Set waveform cursor using a 0..=1000 milli position from UI actions.
    pub fn set_waveform_cursor_milli(&mut self, position_milli: u16) {
        let normalized = normalized_from_milli(position_milli);
        let cursor_unchanged =
            self.ui.waveform.cursor.is_some_and(|existing| {
                (existing - normalized).abs() <= WAVEFORM_CURSOR_NOOP_EPSILON
            });
        if cursor_unchanged && waveform_focus_active(self) {
            return;
        }
        self.set_waveform_cursor(normalized);
        self.focus_waveform();
    }

    /// Set waveform selection range using 0..=1000 milli positions from UI actions.
    pub fn set_waveform_selection_range_milli(&mut self, start_milli: u16, end_milli: u16) {
        let next_range = selection_range_from_milli(start_milli, end_milli);
        let existing_range = self
            .selection_state
            .range
            .range()
            .or(self.ui.waveform.selection);
        if existing_range == Some(next_range) && waveform_focus_active(self) {
            return;
        }
        self.set_selection_range(next_range);
        self.focus_waveform();
    }

    /// Set waveform edit selection range using 0..=1000 milli positions from UI actions.
    pub fn set_waveform_edit_selection_range_milli(&mut self, start_milli: u16, end_milli: u16) {
        let next_range = selection_range_from_milli(start_milli, end_milli);
        let existing_range = self
            .selection_state
            .edit_range
            .range()
            .or(self.ui.waveform.edit_selection);
        if existing_range == Some(next_range) && waveform_focus_active(self) {
            return;
        }
        self.set_edit_selection_range(next_range);
        self.focus_waveform();
    }

    /// Set waveform edit fade-in handle using a 0..=1000 milli position from UI actions.
    pub fn set_waveform_edit_fade_in_end_milli(&mut self, position_milli: u16) {
        let Some(existing_range) = self
            .selection_state
            .edit_range
            .range()
            .or(self.ui.waveform.edit_selection)
        else {
            return;
        };
        let next_range = update_edit_fade_in_end_from_milli(existing_range, position_milli);
        if existing_range == next_range && waveform_focus_active(self) {
            return;
        }
        self.selection_state.edit_range.set_range(Some(next_range));
        self.apply_edit_selection(Some(next_range));
        self.focus_waveform();
    }

    /// Set waveform edit fade-out handle using a 0..=1000 milli position from UI actions.
    pub fn set_waveform_edit_fade_out_start_milli(&mut self, position_milli: u16) {
        let Some(existing_range) = self
            .selection_state
            .edit_range
            .range()
            .or(self.ui.waveform.edit_selection)
        else {
            return;
        };
        let next_range = update_edit_fade_out_start_from_milli(existing_range, position_milli);
        if existing_range == next_range && waveform_focus_active(self) {
            return;
        }
        self.selection_state.edit_range.set_range(Some(next_range));
        self.apply_edit_selection(Some(next_range));
        self.focus_waveform();
    }

    /// Clear waveform selection and keep waveform focus active.
    pub fn clear_waveform_selection_with_focus(&mut self) {
        self.clear_selection();
        self.focus_waveform();
    }

    /// Clear waveform edit selection and keep waveform focus active.
    pub fn clear_waveform_edit_selection_with_focus(&mut self) {
        self.clear_edit_selection();
        self.focus_waveform();
    }

    /// Zoom waveform from UI actions using clamped step counts and focus retention.
    pub fn zoom_waveform_steps_from_ui(&mut self, zoom_in: bool, steps: u8) {
        let before_view = self.ui.waveform.view;
        let focused_before = waveform_focus_active(self);
        self.zoom_waveform_steps_with_factor(
            zoom_in,
            zoom_steps_from_ui(steps),
            None,
            None,
            true,
            true,
        );
        if focused_before && !waveform_view_changed(before_view, self.ui.waveform.view) {
            return;
        }
        self.focus_waveform();
    }

    /// Zoom waveform to current selection while preserving waveform focus.
    pub fn zoom_waveform_to_selection_with_focus(&mut self) {
        self.zoom_waveform_to_selection();
        self.focus_waveform();
    }

    /// Reset waveform zoom to full range while preserving waveform focus.
    pub fn zoom_waveform_full_with_focus(&mut self) {
        self.zoom_waveform_full();
        self.focus_waveform();
    }

    /// Restart playback from the last recorded start position.
    pub fn replay_from_last_start(&mut self) -> bool {
        transport::replay_from_last_start(self)
    }

    /// Start playback from the current cursor position.
    pub fn play_from_cursor(&mut self) -> bool {
        transport::play_from_cursor(self)
    }

    /// Record the most recent play start position.
    pub fn record_play_start(&mut self, position: f32) {
        transport::record_play_start(self, position);
    }

    /// Apply the output volume multiplier immediately without persisting config.
    pub fn set_volume_live(&mut self, volume: f32) {
        transport::set_volume_live(self, volume);
    }

    /// Persist the current output volume setting if a live change is pending.
    pub fn commit_volume_setting(&mut self) {
        transport::commit_volume_setting(self);
    }

    /// Flush a pending debounced volume-setting persistence if due.
    pub(crate) fn flush_pending_volume_setting(&mut self) {
        transport::flush_pending_volume_setting(self);
    }

    /// Return true when a deferred volume-setting persistence write is queued.
    pub(crate) fn has_pending_volume_setting_flush(&self) -> bool {
        self.runtime.volume_persist_dirty
    }

    /// Flush a pending deferred waveform seek commit if due.
    pub(crate) fn flush_pending_waveform_seek_commit(&mut self) {
        transport::flush_pending_waveform_seek_commit(self);
    }

    /// Return true when a deferred waveform-seek commit is queued.
    pub(crate) fn has_pending_waveform_seek_commit(&self) -> bool {
        self.runtime.pending_waveform_seek_milli.is_some()
    }

    #[cfg(test)]
    /// Expose queued deferred waveform seek target for controller/runtime tests.
    pub(crate) fn pending_waveform_seek_milli_for_test(&self) -> Option<u16> {
        self.runtime.pending_waveform_seek_milli
    }

    /// Toggle between play and pause.
    pub fn toggle_play_pause(&mut self) {
        transport::toggle_play_pause(self);
    }

    /// Stop playback if it is currently active.
    pub fn stop_playback_if_active(&mut self) -> bool {
        transport::stop_playback_if_active(self)
    }

    /// Handle escape key behavior for playback and selection.
    pub fn handle_escape(&mut self) {
        transport::handle_escape(self);
    }

    /// Start playback with the provided loop and start override settings.
    pub fn play_audio(&mut self, looped: bool, start_override: Option<f32>) -> Result<(), String> {
        player::play_audio(self, looped, start_override)
    }

    /// Record playback for the currently loaded audio, updating caches and UI.
    pub(crate) fn record_loaded_audio_playback(&mut self) {
        let Some(audio) = self.sample_view.wav.loaded_audio.as_ref() else {
            return;
        };
        let source_id = audio.source_id.clone();
        let root = audio.root.clone();
        let relative_path = audio.relative_path.clone();
        let played_at = now_epoch_seconds();

        self.audio.pending_age_update = Some(PendingAgeUpdate {
            source_id: source_id.clone(),
            root,
            relative_path: relative_path.clone(),
            played_at,
        });

        if self.selection_state.ctx.selected_source.as_ref() == Some(&source_id)
            && let Some(index) = self.wav_index_for_path(&relative_path)
        {
            let _ = self.ensure_wav_page_loaded(index);
            if let Some(entry) = self.wav_entries.entry_mut(index) {
                entry.last_played_at = Some(played_at);
            }
        }
        if let Some(cache) = self.cache.wav.entries.get_mut(&source_id)
            && let Some(index) = cache.lookup.get(&relative_path).copied()
            && let Some(entry) = cache.entry_mut(index)
        {
            entry.last_played_at = Some(played_at);
        }
    }

    /// Defer pending playback-age persistence when focus changes away from a path.
    pub(crate) fn defer_pending_age_update_commit_if_path_changes(&mut self, next_path: &Path) {
        let should_defer = self
            .audio
            .pending_age_update
            .as_ref()
            .is_some_and(|update| update.relative_path != next_path);
        if !should_defer {
            return;
        }
        self.runtime.pending_age_update_commit = self.audio.pending_age_update.take();
        self.runtime.pending_age_update_commit_not_before =
            Some(Instant::now() + DEFERRED_PLAYBACK_AGE_COMMIT_DELAY);
    }

    /// Flush any deferred playback-age update persistence request.
    pub(crate) fn flush_pending_age_update_commit(&mut self) {
        if self
            .runtime
            .pending_age_update_commit_not_before
            .is_some_and(|deadline| Instant::now() < deadline)
        {
            return;
        }
        self.runtime.pending_age_update_commit_not_before = None;
        let Some(update) = self.runtime.pending_age_update_commit.take() else {
            return;
        };
        self.commit_pending_age_update_value(update);
    }

    /// Return true when deferred playback-age persistence is queued.
    pub(crate) fn has_pending_age_update_commit(&self) -> bool {
        self.runtime.pending_age_update_commit.is_some()
    }

    /// Commit any pending playback age update to the database and refresh the UI.
    pub fn commit_pending_age_update(&mut self) {
        let Some(update) = self.audio.pending_age_update.take() else {
            return;
        };
        self.commit_pending_age_update_value(update);
    }

    /// Persist one playback-age update payload and refresh browser lists when needed.
    fn commit_pending_age_update_value(&mut self, update: PendingAgeUpdate) {
        let source = SampleSource {
            id: update.source_id.clone(),
            root: update.root,
        };
        match self.database_for(&source) {
            Ok(db) => {
                if let Err(err) = db.set_last_played_at(&update.relative_path, update.played_at) {
                    warn!(
                        "Failed to update playback age for {}: {}",
                        update.relative_path.display(),
                        err
                    );
                }
            }
            Err(err) => {
                warn!(
                    "Database unavailable for playback age update {}: {}",
                    update.relative_path.display(),
                    err
                );
            }
        }
        if self.selection_state.ctx.selected_source.as_ref() == Some(&update.source_id) {
            self.rebuild_browser_lists();
        }
    }

    /// Return true if audio playback is currently active.
    pub fn is_playing(&self) -> bool {
        player::is_playing(self)
    }

    /// Advance the playhead position based on playback progress.
    pub fn tick_playhead(&mut self) {
        player::tick_playhead(self);
    }

    #[cfg(test)]
    pub(crate) fn update_playhead_from_progress(
        &mut self,
        progress: Option<f32>,
        is_looping: bool,
    ) {
        player::update_playhead_from_progress(self, progress, is_looping, false);
    }

    pub(crate) fn hide_waveform_playhead(&mut self) {
        player::hide_waveform_playhead(self);
    }

    #[cfg(test)]
    pub(crate) fn playhead_completed_span_for_tests(
        &self,
        progress: f32,
        is_looping: bool,
    ) -> bool {
        player::playhead_completed_span_for_tests(self, progress, is_looping)
    }

    #[cfg(test)]
    pub(crate) fn hide_waveform_playhead_for_tests(&mut self) {
        player::hide_waveform_playhead_for_tests(self);
    }

    pub(crate) fn apply_selection(&mut self, range: Option<SelectionRange>) {
        player::apply_selection(self, range);
    }

    pub(crate) fn apply_edit_selection(&mut self, range: Option<SelectionRange>) {
        player::apply_edit_selection(self, range);
    }

    /// Update the hover time indicator for the waveform.
    pub fn update_waveform_hover_time(&mut self, position: Option<f32>) {
        player::update_waveform_hover_time(self, position);
    }

    #[cfg(test)]
    pub(crate) fn selection_duration_label(&self, range: SelectionRange) -> Option<String> {
        player::selection_duration_label(self, range)
    }

    pub(crate) fn apply_volume(&mut self, volume: f32) {
        player::apply_volume(self, volume);
    }

    pub(crate) fn ensure_player(&mut self) -> Result<Option<Rc<RefCell<AudioPlayer>>>, String> {
        player::ensure_player(self)
    }

    pub(crate) fn defer_loop_disable_after_cycle(&mut self) -> Result<(), String> {
        player::defer_loop_disable_after_cycle(self)
    }

    /// Tag the focused/selected wavs and keep the current focus.
    pub fn tag_selected(&mut self, target: crate::sample_sources::Rating) {
        tagging::tag_selected(self, target);
    }

    /// Move selection within the current sample browser list by an offset and play.
    pub fn nudge_selection(&mut self, offset: isize) {
        browser_nav::nudge_selection(self, offset);
    }

    /// Extend selection with shift navigation while keeping the current focus for playback.
    pub fn grow_selection(&mut self, offset: isize) {
        browser_nav::grow_selection(self, offset);
    }

    /// Jump to a random visible sample in the browser and start playback.
    pub fn play_random_visible_sample(&mut self) {
        random_nav::play_random_visible_sample(self);
    }

    #[cfg(test)]
    pub(crate) fn play_random_visible_sample_with_seed(&mut self, seed: u64) {
        random_nav::play_random_visible_sample_with_seed(self, seed);
    }

    /// Focus a random visible sample without starting playback (used for navigation flows).
    pub fn focus_random_visible_sample(&mut self) {
        random_nav::focus_random_visible_sample(self);
    }

    /// Play the previous entry from the random history stack.
    pub fn play_previous_random_sample(&mut self) {
        random_nav::play_previous_random_sample(self);
    }

    /// Toggle sticky random navigation for Up/Down in the browser.
    pub fn toggle_random_navigation_mode(&mut self) {
        random_nav::toggle_random_navigation_mode(self);
    }

    /// Return whether sticky random navigation mode is enabled.
    pub fn random_navigation_mode_enabled(&self) -> bool {
        random_nav::random_navigation_mode_enabled(self)
    }

    /// Cycle the triage flag filter (-1 left, +1 right) to mirror old column navigation.
    pub fn move_selection_column(&mut self, delta: isize) {
        tagging::move_selection_column(self, delta);
    }

    /// Tag leftwards: Keep -> Neutral, otherwise -> Trash.
    pub fn tag_selected_left(&mut self) {
        tagging::tag_selected_left(self);
    }

    /// Adjust rating for selected items by a delta (-3 to 3 relative change).
    pub fn adjust_selected_rating(&mut self, delta: i8) {
        tagging::adjust_selected_rating(self, delta);
    }
}

fn normalized_from_milli(value: u16) -> f32 {
    (value.min(1000) as f32) / 1000.0
}

fn selection_range_from_milli(start_milli: u16, end_milli: u16) -> SelectionRange {
    SelectionRange::new(
        normalized_from_milli(start_milli),
        normalized_from_milli(end_milli),
    )
}

/// Update edit fade-in length from one absolute waveform milli handle position.
fn update_edit_fade_in_end_from_milli(
    range: SelectionRange,
    position_milli: u16,
) -> SelectionRange {
    let width = range.width();
    if width <= 0.0 {
        return range;
    }
    let start = range.start();
    let end = range.end();
    let clamped_position = normalized_from_milli(position_milli).clamp(start, end);
    let length = ((clamped_position - start) / width).clamp(0.0, 1.0);
    let curve = range.fade_in().map(|fade| fade.curve).unwrap_or(0.5);
    range.with_fade_in(length, curve)
}

/// Update edit fade-out length from one absolute waveform milli handle position.
fn update_edit_fade_out_start_from_milli(
    range: SelectionRange,
    position_milli: u16,
) -> SelectionRange {
    let width = range.width();
    if width <= 0.0 {
        return range;
    }
    let start = range.start();
    let end = range.end();
    let clamped_position = normalized_from_milli(position_milli).clamp(start, end);
    let length = ((end - clamped_position) / width).clamp(0.0, 1.0);
    let curve = range.fade_out().map(|fade| fade.curve).unwrap_or(0.5);
    range.with_fade_out(length, curve)
}

fn zoom_steps_from_ui(steps: u8) -> u32 {
    u32::from(steps.max(1))
}

/// Return whether waveform focus is already active.
fn waveform_focus_active(controller: &AppController) -> bool {
    controller.ui.focus.context == crate::app::state::FocusContext::Waveform
}

/// Return whether two waveform views differ enough to warrant follow-up focus work.
fn waveform_view_changed(
    before: crate::app::state::WaveformView,
    after: crate::app::state::WaveformView,
) -> bool {
    (before.start - after.start).abs() > WAVEFORM_VIEW_NOOP_EPSILON
        || (before.end - after.end).abs() > WAVEFORM_VIEW_NOOP_EPSILON
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::controller::state::audio::PendingAgeUpdate;
    use crate::app::controller::test_support;
    use std::path::Path;
    use std::path::PathBuf;

    #[test]
    fn selection_duration_label_uses_loaded_audio() {
        let (mut controller, source) = test_support::dummy_controller();
        controller.sample_view.wav.loaded_audio = Some(LoadedAudio {
            source_id: source.id.clone(),
            root: source.root.clone(),
            relative_path: PathBuf::from("clip.wav"),
            bytes: Vec::new().into(),
            duration_seconds: 4.0,
            sample_rate: 48_000,
        });
        let label = controller.selection_duration_label(SelectionRange::new(0.25, 0.75));
        assert_eq!(label.as_deref(), Some("2.00 s"));
    }

    #[test]
    fn selection_duration_label_is_absent_without_audio() {
        let (controller, _) = test_support::dummy_controller();
        let label = controller.selection_duration_label(SelectionRange::new(0.0, 1.0));
        assert!(label.is_none());
    }

    #[test]
    fn playhead_progress_updates_position_without_play_state() {
        let (mut controller, _source) = test_support::dummy_controller();

        controller.update_playhead_from_progress(Some(0.42), false);

        assert!(controller.ui.waveform.playhead.visible);
        assert!((controller.ui.waveform.playhead.position - 0.42).abs() < 0.0001);
    }

    #[test]
    fn playhead_progress_completion_hides_playhead() {
        let (mut controller, _source) = test_support::dummy_controller();
        controller.ui.waveform.playhead.active_span_end = Some(1.0);

        controller.update_playhead_from_progress(Some(0.9995), false);

        assert!(!controller.ui.waveform.playhead.visible);
        assert!(controller.ui.waveform.playhead.active_span_end.is_none());
    }

    #[test]
    fn normalized_from_milli_clamps_bounds() {
        assert_eq!(normalized_from_milli(0), 0.0);
        assert_eq!(normalized_from_milli(455), 0.455);
        assert_eq!(normalized_from_milli(2000), 1.0);
    }

    #[test]
    fn selection_range_from_milli_clamps_and_orders_bounds() {
        let range = selection_range_from_milli(750, 250);
        assert_eq!(range.start(), 0.25);
        assert_eq!(range.end(), 0.75);

        let range = selection_range_from_milli(2000, 0);
        assert_eq!(range.start(), 0.0);
        assert_eq!(range.end(), 1.0);
    }

    #[test]
    fn zoom_steps_from_ui_clamps_to_at_least_one() {
        assert_eq!(zoom_steps_from_ui(0), 1);
        assert_eq!(zoom_steps_from_ui(1), 1);
        assert_eq!(zoom_steps_from_ui(12), 12);
    }

    /// Tiny floating-point drift should not be treated as a waveform view change.
    #[test]
    fn waveform_view_changed_ignores_tiny_float_noise() {
        let base = crate::app::state::WaveformView {
            start: 0.25,
            end: 0.75,
        };
        let nearly_equal = crate::app::state::WaveformView {
            start: 0.25 + (WAVEFORM_VIEW_NOOP_EPSILON * 0.25),
            end: 0.75 - (WAVEFORM_VIEW_NOOP_EPSILON * 0.25),
        };
        assert!(!waveform_view_changed(base, nearly_equal));
    }

    /// Cursor updates should no-op when the cursor is unchanged and waveform is focused.
    #[test]
    fn set_waveform_cursor_milli_noops_when_unchanged_and_focused() {
        let (mut controller, _source) = test_support::dummy_controller();
        controller.ui.focus.context = crate::app::state::FocusContext::Waveform;
        controller.ui.waveform.cursor = Some(0.5);
        let previous_nav = std::time::Instant::now() - std::time::Duration::from_millis(2);
        controller.ui.waveform.cursor_last_navigation_at = Some(previous_nav);

        controller.set_waveform_cursor_milli(500);

        assert_eq!(controller.ui.waveform.cursor, Some(0.5));
        assert_eq!(
            controller.ui.waveform.cursor_last_navigation_at,
            Some(previous_nav)
        );
    }

    /// Selection updates should no-op when the range is unchanged and waveform is focused.
    #[test]
    fn set_waveform_selection_range_milli_noops_when_unchanged_and_focused() {
        let (mut controller, _source) = test_support::dummy_controller();
        controller.ui.focus.context = crate::app::state::FocusContext::Waveform;
        let range = SelectionRange::new(0.25, 0.75);
        controller.selection_state.range.set_range(Some(range));
        controller.ui.waveform.selection = Some(range);

        controller.set_waveform_selection_range_milli(250, 750);

        assert_eq!(controller.selection_state.range.range(), Some(range));
        assert_eq!(controller.ui.waveform.selection, Some(range));
    }

    /// Edit-selection updates should no-op when the range is unchanged and waveform is focused.
    #[test]
    fn set_waveform_edit_selection_range_milli_noops_when_unchanged_and_focused() {
        let (mut controller, _source) = test_support::dummy_controller();
        controller.ui.focus.context = crate::app::state::FocusContext::Waveform;
        let range = SelectionRange::new(0.2, 0.6);
        controller.selection_state.edit_range.set_range(Some(range));
        controller.ui.waveform.edit_selection = Some(range);

        controller.set_waveform_edit_selection_range_milli(200, 600);

        assert_eq!(controller.selection_state.edit_range.range(), Some(range));
        assert_eq!(controller.ui.waveform.edit_selection, Some(range));
    }

    /// Clearing edit selection via native helper should clear edit state and preserve focus.
    #[test]
    fn clear_waveform_edit_selection_with_focus_clears_edit_selection() {
        let (mut controller, _source) = test_support::dummy_controller();
        controller
            .selection_state
            .edit_range
            .set_range(Some(SelectionRange::new(0.1, 0.4)));
        controller.ui.waveform.edit_selection = Some(SelectionRange::new(0.1, 0.4));

        controller.clear_waveform_edit_selection_with_focus();

        assert!(controller.selection_state.edit_range.range().is_none());
        assert!(controller.ui.waveform.edit_selection.is_none());
    }

    /// Edit fade-in handle updates should set a proportional fade-in over the edit selection.
    #[test]
    fn set_waveform_edit_fade_in_end_milli_updates_edit_fade_in_length() {
        let (mut controller, _source) = test_support::dummy_controller();
        let range = SelectionRange::new(0.2, 0.6);
        controller.selection_state.edit_range.set_range(Some(range));
        controller.ui.waveform.edit_selection = Some(range);

        controller.set_waveform_edit_fade_in_end_milli(300);

        let updated = controller.ui.waveform.edit_selection;
        assert!(updated.is_some());
        let fade_in = updated.and_then(|selection| selection.fade_in());
        assert!(fade_in.is_some());
        let fade_in = fade_in.unwrap_or(crate::selection::FadeParams::with_curve(0.0, 0.5));
        assert!((fade_in.length - 0.25).abs() < 0.001);
    }

    /// Edit fade-out handle updates should set a proportional fade-out over the edit selection.
    #[test]
    fn set_waveform_edit_fade_out_start_milli_updates_edit_fade_out_length() {
        let (mut controller, _source) = test_support::dummy_controller();
        let range = SelectionRange::new(0.2, 0.6);
        controller.selection_state.edit_range.set_range(Some(range));
        controller.ui.waveform.edit_selection = Some(range);

        controller.set_waveform_edit_fade_out_start_milli(500);

        let updated = controller.ui.waveform.edit_selection;
        assert!(updated.is_some());
        let fade_out = updated.and_then(|selection| selection.fade_out());
        assert!(fade_out.is_some());
        let fade_out = fade_out.unwrap_or(crate::selection::FadeParams::with_curve(0.0, 0.5));
        assert!((fade_out.length - 0.25).abs() < 0.001);
    }

    /// Deferred playback-age writes should remain queued until debounce expires.
    #[test]
    fn deferred_pending_age_update_commit_waits_for_deadline() {
        let (mut controller, source) = test_support::prepare_with_source_and_wav_entries(vec![
            test_support::sample_entry("one.wav", crate::sample_sources::Rating::NEUTRAL),
            test_support::sample_entry("two.wav", crate::sample_sources::Rating::NEUTRAL),
        ]);
        controller.audio.pending_age_update = Some(PendingAgeUpdate {
            source_id: source.id.clone(),
            root: source.root.clone(),
            relative_path: PathBuf::from("one.wav"),
            played_at: 123,
        });

        controller.defer_pending_age_update_commit_if_path_changes(Path::new("two.wav"));
        assert!(controller.runtime.pending_age_update_commit.is_some());

        controller.flush_pending_age_update_commit();
        assert!(controller.runtime.pending_age_update_commit.is_some());
    }

    /// Queued waveform seek updates should defer commit-side playback work.
    #[test]
    fn queue_waveform_seek_milli_defers_commit_until_deadline() {
        let (mut controller, _source) = test_support::dummy_controller();

        controller.queue_waveform_seek_milli(500);

        assert_eq!(controller.pending_waveform_seek_milli_for_test(), Some(500));
        controller.flush_pending_waveform_seek_commit();
        assert_eq!(controller.pending_waveform_seek_milli_for_test(), Some(500));
    }

    /// Expired deferred waveform seek commits should clear queued seek state.
    #[test]
    fn flush_pending_waveform_seek_commit_clears_queue_after_deadline() {
        let (mut controller, _source) = test_support::dummy_controller();
        controller.queue_waveform_seek_milli(750);
        controller.runtime.pending_waveform_seek_not_before =
            Some(Instant::now() - Duration::from_millis(1));

        controller.flush_pending_waveform_seek_commit();

        assert!(controller.runtime.pending_waveform_seek_milli.is_none());
        assert!(
            controller
                .runtime
                .pending_waveform_seek_not_before
                .is_none()
        );
    }
}
