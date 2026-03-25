use super::*;
use crate::app_dirs::ConfigBaseGuard;
use crate::app_core::state::StatusTone;
use tempfile::tempdir;

#[test]
fn apply_drop_target_transfer_result_reports_cancelled_statuses() {
    let temp = tempdir().must();
    let _guard = ConfigBaseGuard::set(temp.path().to_path_buf());
    let (mut controller, _source, target, _target_drop) = setup_cross_source_drop_fixture(&temp);

    controller
        .drag_drop()
        .apply_drop_target_transfer_result(drop_target_transfer_result(
            DropTargetTransferKind::Copy,
            &target,
            Vec::new(),
            Vec::new(),
            true,
        ));
    assert_eq!(controller.ui.status.text, "Copy cancelled");
    assert_eq!(controller.ui.status.status_tone, StatusTone::Warning);

    controller
        .drag_drop()
        .apply_drop_target_transfer_result(drop_target_transfer_result(
            DropTargetTransferKind::Move,
            &target,
            Vec::new(),
            Vec::new(),
            true,
        ));
    assert_eq!(controller.ui.status.text, "Move cancelled");
    assert_eq!(controller.ui.status.status_tone, StatusTone::Warning);
}

#[test]
fn apply_drop_target_transfer_result_reports_noop_statuses() {
    let temp = tempdir().must();
    let _guard = ConfigBaseGuard::set(temp.path().to_path_buf());
    let (mut controller, _source, target, _target_drop) = setup_cross_source_drop_fixture(&temp);

    controller
        .drag_drop()
        .apply_drop_target_transfer_result(drop_target_transfer_result(
            DropTargetTransferKind::Copy,
            &target,
            Vec::new(),
            Vec::new(),
            false,
        ));
    assert_eq!(controller.ui.status.text, "No samples copied");
    assert_eq!(controller.ui.status.status_tone, StatusTone::Warning);

    controller
        .drag_drop()
        .apply_drop_target_transfer_result(drop_target_transfer_result(
            DropTargetTransferKind::Move,
            &target,
            Vec::new(),
            Vec::new(),
            false,
        ));
    assert_eq!(controller.ui.status.text, "No samples moved");
    assert_eq!(controller.ui.status.status_tone, StatusTone::Warning);
}

#[test]
fn apply_drop_target_transfer_result_reports_partial_errors_with_warning_tone() {
    let temp = tempdir().must();
    let _guard = ConfigBaseGuard::set(temp.path().to_path_buf());
    let (mut controller, source, target, _target_drop) = setup_cross_source_drop_fixture(&temp);

    controller
        .drag_drop()
        .apply_drop_target_transfer_result(drop_target_transfer_result(
            DropTargetTransferKind::Copy,
            &target,
            vec![transferred_sample(&source, "one.wav", "dest/one.wav")],
            vec!["target already contains clip"],
            false,
        ));

    assert_eq!(
        controller.ui.status.text,
        "Copied 1 sample(s) to dest with 1 error(s)"
    );
    assert_eq!(controller.ui.status.status_tone, StatusTone::Warning);
}
