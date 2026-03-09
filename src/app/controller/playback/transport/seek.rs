use super::*;
use std::time::{Duration, Instant};

/// Debounce window for committing queued waveform seek playback updates.
///
/// This keeps drag-heavy seek interactions cheap by applying the final replay
/// seek shortly after pointer activity settles.
const WAVEFORM_SEEK_COMMIT_DEBOUNCE: Duration = Duration::from_millis(24);

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
    clear_selection_for_outside_waveform_seek(controller, clamped);
    controller.set_waveform_cursor_milli(clamped);
    controller.runtime.pending_waveform_seek_milli = Some(clamped);
    controller.runtime.pending_waveform_seek_not_before =
        Some(Instant::now() + WAVEFORM_SEEK_COMMIT_DEBOUNCE);
}

/// Record the most recent play start position.
pub(crate) fn record_play_start(controller: &mut AppController, position: f32) {
    let clamped = position.clamp(0.0, 1.0);
    controller.ui.waveform.last_start_marker = Some(clamped);
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
    let Some(position_milli) = controller.runtime.pending_waveform_seek_milli.take() else {
        return;
    };
    let normalized = (f32::from(position_milli) / 1000.0).clamp(0.0, 1.0);
    seek_to(controller, normalized);
    controller.set_waveform_cursor(normalized);
    controller.focus_waveform();
}

/// Clear the active playback selection when a waveform seek lands outside it.
///
/// This keeps a plain waveform click from leaving an old marked playback span
/// active when the user is clearly asking to audition a different location.
fn clear_selection_for_outside_waveform_seek(controller: &mut AppController, position_milli: u16) {
    let normalized = (f32::from(position_milli.min(1000)) / 1000.0).clamp(0.0, 1.0);
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::controller::test_support;
    use crate::waveform::DecodedWaveform;
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
    fn flush_pending_waveform_seek_commit_waits_for_deadline() {
        let (mut controller, _source) = test_support::dummy_controller();
        queue_waveform_seek_milli(&mut controller, 500);
        controller.runtime.pending_waveform_seek_not_before =
            Some(Instant::now() + Duration::from_millis(50));

        flush_pending_waveform_seek_commit(&mut controller);

        assert_eq!(controller.runtime.pending_waveform_seek_milli, Some(500));
    }

    #[test]
    fn queue_waveform_seek_milli_clears_selection_when_target_is_outside_span() {
        let (mut controller, _source) = test_support::dummy_controller();
        seed_waveform_ready_for_seek(&mut controller);
        let selection = SelectionRange::new(0.2, 0.4);
        controller.selection_state.range.set_range(Some(selection));
        controller.apply_selection(Some(selection));

        queue_waveform_seek_milli(&mut controller, 750);

        assert!(controller.selection_state.range.range().is_none());
        assert!(controller.ui.waveform.selection.is_none());
        assert_eq!(controller.runtime.pending_waveform_seek_milli, Some(750));
        assert_eq!(controller.ui.waveform.cursor, Some(0.75));
    }

    #[test]
    fn queue_waveform_seek_milli_preserves_selection_when_target_is_inside_span() {
        let (mut controller, _source) = test_support::dummy_controller();
        seed_waveform_ready_for_seek(&mut controller);
        let selection = SelectionRange::new(0.2, 0.4);
        controller.selection_state.range.set_range(Some(selection));
        controller.apply_selection(Some(selection));

        queue_waveform_seek_milli(&mut controller, 300);

        assert_eq!(controller.selection_state.range.range(), Some(selection));
        assert_eq!(controller.ui.waveform.selection, Some(selection));
        assert_eq!(controller.runtime.pending_waveform_seek_milli, Some(300));
        assert_eq!(controller.ui.waveform.cursor, Some(0.3));
    }
}
