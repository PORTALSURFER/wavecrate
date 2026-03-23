use super::*;
use crate::app::controller::playback::waveform_actions::{
    nanos_from_milli, normalized64_from_nanos,
};
use std::time::{Duration, Instant};

/// Debounce window for committing queued waveform seek playback updates.
///
/// This keeps drag-heavy seek interactions cheap by applying the final replay
/// seek shortly after pointer activity settles.
const WAVEFORM_SEEK_COMMIT_DEBOUNCE: Duration = Duration::from_millis(24);

pub(crate) fn seek_to(controller: &mut AppController, position: f64) {
    let looped = controller.ui.waveform.loop_enabled;
    record_play_start(controller, position);
    if let Err(err) = controller.play_audio(looped, Some(position)) {
        controller.set_status(err, StatusTone::Error);
    }
}

/// Queue a waveform seek request and defer playback restart to frame prep.
pub(crate) fn queue_waveform_seek_nanos(controller: &mut AppController, position_nanos: u32) {
    let clamped = position_nanos.min(1_000_000_000);
    clear_selection_for_outside_waveform_seek(controller, normalized64_from_nanos(clamped));
    controller.set_waveform_cursor_nanos(clamped);
    if should_commit_waveform_seek_immediately(controller) {
        controller.runtime.pending_waveform_seek_nanos = None;
        controller.runtime.pending_waveform_seek_not_before = None;
        let normalized = normalized64_from_nanos(clamped);
        seek_to(controller, normalized);
        controller.set_waveform_cursor(normalized as f32);
        controller.focus_waveform();
        return;
    }
    controller.runtime.pending_waveform_seek_nanos = Some(clamped);
    controller.runtime.pending_waveform_seek_not_before =
        Some(Instant::now() + WAVEFORM_SEEK_COMMIT_DEBOUNCE);
}

/// Queue a waveform seek request and defer playback restart to frame prep.
pub(crate) fn queue_waveform_seek_milli(controller: &mut AppController, position_milli: u16) {
    queue_waveform_seek_nanos(controller, nanos_from_milli(position_milli));
}

/// Record the most recent play start position.
pub(crate) fn record_play_start(controller: &mut AppController, position: f64) {
    record_play_start_with_view_policy(controller, position, false);
}

/// Record the most recent play start position without changing the current waveform view.
pub(crate) fn record_play_start_preserving_view(controller: &mut AppController, position: f64) {
    record_play_start_with_view_policy(controller, position, true);
}

/// Record the most recent play start position and optionally preserve the current waveform view.
fn record_play_start_with_view_policy(
    controller: &mut AppController,
    position: f64,
    preserve_view: bool,
) {
    let clamped = position.clamp(0.0, 1.0) as f32;
    controller.ui.waveform.last_start_marker = Some(clamped);
    if preserve_view {
        if !controller.waveform_ready() {
            return;
        }
        controller.ui.waveform.cursor = Some(clamped);
        controller.ui.waveform.cursor_last_navigation_at = Some(Instant::now());
        return;
    }
    controller.set_waveform_cursor(clamped);
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
    let Some(position_nanos) = controller.runtime.pending_waveform_seek_nanos.take() else {
        return;
    };
    let normalized = normalized64_from_nanos(position_nanos);
    seek_to(controller, normalized);
    controller.set_waveform_cursor(normalized as f32);
    controller.focus_waveform();
}

/// Clear the active playback selection when a waveform seek lands outside it.
fn clear_selection_for_outside_waveform_seek(controller: &mut AppController, position: f64) {
    let normalized = position.clamp(0.0, 1.0) as f32;
    let Some(selection) = controller
        .selection_state
        .range
        .range()
        .or(controller.ui.waveform.selection)
    else {
        return;
    };
    if waveform_selection_contains_position(selection, normalized) {
        return;
    }
    super::selection::clear_selection(controller);
}

/// Return whether one normalized playback position lands inside a selection.
fn waveform_selection_contains_position(selection: SelectionRange, position: f32) -> bool {
    position >= selection.start() && position <= selection.end()
}

