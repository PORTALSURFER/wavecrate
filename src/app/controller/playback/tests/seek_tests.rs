use super::super::*;
use crate::app::controller::test_support;

/// Queued waveform seek updates should defer commit-side playback work.
#[test]
fn queue_waveform_seek_milli_defers_commit_until_deadline() {
    let (mut controller, _source) = test_support::dummy_controller();

    controller.queue_waveform_seek_milli(500);

    assert_eq!(
        controller.pending_waveform_seek_nanos_for_test(),
        Some(500_000_000)
    );
    controller.flush_pending_waveform_seek_commit();
    assert_eq!(
        controller.pending_waveform_seek_nanos_for_test(),
        Some(500_000_000)
    );
}

/// Expired deferred waveform seek commits should clear queued seek state.
#[test]
fn flush_pending_waveform_seek_commit_clears_queue_after_deadline() {
    let (mut controller, _source) = test_support::dummy_controller();
    controller.queue_waveform_seek_milli(750);
    controller.runtime.pending_waveform_seek_not_before =
        Some(Instant::now() - Duration::from_millis(1));

    controller.flush_pending_waveform_seek_commit();

    assert!(controller.runtime.pending_waveform_seek_nanos.is_none());
    assert!(
        controller
            .runtime
            .pending_waveform_seek_not_before
            .is_none()
    );
}
