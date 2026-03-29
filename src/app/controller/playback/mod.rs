use super::*;
pub(crate) use super::{AppController, BPM_MIN_SELECTION_DIVISOR, StatusTone};
pub(crate) use crate::sample_sources::*;
pub(crate) use crate::selection::SelectionRange;

use std::path::PathBuf;
use std::time::Duration;

pub(crate) mod audio_cache;
pub(crate) mod audio_loader;
pub(crate) mod audio_options;
pub(crate) mod audio_samples;
mod bpm_policy;
pub(crate) mod loop_crossfade;
/// Disk-backed waveform cache persistence used to reuse decoded waveforms across app restarts.
pub(crate) mod persistent_waveform_cache;
mod playback_age;
pub(crate) mod recording;

mod browser_nav;
mod compare_anchor;
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
/// Native waveform action regressions for playback behavior.
mod native_action_tests;
#[cfg(test)]
/// Playback facade behavior tests.
mod tests;
#[cfg(test)]
/// Waveform action behavior tests.
mod waveform_action_tests;

pub(crate) use bpm_policy::{
    bpm_min_selection_seconds, selection_meets_bpm_min_for_playback,
    snap_waveform_delta_to_bpm_step, snap_waveform_micros_to_bpm_anchor, waveform_bpm_snap_step,
};
pub(crate) use compare_anchor::play_loaded_audio_for_path;
use formatting::{format_selection_duration, format_timestamp_hms_ms};

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

impl AppController {
    /// Returns the duration in seconds for the currently loaded audio, if any.
    pub(crate) fn loaded_audio_duration_seconds(&self) -> Option<f32> {
        self.sample_view
            .wav
            .loaded_audio
            .as_ref()
            .map(|audio| audio.duration_seconds)
    }

    /// Start playback from the beginning of the selected sample.
    pub fn play_from_start(&mut self) -> bool {
        transport::play_from_start(self)
    }

    /// Start playback from the current playhead/cursor position.
    pub fn play_from_current_playhead(&mut self) -> bool {
        transport::play_from_current_playhead(self)
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
    pub fn record_play_start(&mut self, position: f64) {
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
        self.runtime.pending_waveform_seek_nanos.is_some()
    }

    #[cfg(test)]
    /// Expose queued deferred waveform seek target for controller/runtime tests.
    pub(crate) fn pending_waveform_seek_nanos_for_test(&self) -> Option<u32> {
        self.runtime.pending_waveform_seek_nanos
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
    pub fn play_audio(&mut self, looped: bool, start_override: Option<f64>) -> Result<(), String> {
        player::play_audio(self, looped, start_override)
    }

    /// Return true if audio playback is currently active.
    pub fn is_playing(&self) -> bool {
        player::is_playing(self)
    }

    /// Return live player progress while transport is actively running.
    ///
    /// Native projection paths use this to derive smoother motion-only playhead
    /// updates than the coarser retained UI snapshot alone can provide.
    pub(crate) fn live_playback_progress(&self) -> Option<f32> {
        player::live_progress(self)
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

    #[cfg(test)]
    /// Seed a minimal loaded-audio fixture for tests that only need duration metadata.
    pub(crate) fn set_loaded_audio_duration_for_tests(&mut self, duration_seconds: f32) {
        self.sample_view.wav.loaded_audio = (duration_seconds.is_finite()
            && duration_seconds > 0.0)
            .then_some(crate::app::controller::state::audio::LoadedAudio {
                source_id: crate::sample_sources::SourceId::new(),
                root: PathBuf::new(),
                relative_path: PathBuf::from("test.wav"),
                bytes: Vec::new().into(),
                duration_seconds,
                sample_rate: 48_000,
            });
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

    /// Queue one loop restart at the current cycle boundary using a new start position.
    pub(crate) fn defer_loop_retarget_after_cycle(
        &mut self,
        start_override: f64,
    ) -> Result<bool, String> {
        player::defer_loop_retarget_after_cycle(self, start_override)
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
