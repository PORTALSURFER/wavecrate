use super::*;

#[test]
fn large_background_auto_rename_reports_partial_failure_through_file_ops_progress() {
    /// Large enough to prove partial failures advance inside a real batch.
    const SAMPLE_COUNT: usize = 24;
    clear_batch_latency();
    let (mut controller, source, paths) = large_auto_rename_fixture(SAMPLE_COUNT);
    std::fs::remove_file(source.root.join("sample_010.wav"))
        .expect("remove one fixture file after browser snapshot is loaded");

    BrowserController::new(&mut controller)
        .auto_rename_browser_sample_paths_background_for_tests(&paths)
        .expect("background auto rename should start");
    assert_eq!(
        controller.ui.progress.task,
        Some(crate::app::state::ProgressTaskKind::FileOps)
    );
    assert_eq!(controller.ui.progress.total, SAMPLE_COUNT);
    assert!(controller.ui.progress.visible);

    wait_for_background_jobs(&mut controller, LARGE_BACKGROUND_FILE_OP_TIMEOUT);

    assert_eq!(
        controller.ui.status.text,
        "Auto Rename: renamed 23, skipped 0, failed 1"
    );
    assert_eq!(
        controller.ui.status.status_tone,
        crate::app::state::StatusTone::Warning
    );
    assert!(source.root.join("artistname_SS.wav").exists());
    assert!(source.root.join("artistname_SS_023.wav").exists());
    assert!(!source.root.join("artistname_SS_010.wav").exists());
    assert!(!controller.ui.progress.visible);
}
