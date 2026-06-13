use super::*;

#[test]
fn large_tag_sidebar_background_auto_rename_streams_file_ops_progress_and_refreshes_rows() {
    /// Large enough to exercise progress streaming and final browser-row refresh behavior.
    const SAMPLE_COUNT: usize = 24;
    clear_batch_latency();
    let (mut controller, source, paths) = large_auto_rename_fixture(SAMPLE_COUNT);
    controller.set_browser_selected_paths(paths.clone());
    let visible_before = controller.visible_browser_len();

    controller
        .apply_browser_tag_sidebar_normal_tag("Vintage FX")
        .expect("large tag mutation should update selected paths");
    controller.ui.browser.tag_sidebar_auto_rename = true;
    BrowserController::new(&mut controller)
        .auto_rename_browser_sample_paths_background_for_tests(&paths)
        .expect("background auto rename should start after tag mutation");

    assert_eq!(
        controller.ui.progress.task,
        Some(crate::app::state::ProgressTaskKind::FileOps)
    );
    assert_eq!(controller.ui.progress.title, "Preparing auto rename");
    assert_eq!(controller.ui.progress.total, SAMPLE_COUNT);
    assert!(controller.ui.progress.cancelable);

    wait_for_file_ops_detail(&mut controller, Duration::from_secs(15), |detail| {
        detail.starts_with("Planning sample_")
    });
    assert_eq!(
        controller.ui.progress.task,
        Some(crate::app::state::ProgressTaskKind::FileOps)
    );
    assert_eq!(controller.ui.progress.completed, 0);
    assert!(
        controller
            .ui
            .progress
            .has_task(crate::app::state::ProgressTaskKind::FileOps)
    );

    wait_for_background_jobs(&mut controller, LARGE_BACKGROUND_FILE_OP_TIMEOUT);

    assert_eq!(controller.visible_browser_len(), visible_before);
    assert!(source.root.join("artistname_SS_vintagefx.wav").exists());
    assert!(source.root.join("artistname_SS_vintagefx_023.wav").exists());
    assert!(!source.root.join("sample_000.wav").exists());
    assert!(!controller.ui.progress.visible);
    assert_eq!(
        controller.ui.status.text,
        "Auto Rename: renamed 24, skipped 0, failed 0"
    );

    let _ = controller.refresh_projection_revision_bus();
    let projected = crate::app_core::ui_projection::project_browser_model(&mut controller);
    assert_eq!(projected.rows[0].label.as_ref(), "artistname_SS_vintagefx");
    assert_eq!(
        controller
            .browser_projection_entry(0)
            .map(|entry| entry.relative_path),
        Some(Path::new("artistname_SS_vintagefx.wav"))
    );
}