fn should_commit_waveform_seek_immediately(controller: &AppController) -> bool {
    !controller.is_playing()
        && controller.sample_view.wav.loaded_audio.is_some()
        && controller.audio.player.is_some()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::controller::test_support;
    use crate::app::controller::test_support::{
        load_waveform_selection, prepare_with_source_and_wav_entries, sample_entry,
    };
    use crate::waveform::DecodedWaveform;
    use std::cell::RefCell;
    use std::rc::Rc;
    use std::sync::Arc;

    /// Seed minimal waveform state so seek tests exercise cursor updates on a ready waveform.
    fn seed_waveform_ready_for_seek(controller: &mut AppController) {
        controller.sample_view.waveform.decoded = Some(Arc::new(DecodedWaveform {
            cache_token: 1,
            samples: Arc::from(vec![0.0; 16]),
            analysis_samples: Arc::from(Vec::new()),
            analysis_sample_rate: 0,
            analysis_stride: 1,
            peaks: None,
            duration_seconds: 1.0,
            sample_rate: 48_000,
            channels: 1,
        }));
    }

    #[test]
    fn queue_waveform_seek_milli_clamps_input() {
        let (mut controller, _source) = test_support::dummy_controller();

        queue_waveform_seek_nanos(&mut controller, 1_500_000_000);

        assert_eq!(
            controller.runtime.pending_waveform_seek_nanos,
            Some(1_000_000_000)
        );
        assert!(
            controller
                .runtime
                .pending_waveform_seek_not_before
                .is_some()
        );
    }

    #[test]
    fn flush_pending_waveform_seek_commit_waits_for_deadline() {
        let (mut controller, _source) = test_support::dummy_controller();
        queue_waveform_seek_nanos(&mut controller, 500_000_000);
        controller.runtime.pending_waveform_seek_not_before =
            Some(Instant::now() + Duration::from_millis(50));

        flush_pending_waveform_seek_commit(&mut controller);

        assert_eq!(
            controller.runtime.pending_waveform_seek_nanos,
            Some(500_000_000)
        );
    }

    #[test]
    fn queue_waveform_seek_milli_clears_selection_when_target_is_outside_span() {
        let (mut controller, _source) = test_support::dummy_controller();
        seed_waveform_ready_for_seek(&mut controller);
        let selection = SelectionRange::new(0.2, 0.4);
        controller.selection_state.range.set_range(Some(selection));
        controller.apply_selection(Some(selection));

        queue_waveform_seek_nanos(&mut controller, 750_000_000);

        assert!(controller.selection_state.range.range().is_none());
        assert!(controller.ui.waveform.selection.is_none());
        assert_eq!(
            controller.runtime.pending_waveform_seek_nanos,
            Some(750_000_000)
        );
        assert_eq!(controller.ui.waveform.cursor, Some(0.75));
    }

    #[test]
    fn queue_waveform_seek_milli_preserves_selection_when_target_is_inside_span() {
        let (mut controller, _source) = test_support::dummy_controller();
        seed_waveform_ready_for_seek(&mut controller);
        let selection = SelectionRange::new(0.2, 0.4);
        controller.selection_state.range.set_range(Some(selection));
        controller.apply_selection(Some(selection));

        queue_waveform_seek_nanos(&mut controller, 300_000_000);

        assert_eq!(controller.selection_state.range.range(), Some(selection));
        assert_eq!(controller.ui.waveform.selection, Some(selection));
        assert_eq!(
            controller.runtime.pending_waveform_seek_nanos,
            Some(300_000_000)
        );
        assert_eq!(controller.ui.waveform.cursor, Some(0.3));
    }

    #[test]
    fn queue_waveform_seek_milli_starts_immediately_when_stopped_with_loaded_audio() {
        let Some(player) = crate::audio::AudioPlayer::playing_for_tests() else {
            return;
        };
        let (mut controller, source) = prepare_with_source_and_wav_entries(vec![sample_entry(
            "instant_seek.wav",
            crate::sample_sources::Rating::NEUTRAL,
        )]);
        controller.audio.player = Some(Rc::new(RefCell::new(player)));
        load_waveform_selection(
            &mut controller,
            &source,
            "instant_seek.wav",
            &[0.0; 64],
            SelectionRange::new(0.25, 0.75),
        );
        controller.selection_state.range.set_range(None);
        controller.ui.waveform.selection = None;
        controller
            .audio
            .player
            .as_ref()
            .expect("player")
            .borrow_mut()
            .stop();

        queue_waveform_seek_nanos(&mut controller, 500_000_000);

        assert!(controller.runtime.pending_waveform_seek_nanos.is_none());
        assert!(
            controller
                .runtime
                .pending_waveform_seek_not_before
                .is_none()
        );
        assert!(controller.is_playing());
        assert_eq!(controller.ui.waveform.cursor, Some(0.5));
        assert_eq!(controller.ui.waveform.playhead.position, 0.5);
        assert_eq!(controller.ui.waveform.last_start_marker, Some(0.5));
    }

    #[test]
    fn queue_waveform_seek_milli_still_defers_commit_while_playing() {
        let Some(player) = crate::audio::AudioPlayer::playing_for_tests() else {
            return;
        };
        let (mut controller, source) = prepare_with_source_and_wav_entries(vec![sample_entry(
            "deferred_seek.wav",
            crate::sample_sources::Rating::NEUTRAL,
        )]);
        controller.audio.player = Some(Rc::new(RefCell::new(player)));
        load_waveform_selection(
            &mut controller,
            &source,
            "deferred_seek.wav",
            &[0.0; 64],
            SelectionRange::new(0.25, 0.75),
        );
        assert!(controller.play_audio(false, None).is_ok());
        assert!(controller.is_playing());

        queue_waveform_seek_nanos(&mut controller, 750_000_000);

        assert_eq!(
            controller.runtime.pending_waveform_seek_nanos,
            Some(750_000_000)
        );
        assert!(
            controller
                .runtime
                .pending_waveform_seek_not_before
                .is_some()
        );
        assert_eq!(controller.ui.waveform.cursor, Some(0.75));
    }
}
