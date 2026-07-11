use super::super::test_support::{
    dummy_controller, prepare_with_source_and_wav_entries, sample_entry,
};
use crate::app::state::StatusTone;

#[test]
fn adding_source_rejects_same_resolved_root_with_different_spelling() {
    let config_root = tempfile::tempdir().expect("create config root");
    let _guard = crate::app_dirs::ConfigBaseGuard::set(config_root.path().to_path_buf());
    let source_root = tempfile::tempdir().expect("create source root");
    let (mut controller, _) = dummy_controller();
    controller.library.sources.clear();

    controller
        .add_source_from_path(source_root.path().to_path_buf())
        .expect("add source");
    controller
        .add_source_from_path(source_root.path().join("."))
        .expect("duplicate source alias should short-circuit");

    assert_eq!(controller.library.sources.len(), 1);
    assert_eq!(controller.ui.status.text, "Source already added");
    assert_eq!(controller.ui.status.status_tone, StatusTone::Info);
}

#[test]
fn adding_source_rejects_nested_source_roots() {
    let config_root = tempfile::tempdir().expect("create config root");
    let _guard = crate::app_dirs::ConfigBaseGuard::set(config_root.path().to_path_buf());
    let library_root = tempfile::tempdir().expect("create library root");
    let parent = library_root.path().join("packs");
    let child = parent.join("drums");
    std::fs::create_dir_all(&child).expect("create nested source roots");

    let (mut controller, _) = dummy_controller();
    controller.library.sources.clear();
    controller
        .add_source_from_path(parent.clone())
        .expect("add parent source");

    let child_err = controller
        .add_source_from_path(child.clone())
        .expect_err("nested child source should be rejected");
    assert!(child_err.contains("Source folders cannot be nested"));
    assert!(child_err.contains("is inside existing source"));
    assert!(child_err.contains("Remove or remap the existing source"));
    assert_eq!(controller.library.sources.len(), 1);

    let (mut controller, _) = dummy_controller();
    controller.library.sources.clear();
    controller
        .add_source_from_path(child)
        .expect("add child source first");

    let parent_err = controller
        .add_source_from_path(parent)
        .expect_err("containing parent source should be rejected");
    assert!(parent_err.contains("Source folders cannot be nested"));
    assert!(parent_err.contains("contains existing source"));
    assert!(parent_err.contains("Remove or remap the existing source"));
    assert_eq!(controller.library.sources.len(), 1);
}

#[test]
/// Verifies removing source rolls back when config save fails.
fn removing_source_rolls_back_when_config_save_fails() {
    let (mut controller, source) = prepare_with_source_and_wav_entries(vec![sample_entry(
        "one.wav",
        crate::sample_sources::Rating::NEUTRAL,
    )]);
    controller.assign_source_to_folder_pane(
        crate::app::state::FolderPaneId::Upper,
        Some(source.id.clone()),
    );
    controller
        .selection_state
        .ctx
        .last_selected_browsable_source = Some(source.id.clone());

    let config_blocker = tempfile::NamedTempFile::new().expect("create config blocker file");
    let _guard = crate::app_dirs::ConfigBaseGuard::set(config_blocker.path().to_path_buf());

    controller.remove_source(0);

    assert_eq!(controller.library.sources.len(), 1);
    assert_eq!(controller.library.sources[0].id, source.id);
    assert_eq!(controller.selected_source_id(), Some(source.id.clone()));
    assert_eq!(controller.ui.status.status_tone, StatusTone::Error);
    assert!(
        controller
            .ui
            .status
            .text
            .contains("Failed to save config after removing source")
    );
    assert_ne!(controller.ui.status.text, "Source removed");
}

#[test]
fn remapping_source_snapshots_wal_resident_metadata() {
    let config_root = tempfile::tempdir().expect("create config root");
    let _guard = crate::app_dirs::ConfigBaseGuard::set(config_root.path().to_path_buf());
    let (mut controller, source) = prepare_with_source_and_wav_entries(vec![sample_entry(
        "wal.wav",
        crate::sample_sources::Rating::NEUTRAL,
    )]);
    let source_db = controller.cache.db.get(&source.id).unwrap().clone();
    source_db
        .set_tag(
            std::path::Path::new("wal.wav"),
            crate::sample_sources::Rating::KEEP_3,
        )
        .expect("commit WAL-resident tag");
    let source_wal = std::path::PathBuf::from(format!(
        "{}-wal",
        crate::sample_sources::database_path_for(&source.root).display()
    ));
    assert!(source_wal.metadata().expect("source WAL").len() > 0);
    let destination = tempfile::tempdir().expect("create destination root");

    controller
        .remap_source_to(0, destination.path().to_path_buf())
        .expect("remap source");

    let destination_db = crate::sample_sources::SourceDatabase::open(destination.path())
        .expect("open destination snapshot");
    let entry = destination_db
        .entry_for_path(std::path::Path::new("wal.wav"))
        .expect("query snapshot")
        .expect("snapshotted row");
    assert_eq!(entry.tag, crate::sample_sources::Rating::KEEP_3);
}
