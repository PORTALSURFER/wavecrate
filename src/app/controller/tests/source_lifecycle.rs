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

#[test]
fn remapping_source_rolls_back_root_and_snapshot_when_config_save_fails() {
    let (mut controller, source) = prepare_with_source_and_wav_entries(vec![sample_entry(
        "rollback.wav",
        crate::sample_sources::Rating::KEEP_1,
    )]);
    let destination = tempfile::tempdir().expect("destination");
    let config_blocker = tempfile::NamedTempFile::new().expect("config blocker");
    let _guard = crate::app_dirs::ConfigBaseGuard::set(config_blocker.path().to_path_buf());

    let error = controller
        .remap_source_to(0, destination.path().to_path_buf())
        .expect_err("config save should fail");

    assert!(error.contains("Failed to save config after remapping source"));
    assert_eq!(controller.library.sources[0].root, source.root);
    assert!(!crate::sample_sources::database_path_for(destination.path()).exists());
}

#[test]
fn remapping_source_preserves_and_migrates_legacy_destination_database() {
    let config_root = tempfile::tempdir().expect("create config root");
    let _guard = crate::app_dirs::ConfigBaseGuard::set(config_root.path().to_path_buf());
    let (mut controller, source) = prepare_with_source_and_wav_entries(vec![sample_entry(
        "source.wav",
        crate::sample_sources::Rating::NEUTRAL,
    )]);
    controller.cache.db.remove(&source.id);
    std::fs::remove_file(crate::sample_sources::database_path_for(&source.root))
        .expect("remove source database");
    let destination = tempfile::tempdir().expect("destination root");
    let legacy = destination
        .path()
        .join(crate::sample_sources::db::LEGACY_DB_FILE_NAME);
    let destination_db = crate::sample_sources::SourceDatabase::open(destination.path())
        .expect("destination database");
    destination_db
        .upsert_file(std::path::Path::new("legacy.wav"), 10, 5)
        .expect("legacy row");
    drop(destination_db);
    std::fs::rename(
        crate::sample_sources::database_path_for(destination.path()),
        &legacy,
    )
    .expect("rename current database to legacy name");

    controller
        .remap_source_to(0, destination.path().to_path_buf())
        .expect("remap source");

    let destination_db = crate::sample_sources::SourceDatabase::open(destination.path())
        .expect("migrated destination database");
    assert!(
        destination_db
            .entry_for_path(std::path::Path::new("legacy.wav"))
            .expect("legacy query")
            .is_some()
    );
}

#[test]
fn remapping_source_rejects_destination_owned_by_pending_add() {
    let (mut controller, source) = prepare_with_source_and_wav_entries(vec![sample_entry(
        "one.wav",
        crate::sample_sources::Rating::NEUTRAL,
    )]);
    let destination = tempfile::tempdir().expect("destination root");
    let pending_source = crate::sample_sources::SampleSource::new(destination.path().to_path_buf());
    controller.runtime.source_lane.pending_adds.insert(
        pending_source.root.clone(),
        crate::app::controller::state::runtime::PendingSourceAdd {
            request_id: 77,
            source: pending_source,
            queued_at: std::time::Instant::now(),
        },
    );

    let error = controller
        .remap_source_to(0, destination.path().to_path_buf())
        .expect_err("pending add destination must reject remap");

    assert!(error.contains("being added"));
    assert_eq!(controller.library.sources[0].root, source.root);
}

#[test]
fn removing_source_cancels_matching_pending_remap_generation() {
    let config_root = tempfile::tempdir().expect("config root");
    let _guard = crate::app_dirs::ConfigBaseGuard::set(config_root.path().to_path_buf());
    let (mut controller, source) = prepare_with_source_and_wav_entries(vec![sample_entry(
        "pending.wav",
        crate::sample_sources::Rating::NEUTRAL,
    )]);
    controller.runtime.source_lane.pending_remap =
        Some(crate::app::controller::state::runtime::PendingSourceRemap {
            request_id: 41,
            source: source.clone(),
            new_root: tempfile::tempdir().expect("destination").keep(),
            queued_at: std::time::Instant::now(),
            canceled: false,
        });

    controller.remove_source(0);

    assert!(
        controller
            .runtime
            .source_lane
            .pending_remap
            .as_ref()
            .is_some_and(|pending| pending.request_id == 41 && pending.canceled)
    );
}

#[test]
fn source_mutation_cancels_matching_pending_remap_generation() {
    let config_root = tempfile::tempdir().expect("config root");
    let _guard = crate::app_dirs::ConfigBaseGuard::set(config_root.path().to_path_buf());
    let (mut controller, source) = prepare_with_source_and_wav_entries(vec![sample_entry(
        "pending.wav",
        crate::sample_sources::Rating::NEUTRAL,
    )]);
    controller.runtime.source_lane.pending_remap =
        Some(crate::app::controller::state::runtime::PendingSourceRemap {
            request_id: 42,
            source: source.clone(),
            new_root: tempfile::tempdir().expect("destination").keep(),
            queued_at: std::time::Instant::now(),
            canceled: false,
        });

    controller.begin_pending_file_mutation(&source.id, [std::path::PathBuf::from("pending.wav")]);

    assert!(
        controller
            .runtime
            .source_lane
            .pending_remap
            .as_ref()
            .is_some_and(|pending| pending.request_id == 42 && pending.canceled)
    );
    controller.finish_pending_file_mutation(&source.id, [std::path::PathBuf::from("pending.wav")]);
}

#[test]
fn metadata_mutation_cancels_matching_pending_remap_generation() {
    let config_root = tempfile::tempdir().expect("config root");
    let _guard = crate::app_dirs::ConfigBaseGuard::set(config_root.path().to_path_buf());
    let (mut controller, source) = prepare_with_source_and_wav_entries(vec![sample_entry(
        "pending.wav",
        crate::sample_sources::Rating::NEUTRAL,
    )]);
    controller.runtime.source_lane.pending_remap =
        Some(crate::app::controller::state::runtime::PendingSourceRemap {
            request_id: 43,
            source: source.clone(),
            new_root: tempfile::tempdir().expect("destination").keep(),
            queued_at: std::time::Instant::now(),
            canceled: false,
        });

    controller.queue_metadata_mutation(
        &source,
        vec![
            crate::app::controller::jobs::SourceMetadataMutationOp::SetTagAndLocked {
                relative_path: std::path::PathBuf::from("pending.wav"),
                tag: crate::sample_sources::Rating::KEEP_1,
                locked: false,
            },
        ],
        Vec::new(),
        Vec::new(),
        false,
    );

    assert!(
        controller
            .runtime
            .source_lane
            .pending_remap
            .as_ref()
            .is_some_and(|pending| pending.request_id == 43 && pending.canceled)
    );
}
