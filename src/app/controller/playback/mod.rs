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
/// Waveform selection/cursor/zoom action facade methods.
mod waveform_actions;

#[cfg(test)]
mod audio_options_tests;
#[cfg(test)]
/// Playback facade behavior tests.
mod tests;

use formatting::{format_selection_duration, format_timestamp_hms_ms};
use tracing::warn;

#[cfg(test)]
const SHOULD_PLAY_RANDOM_SAMPLE: bool = false;
#[cfg(not(test))]
const SHOULD_PLAY_RANDOM_SAMPLE: bool = true;
const PLAYHEAD_COMPLETION_EPSILON: f32 = 0.001;
/// Equality epsilon used for waveform view no-op detection.
const WAVEFORM_VIEW_NOOP_EPSILON: f64 = 1.0e-9;
/// Integer precision used for pointer-anchored waveform zoom ratios.
const WAVEFORM_ANCHOR_RATIO_MICROS_SCALE: f64 = 1_000_000.0;
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
/// Compute the BPM-snapped minimum selection length (seconds) when snap is enabled.
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
/// Check whether a candidate selection range is long enough for BPM-snapped playback.
pub(crate) fn selection_meets_bpm_min_for_playback(
    controller: &AppController,
    range: SelectionRange,
) -> bool {
    selection_meets_bpm_min(controller, range)
}

impl AppController {
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
        let played_at = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs() as i64;

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
    /// Hide the waveform playhead/selection overlays when playback is inactive.
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
    /// Apply a committed playback selection range and refresh dependent labels/preview state.
    pub(crate) fn apply_selection(&mut self, range: Option<SelectionRange>) {
        player::apply_selection(self, range);
    }
    /// Apply the edit-selection overlay range used by waveform fade/trim editing tools.
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
    /// Apply output volume to runtime audio state without persisting configuration.
    pub(crate) fn apply_volume(&mut self, volume: f32) {
        player::apply_volume(self, volume);
    }
    /// Lazily create or return the shared audio player used by playback operations.
    pub(crate) fn ensure_player(&mut self) -> Result<Option<Rc<RefCell<AudioPlayer>>>, String> {
        player::ensure_player(self)
    }
    /// Queue loop disable after the current cycle boundary to avoid mid-cycle discontinuities.
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
