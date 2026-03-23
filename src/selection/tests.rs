use super::*;

fn assert_range_close(actual: SelectionRange, expected: SelectionRange) {
    let eps = 1e-6;
    assert!((actual.start() - expected.start()).abs() < eps);
    assert!((actual.end() - expected.end()).abs() < eps);
}

#[test]
fn new_range_orders_bounds() {
    let range = SelectionRange::new(0.8, 0.2);
    assert_eq!(range.start(), 0.2);
    assert_eq!(range.end(), 0.8);
}

#[test]
fn empty_range_reports_zero_width() {
    let range = SelectionRange::new(0.5, 0.5);
    assert!(range.is_empty());
    assert_eq!(range.width(), 0.0);
}

#[test]
fn drag_create_tracks_anchor() {
    let mut state = SelectionState::new();
    state.begin_new(0.1);
    let updated = state.update_drag(0.6).unwrap();
    assert_eq!(updated, SelectionRange::new(0.1, 0.6));
}

#[test]
fn arm_new_defers_visible_selection_until_drag_updates() {
    let mut state = SelectionState::new();
    state.arm_new(0.1);
    assert!(state.is_dragging());
    assert!(state.is_creating());
    assert!(state.range().is_none());

    let updated = state.update_drag(0.6).unwrap();
    assert_eq!(updated, SelectionRange::new(0.1, 0.6));
}

#[test]
fn arm_new_preserves_existing_range_until_drag_updates() {
    let mut state = SelectionState::new();
    let existing = SelectionRange::new(0.2, 0.4);
    state.set_range(Some(existing));

    state.arm_new(0.7);

    assert_eq!(state.range(), Some(existing));
    let updated = state.update_drag(0.9).unwrap();
    assert_eq!(updated, SelectionRange::new(0.7, 0.9));
}

#[test]
fn drag_updates_clamp_outside_bounds() {
    let mut state = SelectionState::new();
    state.begin_new(0.3);
    let first = state.update_drag(-0.5).unwrap();
    assert_eq!(first, SelectionRange::new(0.0, 0.3));
    let second = state.update_drag(1.4).unwrap();
    assert_eq!(second, SelectionRange::new(0.3, 1.0));
}

#[test]
fn drag_edges_updates_individually() {
    let mut state = SelectionState::new();
    state.begin_new(0.2);
    state.update_drag(0.7);
    assert!(state.begin_edge_drag(SelectionEdge::Start));
    assert!(state.is_dragging());
    state.update_drag(0.1);
    assert_eq!(state.range().unwrap(), SelectionRange::new(0.1, 0.7));
    assert!(state.begin_edge_drag(SelectionEdge::End));
    state.update_drag(0.9);
    assert_eq!(state.range().unwrap(), SelectionRange::new(0.1, 0.9));
    assert!(state.is_dragging());
}

#[test]
fn dragging_state_clears_on_finish() {
    let mut state = SelectionState::new();
    state.begin_new(0.2);
    state.update_drag(0.7);
    assert!(state.is_dragging());
    state.finish_drag();
    assert!(!state.is_dragging());
}

#[test]
fn drag_create_snaps_to_beats() {
    let mut state = SelectionState::new();
    state.begin_new(0.1);
    let updated = state.update_drag_snapped(0.45, 0.25).unwrap();
    assert_range_close(updated, SelectionRange::new(0.1, 0.35));
}

#[test]
fn drag_edge_snaps_to_beats() {
    let mut state = SelectionState::new();
    state.set_range(Some(SelectionRange::new(0.2, 0.8)));
    assert!(state.begin_edge_drag(SelectionEdge::Start));
    let updated = state.update_drag_snapped(0.1, 0.25).unwrap();
    assert_range_close(updated, SelectionRange::new(0.05, 0.8));
}

#[test]
fn drag_create_below_step_clears_range() {
    let mut state = SelectionState::new();
    state.begin_new(0.2);
    let updated = state.update_drag_snapped(0.22, 0.25);
    assert!(updated.is_none());
    assert!(state.range().is_none());
}

#[test]
fn drag_edge_enforces_min_width() {
    let mut state = SelectionState::new();
    state.set_range(Some(SelectionRange::new(0.2, 0.8)));
    assert!(state.begin_edge_drag(SelectionEdge::Start));
    let updated = state.update_drag_snapped(0.75, 0.25).unwrap();
    assert_range_close(updated, SelectionRange::new(0.55, 0.8));
}

#[test]
fn clear_resets_state() {
    let mut state = SelectionState::new();
    state.begin_new(0.2);
    assert!(state.clear());
    assert!(state.range().is_none());
}

#[test]
fn shift_clamps_within_bounds() {
    let range = SelectionRange::new(0.2, 0.4);
    assert_range_close(range.shift(0.1), SelectionRange::new(0.3, 0.5));
    assert_range_close(range.shift(-0.3), SelectionRange::new(0.0, 0.2));
    assert_range_close(range.shift(1.0), SelectionRange::new(0.8, 1.0));
}

#[test]
fn shift_noops_on_nan() {
    let range = SelectionRange::new(0.2, 0.4);
    assert_eq!(range.shift(f32::NAN), range);
}

