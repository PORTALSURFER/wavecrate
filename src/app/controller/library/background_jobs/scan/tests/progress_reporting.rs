use super::*;

#[test]
fn scan_progress_updates_keep_indeterminate_total_and_path_detail() {
    let (mut controller, _source) = dummy_controller();
    controller.show_status_progress(ProgressTaskKind::Scan, "Scanning source", 0, true);

    handle_scan_progress(&mut controller, 12, Some(String::from("drums\\kick.wav")));

    assert_eq!(controller.ui.progress.task, Some(ProgressTaskKind::Scan));
    assert_eq!(controller.ui.progress.total, 0);
    assert_eq!(controller.ui.progress.completed, 12);
    assert_eq!(
        controller.ui.progress.detail.as_deref(),
        Some("drums\\kick.wav")
    );
}
