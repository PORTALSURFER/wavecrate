use std::fs;

use super::*;
use crate::native_app::sample_library::folder_browser::scan::{
    FolderScanProgress, FolderScanRequest, scan_source_with_progress,
};

fn temp_dir_with_wav() -> tempfile::TempDir {
    let root = tempfile::tempdir().expect("source root");
    fs::write(root.path().join("sample.wav"), [0_u8; 8]).expect("write sample");
    root
}

#[test]
fn stale_progress_is_ignored() {
    let root = temp_dir_with_wav();
    let mut browser = FolderBrowserState::load_default();
    let mut workflow = SourceScanWorkflow::new();
    let request = workflow
        .begin_add_source_path(&mut browser, root.path().to_path_buf(), 7)
        .expect("scan request");
    workflow.start_scan(&request);

    let stale = FolderScanProgress {
        task_id: request.task_id + 1,
        source_id: request.source_id.clone(),
        label: request.label.clone(),
        phase: String::from("Scanning"),
        completed: 1,
        total: 1,
        detail: String::new(),
    };

    assert!(!workflow.apply_progress(&browser, stale));
    assert_eq!(
        workflow.progress().expect("queued progress").phase,
        "Queued"
    );
}

#[test]
fn stale_finish_keeps_active_scan_owner() {
    let root = temp_dir_with_wav();
    let mut browser = FolderBrowserState::load_default();
    let mut workflow = SourceScanWorkflow::new();
    let request = workflow
        .begin_add_source_path(&mut browser, root.path().to_path_buf(), 11)
        .expect("scan request");
    workflow.start_scan(&request);
    let stale_result = scan_source_with_progress(
        FolderScanRequest {
            task_id: request.task_id + 1,
            source_id: request.source_id.clone(),
            label: request.label.clone(),
            root: request.root.clone(),
        },
        |_| {},
        |_| {},
    );

    assert!(matches!(
        workflow.finish_scan(&mut browser, stale_result),
        SourceScanFinish::Stale { .. }
    ));
    assert!(workflow.active());
}

#[test]
fn pending_refresh_waits_for_active_scan() {
    let root = temp_dir_with_wav();
    let mut browser = FolderBrowserState::load_default();
    let mut workflow = SourceScanWorkflow::new();
    let request = workflow
        .begin_add_source_path(&mut browser, root.path().to_path_buf(), 21)
        .expect("scan request");
    let source_id = request.source_id.clone();
    workflow.start_scan(&request);

    let plan = workflow.plan_filesystem_change(&mut browser, source_id.clone(), &[], true);

    assert!(matches!(
        plan,
        SourceFilesystemChangePlan::DeferredAlreadyRunning { .. }
    ));
    assert_eq!(workflow.next_pending_refresh_if_idle(), None);
    let result = scan_source_with_progress(request, |_| {}, |_| {});
    assert!(matches!(
        workflow.finish_scan(&mut browser, result),
        SourceScanFinish::Applied { .. }
    ));
    assert_eq!(workflow.next_pending_refresh_if_idle(), Some(source_id));
}
