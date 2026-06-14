use super::*;

#[test]
fn read_only_open_reads_existing_entries() {
    let dir = tempdir().unwrap();
    let db = SourceDatabase::open(dir.path()).unwrap();
    db.upsert_file(Path::new("one.wav"), 10, 5).unwrap();

    let read_only = SourceDatabase::open_read_only(dir.path()).unwrap();
    let rows = read_only.list_files().unwrap();
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0].relative_path, PathBuf::from("one.wav"));
}

#[test]
fn ui_read_role_reads_existing_entries() {
    let dir = tempdir().unwrap();
    let db = SourceDatabase::open(dir.path()).unwrap();
    db.upsert_file(Path::new("one.wav"), 10, 5).unwrap();

    let ui_read =
        SourceDatabase::open_with_role(dir.path(), SourceDatabaseConnectionRole::UiRead).unwrap();
    let rows = ui_read.list_files().unwrap();
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0].relative_path, PathBuf::from("one.wav"));
}

#[test]
fn open_defaults_to_read_only_when_enabled() {
    let dir = match tempdir() {
        Ok(dir) => dir,
        Err(err) => panic!("tempdir failed: {err}"),
    };

    assert!(matches!(
        open_source_database(dir.path(), true, false, SourceDatabaseOpenMode::Full),
        Err(SourceDbError::ReadOnlyDatabaseMissing(_))
    ));
}
