use super::*;
use crate::app::controller::test_support::dummy_controller;
use crate::sample_sources::SampleSource;
use std::fs;
use tempfile::tempdir;

#[test]
fn applying_recovery_report_updates_ui_entries() {
    let (mut controller, source) = dummy_controller();
    controller.library.sources.push(source.clone());
    controller.ui.sources.folders.delete_recovery.in_progress = true;
    let report = DeleteRecoveryReport {
        entries: vec![DeleteRecoveryEntry {
            source_id: source.id.clone(),
            source_root: source.root.clone(),
            original_relative: "gone".into(),
            action: DeleteRecoveryAction::Restore,
            status: DeleteRecoveryStatus::Completed,
            detail: Some("Already restored".into()),
        }],
        retained_entries: Vec::new(),
        scan_sources: Vec::new(),
        errors: Vec::new(),
    };

    controller.apply_folder_delete_recovery_report(report);

    assert!(!controller.ui.sources.folders.delete_recovery.in_progress);
    assert_eq!(
        controller.ui.sources.folders.delete_recovery.entries.len(),
        1
    );
    let entry = &controller.ui.sources.folders.delete_recovery.entries[0];
    assert_eq!(entry.source_label, "source");
    assert_eq!(entry.detail.as_deref(), Some("Already restored"));
}

#[test]
fn clear_folder_delete_recovery_log_removes_entries() {
    let (mut controller, source) = dummy_controller();
    controller
        .ui
        .sources
        .folders
        .delete_recovery
        .entries
        .push(UiDeleteRecoveryEntry {
            source_label: source.root.to_string_lossy().to_string(),
            relative_path: "gone".into(),
            action: UiDeleteRecoveryAction::Restore,
            status: UiDeleteRecoveryStatus::Completed,
            detail: None,
        });

    controller.clear_folder_delete_recovery_log();

    assert!(
        controller
            .ui
            .sources
            .folders
            .delete_recovery
            .entries
            .is_empty()
    );
}

#[test]
fn applying_recovery_report_requests_hard_sync_for_scan_sources() {
    let (mut controller, source) = dummy_controller();
    controller.library.sources.push(source.clone());

    controller.apply_folder_delete_recovery_report(DeleteRecoveryReport {
        entries: Vec::new(),
        retained_entries: Vec::new(),
        scan_sources: vec![source.id.clone()],
        errors: Vec::new(),
    });

    assert!(controller.runtime.jobs.scan_in_progress());
    assert_eq!(
        controller.ui.progress.task,
        Some(crate::app::state::ProgressTaskKind::Scan)
    );
}

#[test]
fn applying_recovery_uses_source_name_when_source_is_still_loaded() {
    let (mut controller, source) = named_source_controller("Drums");
    controller.ui.sources.folders.delete_recovery.in_progress = true;

    controller.apply_folder_delete_recovery_report(DeleteRecoveryReport {
        entries: vec![DeleteRecoveryEntry {
            source_id: source.id.clone(),
            source_root: source.root.clone(),
            original_relative: "gone".into(),
            action: DeleteRecoveryAction::Finalize,
            status: DeleteRecoveryStatus::Completed,
            detail: None,
        }],
        retained_entries: Vec::new(),
        scan_sources: Vec::new(),
        errors: Vec::new(),
    });

    assert_eq!(
        controller
            .ui
            .sources
            .folders
            .delete_recovery
            .retained_entries
            .len(),
        0
    );
    assert_eq!(
        controller.ui.sources.folders.delete_recovery.entries[0].source_label,
        "Drums"
    );
}

#[test]
fn applying_recovery_report_tracks_retained_delete_entries() {
    let (mut controller, source) = named_source_controller("Drums");
    let deleted_entries = vec![crate::sample_sources::WavEntry {
        relative_path: "gone/kick.wav".into(),
        file_size: 42,
        modified_ns: 9,
        content_hash: Some("hash".into()),
        tag: crate::sample_sources::Rating::KEEP_3,
        looped: true,
        locked: true,
        missing: false,
        last_played_at: Some(12),
    }];

    controller.apply_folder_delete_recovery_report(DeleteRecoveryReport {
        entries: Vec::new(),
        retained_entries: vec![RetainedDeleteEntry {
            id: "retained-1".into(),
            source_id: source.id.clone(),
            source_root: source.root.clone(),
            original_relative: "gone".into(),
            staged_relative: "gone".into(),
            deleted_entries: deleted_entries.clone(),
        }],
        scan_sources: Vec::new(),
        errors: Vec::new(),
    });

    assert_eq!(
        controller
            .ui
            .sources
            .folders
            .delete_recovery
            .retained_entries
            .len(),
        1
    );
    let entry = &controller
        .ui
        .sources
        .folders
        .delete_recovery
        .retained_entries[0];
    assert_eq!(entry.source_label, "Drums");
    assert_eq!(entry.relative_path, std::path::Path::new("gone"));
    assert_eq!(entry.deleted_entries.len(), 1);
    assert_eq!(
        entry.deleted_entries[0].relative_path,
        deleted_entries[0].relative_path
    );
    assert_eq!(
        entry.deleted_entries[0].content_hash.as_deref(),
        deleted_entries[0].content_hash.as_deref()
    );
    assert_eq!(
        entry.deleted_entries[0].tag.val(),
        deleted_entries[0].tag.val()
    );
    assert_eq!(entry.deleted_entries[0].looped, deleted_entries[0].looped);
    assert_eq!(entry.deleted_entries[0].locked, deleted_entries[0].locked);
    assert_eq!(
        entry.deleted_entries[0].last_played_at,
        deleted_entries[0].last_played_at
    );
}

fn named_source_controller(name: &str) -> (AppController, SampleSource) {
    let (mut controller, source) = dummy_controller();
    let dir = tempdir().unwrap();
    let root = dir.path().join(name);
    fs::create_dir_all(&root).unwrap();
    std::mem::forget(dir);
    let source = SampleSource { root, ..source };
    controller.library.sources.push(source.clone());
    (controller, source)
}
