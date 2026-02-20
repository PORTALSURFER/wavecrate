use super::super::test_support::dummy_controller;

#[test]
/// Live volume changes should defer persistence and mark runtime state dirty.
fn set_volume_live_marks_setting_dirty_for_deferred_persist() {
    let (mut controller, _source) = dummy_controller();
    controller.ui.volume = 0.2;
    controller.runtime.volume_persist_dirty = false;
    controller.runtime.volume_persist_deadline = None;

    controller.set_volume_live(0.75);

    assert!((controller.ui.volume - 0.75).abs() < f32::EPSILON);
    assert!(controller.runtime.volume_persist_dirty);
    assert!(controller.runtime.volume_persist_deadline.is_some());
}

#[test]
/// Re-applying the same volume should not enqueue persistence work.
fn set_volume_live_does_not_mark_dirty_for_noop_value() {
    let (mut controller, _source) = dummy_controller();
    controller.ui.volume = 0.5;
    controller.runtime.volume_persist_dirty = false;
    controller.runtime.volume_persist_deadline = None;

    controller.set_volume_live(0.5);

    assert!((controller.ui.volume - 0.5).abs() < f32::EPSILON);
    assert!(!controller.runtime.volume_persist_dirty);
    assert!(controller.runtime.volume_persist_deadline.is_none());
}
