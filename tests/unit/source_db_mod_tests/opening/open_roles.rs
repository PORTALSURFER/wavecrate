use super::*;

#[test]
fn ui_read_open_reads_existing_entries() {
    let dir = tempdir().unwrap();
    let db = SourceDatabase::open_for_source_write(dir.path()).unwrap();
    db.upsert_file(Path::new("one.wav"), 10, 5).unwrap();

    let read_only = SourceDatabase::open_for_ui_read(dir.path()).unwrap();
    let rows = read_only.list_files().unwrap();
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0].relative_path, PathBuf::from("one.wav"));
}

#[test]
fn ui_read_role_reads_existing_entries() {
    let dir = tempdir().unwrap();
    let db = SourceDatabase::open_for_source_write(dir.path()).unwrap();
    db.upsert_file(Path::new("one.wav"), 10, 5).unwrap();

    let ui_read =
        SourceDatabase::open_with_role(dir.path(), SourceDatabaseConnectionRole::UiRead).unwrap();
    let rows = ui_read.list_files().unwrap();
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0].relative_path, PathBuf::from("one.wav"));
}

#[test]
fn source_write_open_honors_read_only_environment_override() {
    let dir = match tempdir() {
        Ok(dir) => dir,
        Err(err) => panic!("tempdir failed: {err}"),
    };

    assert!(matches!(
        open_source_database(dir.path(), true, SourceDatabaseOpenMode::Full),
        Err(SourceDbError::ReadOnlyDatabaseMissing(_))
    ));
}
