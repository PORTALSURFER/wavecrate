use std::path::Path;

use tempfile::tempdir;

use super::{SourceDatabase, SourceDatabaseConnectionRole};

#[test]
fn role_scoped_entrypoints_preserve_read_write_contracts() {
    let dir = tempdir().unwrap();
    let writer = SourceDatabase::open_for_source_write(dir.path()).unwrap();
    writer.upsert_file(Path::new("one.wav"), 10, 5).unwrap();

    let ui_read = SourceDatabase::open_for_ui_read(dir.path()).unwrap();
    assert_eq!(ui_read.count_files().unwrap(), 1);

    let background = SourceDatabase::open_for_background_job(dir.path()).unwrap();
    background
        .set_metadata("contract_probe", "background")
        .unwrap();

    let maintenance = SourceDatabase::open_for_maintenance(dir.path()).unwrap();
    assert_eq!(
        maintenance
            .get_metadata("contract_probe")
            .unwrap()
            .as_deref(),
        Some("background")
    );
}

#[test]
fn role_profiles_apply_expected_access_and_busy_timeouts() {
    let source_root = tempdir().unwrap();
    let database_root = tempdir().unwrap();
    let setup = SourceDatabase::open_for_source_write_with_database_root(
        source_root.path(),
        database_root.path(),
    )
    .unwrap();
    setup.upsert_file(Path::new("one.wav"), 10, 5).unwrap();
    drop(setup);

    for (role, expected_timeout_ms, writable) in [
        (SourceDatabaseConnectionRole::UiRead, 25, false),
        (SourceDatabaseConnectionRole::BackgroundRead, 5_000, false),
        (SourceDatabaseConnectionRole::JobWorker, 5_000, true),
        (SourceDatabaseConnectionRole::UserMetadataWrite, 5_000, true),
        (
            SourceDatabaseConnectionRole::PlaybackHistoryWrite,
            100,
            true,
        ),
        (SourceDatabaseConnectionRole::Maintenance, 5_000, true),
    ] {
        let database = SourceDatabase::open_with_role_and_database_root(
            source_root.path(),
            database_root.path(),
            role,
        )
        .unwrap();
        let busy_timeout_ms: u64 = database
            .connection
            .query_row("PRAGMA busy_timeout", [], |row| row.get(0))
            .unwrap();
        assert_eq!(busy_timeout_ms, expected_timeout_ms, "role: {role:?}");

        let write_result = database.set_metadata("role_probe", "value");
        assert_eq!(write_result.is_ok(), writable, "role: {role:?}");
    }
}