#[test]
fn fade_values_are_clamped() {
    let range = SelectionRange::new(0.2, 0.8)
        .with_fade_in(0.6, 0.5)
        .with_fade_out(0.6, 0.5);
    // fade_in + fade_out should not exceed 1.0
    assert!(range.fade_in_length() + range.fade_out_length() <= 1.0);
}

#[test]
fn fade_in_clamps_when_fade_out_exists() {
    let range = SelectionRange::new(0.2, 0.8)
        .with_fade_out(0.7, 0.5)
        .with_fade_in(0.5, 0.5);
    assert_eq!(range.fade_out_length(), 0.7);
    assert_eq!(range.fade_in_length(), 0.3); // Clamped to 1.0 - 0.7
}

#[test]
fn fade_out_clamps_when_fade_in_exists() {
    let range = SelectionRange::new(0.2, 0.8)
        .with_fade_in(0.6, 0.5)
        .with_fade_out(0.8, 0.5);
    assert_eq!(range.fade_in_length(), 0.6);
    assert_eq!(range.fade_out_length(), 0.4); // Clamped to 1.0 - 0.6
}

#[test]
fn fades_preserved_during_shift() {
    let range = SelectionRange::new(0.2, 0.4)
        .with_fade_in(0.2, 0.5)
        .with_fade_out(0.3, 0.5);
    let shifted = range.shift(0.1);
    assert_eq!(shifted.fade_in_length(), 0.2);
    assert_eq!(shifted.fade_out_length(), 0.3);
}

#[test]
fn fade_mute_sections_zero_gain() {
    let range = SelectionRange::new(0.2, 0.8)
        .with_fade_in(0.4, 0.0)
        .with_fade_out(0.4, 0.0)
        .with_fade_in_mute(0.2)
        .with_fade_out_mute(0.1);
    let muted_start = fade_gain_at_position(
        0.1,
        range.start(),
        range.end(),
        range.gain(),
        range.fade_in(),
        range.fade_out(),
        0.0,
    );
    let muted_end = fade_gain_at_position(
        0.82,
        range.start(),
        range.end(),
        range.gain(),
        range.fade_in(),
        range.fade_out(),
        0.0,
    );
    let ramp_mid = fade_gain_at_position(
        0.3,
        range.start(),
        range.end(),
        range.gain(),
        range.fade_in(),
        range.fade_out(),
        0.0,
    );
    assert!(muted_start.abs() < 1e-6);
    assert!(muted_end.abs() < 1e-6);
    assert!(ramp_mid > 0.0 && ramp_mid < 1.0);
}

#[test]
fn fade_mute_can_extend_past_selection_width() {
    let range = SelectionRange::new(0.4, 0.5)
        .with_fade_in(0.2, 0.0)
        .with_fade_out(0.2, 0.0)
        .with_fade_in_mute(4.0);
    let muted_far_left = fade_gain_at_position(
        0.05,
        range.start(),
        range.end(),
        range.gain(),
        range.fade_in(),
        range.fade_out(),
        0.0,
    );
    assert!(muted_far_left.abs() < 1e-6);
}

#[test]
fn fade_mute_persists_when_fade_length_collapses() {
    let range = SelectionRange::new(0.2, 0.8)
        .with_fade_in(0.3, 0.5)
        .with_fade_in_mute(0.2)
        .with_fade_in(0.0, 0.5);
    assert_eq!(range.fade_in_length(), 0.0);
    assert!(range.fade_in().is_some());
    assert!(range.fade_in_mute_length() > 0.0);
}

#[test]
fn new_range_has_zero_fades() {
    let range = SelectionRange::new(0.3, 0.7);
    assert_eq!(range.fade_in_length(), 0.0);
    assert_eq!(range.fade_out_length(), 0.0);
}

#[test]
fn fade_gain_ramps_selection_edges() {
    let range = SelectionRange::new(0.0, 1.0)
        .with_fade_in(0.2, 0.0)
        .with_fade_out(0.2, 0.0);
    let gain_start = fade_gain_at_position(
        0.0,
        range.start(),
        range.end(),
        range.gain(),
        range.fade_in(),
        range.fade_out(),
        0.0,
    );
    let gain_mid = fade_gain_at_position(
        0.5,
        range.start(),
        range.end(),
        range.gain(),
        range.fade_in(),
        range.fade_out(),
        0.0,
    );
    let gain_end = fade_gain_at_position(
        1.0,
        range.start(),
        range.end(),
        range.gain(),
        range.fade_in(),
        range.fade_out(),
        0.0,
    );
    assert!(gain_start.abs() < 1e-6);
    assert!((gain_mid - 1.0).abs() < 1e-6);
    assert!(gain_end.abs() < 1e-6);
}

#[test]
fn fade_mute_does_not_extend_fade_curve() {
    let range = SelectionRange::new(0.0, 1.0)
        .with_fade_in(0.2, 0.0)
        .with_fade_in_mute(0.3);
    let post_fade_gain = fade_gain_at_position(
        0.25,
        range.start(),
        range.end(),
        range.gain(),
        range.fade_in(),
        range.fade_out(),
        0.0,
    );
    assert!((post_fade_gain - 1.0).abs() < 1e-6);
}
