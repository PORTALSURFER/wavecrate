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
            database_root: request.database_root.clone(),
            rating_decay_weeks: request.rating_decay_weeks,
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

#[test]
fn cached_source_selection_defers_reconcile_while_another_scan_is_active() {
    let first_root = temp_dir_with_wav();
    let second_root = temp_dir_with_wav();
    let mut browser = FolderBrowserState::load_default();
    let mut workflow = SourceScanWorkflow::new();

    let first = workflow
        .begin_add_source_path(&mut browser, first_root.path().to_path_buf(), 31)
        .expect("first source scan");
    let first_id = first.source_id.clone();
    workflow.start_scan(&first);
    let first_result = scan_source_with_progress(first, |_| {}, |_| {});
    assert!(matches!(
        workflow.finish_scan(&mut browser, first_result),
        SourceScanFinish::Applied { .. }
    ));

    let second = workflow
        .begin_add_source_path_preserving_selection(
            &mut browser,
            second_root.path().to_path_buf(),
            32,
        )
        .expect("second source scan");
    let second_id = second.source_id.clone();
    workflow.start_scan(&second);
    let second_result = scan_source_with_progress(second, |_| {}, |_| {});
    assert!(matches!(
        workflow.finish_scan(&mut browser, second_result),
        SourceScanFinish::Applied { .. }
    ));

    let active = workflow
        .begin_source_scan(&mut browser, first_id.clone(), 33)
        .expect("active source rescan");
    workflow.start_scan(&active);

    assert!(matches!(
        workflow.begin_select_source(&mut browser, second_id.clone(), 34),
        SourceSelectionRequest::Settled
    ));
    assert_eq!(browser.selected_source_id(), second_id);
    assert!(browser.selected_source_loaded());
    assert!(workflow.pending_refresh_contains_for_tests(&second_id));
    let second_visible = browser
        .selected_audio_files()
        .into_iter()
        .map(|file| file.id.clone())
        .collect::<Vec<_>>();

    assert!(matches!(
        workflow.begin_select_source(&mut browser, second_id.clone(), 35),
        SourceSelectionRequest::Settled
    ));
    assert_eq!(
        browser
            .selected_audio_files()
            .into_iter()
            .map(|file| file.id.clone())
            .collect::<Vec<_>>(),
        second_visible,
        "reselecting the visible source must preserve its live tree"
    );
    assert!(matches!(
        workflow.begin_select_source(&mut browser, first_id.clone(), 36),
        SourceSelectionRequest::Settled
    ));
    assert_eq!(browser.selected_source_id(), first_id);
    assert!(browser.selected_source_loaded());
    assert!(!workflow.pending_refresh_contains_for_tests(&first_id));

    let active_result = scan_source_with_progress(active, |_| {}, |_| {});
    assert!(matches!(
        workflow.finish_scan(&mut browser, active_result),
        SourceScanFinish::Applied { .. }
    ));
    assert_eq!(workflow.next_pending_refresh_if_idle(), Some(second_id));
}

#[test]
fn active_source_reselection_preserves_visible_tree() {
    let root = temp_dir_with_wav();
    let mut browser = FolderBrowserState::load_default();
    let mut workflow = SourceScanWorkflow::new();
    let initial = workflow
        .begin_add_source_path(&mut browser, root.path().to_path_buf(), 40)
        .expect("initial scan");
    let source_id = initial.source_id.clone();
    workflow.start_scan(&initial);
    let result = scan_source_with_progress(initial, |_| {}, |_| {});
    assert!(matches!(
        workflow.finish_scan(&mut browser, result),
        SourceScanFinish::Applied { .. }
    ));
    let rescan = workflow
        .begin_source_scan(&mut browser, source_id.clone(), 41)
        .expect("rescan");
    workflow.start_scan(&rescan);
    let visible_before = browser
        .selected_audio_files()
        .into_iter()
        .map(|file| file.id.clone())
        .collect::<Vec<_>>();

    assert!(matches!(
        workflow.begin_select_source(&mut browser, source_id, 42),
        SourceSelectionRequest::Settled
    ));
    assert_eq!(
        browser
            .selected_audio_files()
            .into_iter()
            .map(|file| file.id.clone())
            .collect::<Vec<_>>(),
        visible_before
    );
}

#[test]
fn uncached_selection_reports_deferred_while_another_scan_is_active() {
    let first = temp_dir_with_wav();
    let second = temp_dir_with_wav();
    let mut browser = FolderBrowserState::load_default();
    let mut workflow = SourceScanWorkflow::new();
    let active = workflow
        .begin_add_source_path(&mut browser, first.path().to_path_buf(), 50)
        .expect("active scan");
    workflow.start_scan(&active);
    let pending = browser
        .begin_add_source_path_preserving_selection(second.path().to_path_buf(), 51)
        .expect("pending source");

    assert!(matches!(
        workflow.begin_select_source(&mut browser, pending.source_id.clone(), 52),
        SourceSelectionRequest::Deferred
    ));
    assert!(workflow.pending_refresh_contains_for_tests(&pending.source_id));
}

