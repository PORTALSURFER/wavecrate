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
    let persisted = crate::sample_sources::config::load_or_default().expect("load saved config");
    assert_eq!(
        persisted.core.last_selected_source,
        controller.selected_source_id()
    );
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
fn adding_source_publishes_no_runtime_state_when_config_save_fails() {
    let (mut controller, original_source) = dummy_controller();
    controller.library.sources.push(original_source.clone());
    controller.cache_db(&original_source).unwrap();
    let added_root = tempfile::tempdir().expect("create source root");
    let selected_before = controller.selected_source_id();
    let source_count_before = controller.library.sources.len();
    let db_cache_count_before = controller.cache.db.len();
    let ui_rows_before = controller.ui.sources.rows.len();
    let config_blocker = tempfile::NamedTempFile::new().expect("create config blocker file");
    let _guard = crate::app_dirs::ConfigBaseGuard::set(config_blocker.path().to_path_buf());

    let error = controller
        .add_source_from_path(added_root.path().to_path_buf())
        .expect_err("config persistence must fail");

    assert!(error.contains("Failed to save config after adding source"));
    assert_eq!(controller.library.sources.len(), source_count_before);
    assert_eq!(controller.library.sources[0].id, original_source.id);
    assert_eq!(controller.selected_source_id(), selected_before);
    assert_eq!(controller.cache.db.len(), db_cache_count_before);
    assert_eq!(controller.ui.sources.rows.len(), ui_rows_before);
}

#[test]
fn remapping_source_rolls_back_runtime_and_created_artifacts_when_config_save_fails() {
    let (mut controller, source) = prepare_with_source_and_wav_entries(vec![sample_entry(
        "one.wav",
        crate::sample_sources::Rating::KEEP_1,
    )]);
    let old_root = source.root.clone();
    let new_root = tempfile::tempdir().expect("create remap root");
    let new_database = crate::sample_sources::database_path_for(new_root.path());
    let selected_before = controller.selected_source_id();
    let db_cache_before = controller.cache.db.get(&source.id).unwrap().clone();
    let wav_cache_count_before = controller.cache.wav.entries.len();
    let missing_before = controller.library.missing.sources.clone();
    let config_blocker = tempfile::NamedTempFile::new().expect("create config blocker file");
    let _guard = crate::app_dirs::ConfigBaseGuard::set(config_blocker.path().to_path_buf());

    let error = controller
        .remap_source_to(0, new_root.path().to_path_buf())
        .expect_err("config persistence must fail");

    assert!(error.contains("Failed to save config after remapping source"));
    assert_eq!(controller.library.sources[0].root, old_root);
    assert_eq!(controller.selected_source_id(), selected_before);
    assert!(std::rc::Rc::ptr_eq(
        controller.cache.db.get(&source.id).unwrap(),
        &db_cache_before
    ));
    assert_eq!(controller.cache.wav.entries.len(), wav_cache_count_before);
    assert_eq!(controller.library.missing.sources, missing_before);
    for artifact in [
        new_database.clone(),
        std::path::PathBuf::from(format!("{}-wal", new_database.display())),
        std::path::PathBuf::from(format!("{}-shm", new_database.display())),
        std::path::PathBuf::from(format!("{}-journal", new_database.display())),
    ] {
        assert!(
            !artifact.exists(),
            "orphan artifact: {}",
            artifact.display()
        );
    }
}

#[test]
fn remapping_source_publishes_runtime_changes_after_persistence_and_db_prepare() {
    let config_root = tempfile::tempdir().expect("create config root");
    let _guard = crate::app_dirs::ConfigBaseGuard::set(config_root.path().to_path_buf());
    let (mut controller, source) = prepare_with_source_and_wav_entries(vec![sample_entry(
        "one.wav",
        crate::sample_sources::Rating::KEEP_1,
    )]);
    let new_root = tempfile::tempdir().expect("create remap root");

    controller
        .remap_source_to(0, new_root.path().to_path_buf())
        .expect("remap source");

    assert_eq!(controller.library.sources[0].root, new_root.path());
    assert_eq!(controller.selected_source_id(), Some(source.id.clone()));
    assert_eq!(
        controller
            .cache
            .db
            .get(&source.id)
            .expect("remapped database should be cached")
            .root(),
        new_root.path()
    );
    assert!(crate::sample_sources::database_path_for(new_root.path()).is_file());
    assert_eq!(controller.ui.status.text, "Source remapped");
    assert_eq!(controller.ui.status.status_tone, StatusTone::Info);
}

#[test]
fn failed_remap_forgets_target_root_mapping() {
    let config_root = tempfile::tempdir().expect("create config root");
    let _guard = crate::app_dirs::ConfigBaseGuard::set(config_root.path().to_path_buf());
    let (mut controller, source) = prepare_with_source_and_wav_entries(vec![sample_entry(
        "one.wav",
        crate::sample_sources::Rating::KEEP_1,
    )]);
    let old_root = source.root.clone();
    let new_root = tempfile::tempdir().expect("create remap root");
    std::fs::create_dir(crate::sample_sources::database_path_for(new_root.path()))
        .expect("block destination database open");

    controller
        .remap_source_to(0, new_root.path().to_path_buf())
        .expect_err("database preparation must fail");

    assert_eq!(controller.library.sources[0].root, old_root);
    assert_eq!(
        crate::sample_sources::library::lookup_source_id_for_root(new_root.path())
            .expect("lookup failed remap root"),
        None
    );
}

#[test]
fn failed_remap_restores_preexisting_legacy_database_name() {
    let config_root = tempfile::tempdir().expect("create config root");
    let _guard = crate::app_dirs::ConfigBaseGuard::set(config_root.path().to_path_buf());
    let (mut controller, source) = prepare_with_source_and_wav_entries(vec![sample_entry(
        "one.wav",
        crate::sample_sources::Rating::KEEP_1,
    )]);
    controller.cache.db.remove(&source.id);
    std::fs::remove_file(crate::sample_sources::database_path_for(&source.root))
        .expect("remove source database so remap does not copy it");
    let new_root = tempfile::tempdir().expect("create remap root");
    let legacy = new_root
        .path()
        .join(crate::sample_sources::db::LEGACY_DB_FILE_NAME);
    {
        let connection = rusqlite::Connection::open(&legacy).expect("create legacy database");
        connection
            .execute_batch("CREATE TABLE analysis_jobs (id INTEGER PRIMARY KEY);")
            .expect("create incompatible legacy schema");
    }

    controller
        .remap_source_to(0, new_root.path().to_path_buf())
        .expect_err("invalid migrated legacy database must fail preparation");

    assert!(legacy.is_file(), "legacy database name must be restored");
    assert!(!crate::sample_sources::database_path_for(new_root.path()).exists());
}
