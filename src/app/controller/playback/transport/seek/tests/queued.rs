use super::support::*;

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
fn queue_waveform_seek_nanos_cancels_click_armed_selection_drag() {
    let (mut controller, _source) = test_support::dummy_controller();
    seed_waveform_ready_for_seek(&mut controller);
    super::super::super::selection::start_selection_drag(&mut controller, 0.25);

    assert!(controller.selection_state.range.is_creating());
    assert!(controller.selection_state.pending_undo.is_some());

    queue_waveform_seek_nanos(&mut controller, 750_000_000);

    assert!(!controller.selection_state.range.is_dragging());
    assert!(controller.selection_state.pending_undo.is_none());
    assert_eq!(
        controller.runtime.pending_waveform_seek_nanos,
        Some(750_000_000)
    );
    assert_eq!(controller.ui.waveform.cursor, Some(0.75));
}

#[test]
fn queue_waveform_seek_nanos_clears_existing_selection_after_canceling_click_arm() {
    let (mut controller, _source) = test_support::dummy_controller();
    seed_waveform_ready_for_seek(&mut controller);
    let selection = SelectionRange::new(0.2, 0.4);
    controller.selection_state.range.set_range(Some(selection));
    controller.apply_selection(Some(selection));
    super::super::super::selection::start_selection_drag(&mut controller, 0.7);

    assert!(controller.selection_state.range.is_creating());

    queue_waveform_seek_nanos(&mut controller, 750_000_000);

    assert!(!controller.selection_state.range.is_dragging());
    assert!(controller.selection_state.range.range().is_none());
    assert!(controller.ui.waveform.selection.is_none());
    assert!(controller.selection_state.pending_undo.is_none());
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
