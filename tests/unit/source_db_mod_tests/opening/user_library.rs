use super::*;

#[test]
fn open_blocks_writes_for_user_library_roots_without_override() {
    let home = match tempdir() {
        Ok(home) => home,
        Err(err) => panic!("tempdir failed: {err}"),
    };
    let user_home = home.path().join("home");
    let user_music = user_home.join("Music");
    if let Err(err) = std::fs::create_dir_all(&user_music) {
        panic!("create fake user library dir failed: {err}");
    }
    with_home_env_override(&user_home, || {
        let blocked = open_source_database(&user_music, false, false, SourceDatabaseOpenMode::Full);
        assert!(matches!(
            blocked,
            Err(SourceDbError::UserLibraryWriteBlocked { .. })
        ));

        let db = open_source_database(&user_music, false, true, SourceDatabaseOpenMode::Full);
        assert!(db.is_ok());
        let opened = match db {
            Ok(opened) => opened,
            Err(err) => panic!("db open with override should be allowed: {err}"),
        };
        assert_eq!(opened.root(), user_music.as_path());
    });
}

#[test]
fn user_metadata_write_role_allows_configured_user_library_roots() {
    let home = match tempdir() {
        Ok(home) => home,
        Err(err) => panic!("tempdir failed: {err}"),
    };
    let user_home = home.path().join("home");
    let user_documents = user_home.join("Documents");
    if let Err(err) = std::fs::create_dir_all(&user_documents) {
        panic!("create fake user library dir failed: {err}");
    }
    with_home_env_override(&user_home, || {
        let db = SourceDatabase::open_for_user_metadata_write(&user_documents);
        assert!(db.is_ok());
        let opened = match db {
            Ok(opened) => opened,
            Err(err) => panic!("user metadata write should be allowed: {err}"),
        };
        assert_eq!(opened.root(), user_documents.as_path());
    });
}