#[test]
fn missing_selection_is_not_queued_for_permanent_refresh() {
    let first = temp_dir_with_wav();
    let second = temp_dir_with_wav();
    let mut browser = FolderBrowserState::load_default();
    let mut workflow = SourceScanWorkflow::new();
    let active = workflow
        .begin_add_source_path(&mut browser, first.path().to_path_buf(), 60)
        .expect("active scan");
    workflow.start_scan(&active);
    let pending = browser
        .begin_add_source_path_preserving_selection(second.path().to_path_buf(), 61)
        .expect("pending source");
    let source_id = pending.source_id.clone();
    drop(second);

    assert!(matches!(
        workflow.begin_select_source(&mut browser, source_id.clone(), 62),
        SourceSelectionRequest::Settled
    ));
    assert!(!workflow.pending_refresh_contains_for_tests(&source_id));
}

#[test]
fn duplicate_add_is_blocked_while_its_scan_is_active() {
    let root = temp_dir_with_wav();
    let mut browser = FolderBrowserState::load_default();
    let mut workflow = SourceScanWorkflow::new();
    let active = workflow
        .begin_add_source_path(&mut browser, root.path().to_path_buf(), 70)
        .expect("active scan");
    workflow.start_scan(&active);

    assert!(
        workflow
            .begin_add_source_path(&mut browser, root.path().to_path_buf(), 71)
            .is_none()
    );
    assert_eq!(workflow.progress().unwrap().task_id, active.task_id);
    assert!(browser.scan_is_active(&active.source_id, active.task_id));
}

#[test]
fn latest_deferred_selection_is_refreshed_first() {
    let first = temp_dir_with_wav();
    let second = temp_dir_with_wav();
    let third = temp_dir_with_wav();
    let mut browser = FolderBrowserState::load_default();
    let mut workflow = SourceScanWorkflow::new();
    let active = workflow
        .begin_add_source_path(&mut browser, first.path().to_path_buf(), 80)
        .expect("active scan");
    workflow.start_scan(&active);
    let second_request = browser
        .begin_add_source_path_preserving_selection(second.path().to_path_buf(), 81)
        .expect("second source");
    let third_request = browser
        .begin_add_source_path_preserving_selection(third.path().to_path_buf(), 82)
        .expect("third source");

    assert!(matches!(
        workflow.begin_select_source(&mut browser, second_request.source_id.clone(), 83),
        SourceSelectionRequest::Deferred
    ));
    assert!(matches!(
        workflow.begin_select_source(&mut browser, third_request.source_id.clone(), 84),
        SourceSelectionRequest::Deferred
    ));
    let result = scan_source_with_progress(active, |_| {}, |_| {});
    assert!(matches!(
        workflow.finish_scan(&mut browser, result),
        SourceScanFinish::Applied { .. }
    ));

    assert_eq!(
        workflow.next_pending_refresh_if_idle(),
        Some(third_request.source_id)
    );
    assert_eq!(
        workflow.next_pending_refresh_if_idle(),
        Some(second_request.source_id)
    );
}

#[test]
fn deferred_selection_missing_before_execution_is_dropped() {
    let first = temp_dir_with_wav();
    let second = temp_dir_with_wav();
    let mut browser = FolderBrowserState::load_default();
    let mut workflow = SourceScanWorkflow::new();
    let active = workflow
        .begin_add_source_path(&mut browser, first.path().to_path_buf(), 90)
        .expect("active scan");
    workflow.start_scan(&active);
    let pending = browser
        .begin_add_source_path_preserving_selection(second.path().to_path_buf(), 91)
        .expect("pending source");
    let source_id = pending.source_id.clone();
    assert!(matches!(
        workflow.begin_select_source(&mut browser, source_id.clone(), 92),
        SourceSelectionRequest::Deferred
    ));
    let result = scan_source_with_progress(active, |_| {}, |_| {});
    assert!(matches!(
        workflow.finish_scan(&mut browser, result),
        SourceScanFinish::Applied { .. }
    ));
    assert_eq!(
        workflow.next_pending_refresh_if_idle(),
        Some(source_id.clone())
    );
    drop(second);

    assert!(matches!(
        workflow.begin_filesystem_refresh(&mut browser, source_id.clone(), 93),
        SourceRefreshRequest::IgnoredMissing { .. }
    ));
    assert!(!workflow.pending_refresh_contains_for_tests(&source_id));
    assert_eq!(workflow.next_pending_refresh_if_idle(), None);
}

