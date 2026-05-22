use super::*;
use crate::app::controller::test_support;

#[test]
fn start_selection_drag_arms_without_creating_visible_selection() {
    let (mut controller, _source) = test_support::dummy_controller();
    controller.ui.waveform.bpm_snap_enabled = true;

    start_selection_drag(&mut controller, 0.005);

    assert!(controller.selection_state.range.is_dragging());
    assert!(controller.selection_state.range.is_creating());
    assert!(controller.selection_state.range.range().is_none());
    assert!(controller.ui.waveform.selection.is_none());
}

#[test]
fn update_selection_drag_materializes_exact_anchor_before_snapping() {
    let (mut controller, source) = test_support::dummy_controller();
    controller.sample_view.wav.loaded_audio = Some(LoadedAudio {
        source_id: source.id.clone(),
        root: source.root.clone(),
        relative_path: PathBuf::from("snap_anchor.wav"),
        bytes: Vec::new().into(),
        duration_seconds: 4.0,
        sample_rate: 48_000,
    });
    controller.ui.waveform.bpm_snap_enabled = true;
    controller.ui.waveform.bpm_value = Some(120.0);

    start_selection_drag(&mut controller, 0.31);
    update_selection_drag(&mut controller, 0.44, false);

    let range = controller
        .selection_state
        .range
        .range()
        .expect("selection range should be initialized");
    assert!((range.start() - 0.31).abs() < 1.0e-6);
    assert!((range.end() - 0.435).abs() < 1.0e-6);
}

#[test]
fn finish_selection_drag_without_motion_keeps_no_selection() {
    let (mut controller, _source) = test_support::dummy_controller();

    start_selection_drag(&mut controller, 0.25);
    finish_selection_drag(&mut controller);

    assert!(!controller.selection_state.range.is_dragging());
    assert!(controller.selection_state.range.range().is_none());
    assert!(controller.ui.waveform.selection.is_none());
}

#[test]
fn cancel_click_armed_selection_drag_clears_pending_drag_and_undo() {
    let (mut controller, _source) = test_support::dummy_controller();

    start_selection_drag(&mut controller, 0.25);

    assert!(controller.selection_state.range.is_creating());
    assert!(controller.selection_state.pending_undo.is_some());

    cancel_click_armed_selection_drag(&mut controller);

    assert!(!controller.selection_state.range.is_dragging());
    assert!(controller.selection_state.range.range().is_none());
    assert!(controller.selection_state.pending_undo.is_none());
}

#[test]
fn start_selection_drag_preserves_existing_visible_selection_until_motion() {
    let (mut controller, _source) = test_support::dummy_controller();
    let existing = SelectionRange::new(0.2, 0.4);
    controller.selection_state.range.set_range(Some(existing));
    controller.apply_selection(Some(existing));

    start_selection_drag(&mut controller, 0.7);

    assert_eq!(controller.ui.waveform.selection, Some(existing));
    assert_eq!(controller.selection_state.range.range(), Some(existing));
    assert!(controller.selection_state.range.is_creating());
}

#[test]
fn cancel_click_armed_selection_drag_preserves_existing_visible_selection() {
    let (mut controller, _source) = test_support::dummy_controller();
    let existing = SelectionRange::new(0.2, 0.4);
    controller.selection_state.range.set_range(Some(existing));
    controller.apply_selection(Some(existing));

    start_selection_drag(&mut controller, 0.7);
    cancel_click_armed_selection_drag(&mut controller);

    assert_eq!(controller.selection_state.range.range(), Some(existing));
    assert_eq!(controller.ui.waveform.selection, Some(existing));
    assert!(!controller.selection_state.range.is_dragging());
    assert!(controller.selection_state.pending_undo.is_none());
}