#[test]
fn new_source_add_is_registered_and_deferred_during_active_scan() {
    let first = temp_dir_with_wav();
    let second = temp_dir_with_wav();
    let mut browser = FolderBrowserState::load_default();
    let mut workflow = SourceScanWorkflow::new();
    let active = workflow
        .begin_add_source_path(&mut browser, first.path().to_path_buf(), 100)
        .expect("active scan");
    workflow.start_scan(&active);

    assert!(
        workflow
            .begin_add_source_path(&mut browser, second.path().to_path_buf(), 101)
            .is_none()
    );
    let deferred_id = browser
        .source_id_for_root_path(second.path())
        .expect("deferred source registered");
    assert!(workflow.pending_refresh_contains_for_tests(&deferred_id));

    let result = scan_source_with_progress(active, |_| {}, |_| {});
    assert!(matches!(
        workflow.finish_scan(&mut browser, result),
        SourceScanFinish::Applied { .. }
    ));
    assert_eq!(
        workflow.next_pending_refresh_if_idle(),
        Some(deferred_id.clone())
    );
    assert!(matches!(
        workflow.begin_filesystem_refresh(&mut browser, deferred_id, 102),
        SourceRefreshRequest::Queued(_)
    ));
}

#[test]
fn removed_pending_source_is_dropped_instead_of_requeued() {
    let first = temp_dir_with_wav();
    let second = temp_dir_with_wav();
    let mut browser = FolderBrowserState::load_default();
    let mut workflow = SourceScanWorkflow::new();
    let active = workflow
        .begin_add_source_path(&mut browser, first.path().to_path_buf(), 110)
        .expect("active scan");
    workflow.start_scan(&active);
    workflow.begin_add_source_path_preserving_selection(
        &mut browser,
        second.path().to_path_buf(),
        111,
    );
    let source_id = browser
        .source_id_for_root_path(second.path())
        .expect("deferred source");
    browser
        .remove_source(&source_id)
        .expect("remove deferred source");

    let result = scan_source_with_progress(active, |_| {}, |_| {});
    workflow.finish_scan(&mut browser, result);
    assert_eq!(
        workflow.next_pending_refresh_if_idle(),
        Some(source_id.clone())
    );
    assert!(matches!(
        workflow.begin_filesystem_refresh(&mut browser, source_id.clone(), 112),
        SourceRefreshRequest::IgnoredMissing { .. }
    ));
    assert!(!workflow.pending_refresh_contains_for_tests(&source_id));
}

#[test]
fn active_source_click_preserves_required_filesystem_refresh() {
    let root = temp_dir_with_wav();
    let mut browser = FolderBrowserState::load_default();
    let mut workflow = SourceScanWorkflow::new();
    let active = workflow
        .begin_add_source_path(&mut browser, root.path().to_path_buf(), 120)
        .expect("active scan");
    let source_id = active.source_id.clone();
    workflow.start_scan(&active);
    assert!(matches!(
        workflow.plan_filesystem_change(&mut browser, source_id.clone(), &[], true),
        SourceFilesystemChangePlan::DeferredAlreadyRunning { .. }
    ));

    assert!(matches!(
        workflow.begin_select_source(&mut browser, source_id.clone(), 121),
        SourceSelectionRequest::Settled
    ));
    assert!(workflow.pending_refresh_contains_for_tests(&source_id));
}

#[test]
fn switching_away_parks_live_discoveries_from_an_active_scan() {
    let first = temp_dir_with_wav();
    let second = temp_dir_with_wav();
    let mut browser = FolderBrowserState::load_default();
    let mut workflow = SourceScanWorkflow::new();
    let first_request = workflow
        .begin_add_source_path(&mut browser, first.path().to_path_buf(), 130)
        .expect("first source");
    let first_id = first_request.source_id.clone();
    workflow.start_scan(&first_request);
    let first_result = scan_source_with_progress(first_request, |_| {}, |_| {});
    workflow.finish_scan(&mut browser, first_result);
    let second_request = workflow
        .begin_add_source_path_preserving_selection(&mut browser, second.path().to_path_buf(), 131)
        .expect("second source");
    let second_id = second_request.source_id.clone();
    workflow.start_scan(&second_request);
    let second_result = scan_source_with_progress(second_request, |_| {}, |_| {});
    workflow.finish_scan(&mut browser, second_result);

    let active = workflow.begin_select_source(&mut browser, first_id.clone(), 132);
    let SourceSelectionRequest::Queued(active) = active else {
        panic!("first source rescan should queue");
    };
    workflow.start_scan(&active);
    let mut batches = Vec::new();
    let _result = scan_source_with_progress(active, |_| {}, |batch| batches.push(batch));
    for event in batches {
        workflow.apply_discovery_batch(
            &mut browser,
            FolderScanDiscoveryBatch {
                task_id: event.task_id,
                source_id: event.source_id.clone(),
                events: vec![event],
            },
        );
    }
    let visible_before = browser.selected_audio_files().len();
    assert!(visible_before > 0);

    workflow.begin_select_source(&mut browser, second_id, 133);
    workflow.begin_select_source(&mut browser, first_id, 134);

    assert_eq!(browser.selected_audio_files().len(), visible_before);
}
