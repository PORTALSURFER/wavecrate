use super::*;
#[test]
/// Single-sample rename restores the old path when the source DB is locked.
fn sample_rename_rolls_back_file_when_db_write_cannot_start() {
    let (_temp, source) = setup_fixture(&["old.wav"]);
    let old_relative = Path::new("old.wav");
    let new_relative = Path::new("renamed.wav");
    let old_absolute = source.root.join(old_relative);
    let new_absolute = source.root.join(new_relative);
    let (lock_release_tx, lock_done_rx) = lock_db_until_released(&source.root);

    let result = perform_sample_rename(
        &source,
        &old_absolute,
        old_relative,
        new_relative,
        Rating::KEEP_3,
        RenameLoopedMetadata::DbOrFallback(false),
        false,
        None,
        Some(SampleSoundType::Kick),
        Some(String::from("Vintage")),
        None,
    );

    release_db_lock(lock_release_tx, lock_done_rx);

    let err = result.expect_err("locked DB should fail rename");
    assert_db_contention_error(&err);
    assert!(old_absolute.is_file());
    assert!(!new_absolute.exists());

    let db = SourceDatabase::open(&source.root).expect("open source db");
    assert_eq!(
        db.tag_for_path(old_relative).expect("old tag"),
        Some(Rating::KEEP_3)
    );
    assert_eq!(
        db.looped_for_path(old_relative).expect("old looped"),
        Some(true)
    );
    assert_eq!(
        db.locked_for_path(old_relative).expect("old locked"),
        Some(true)
    );
    assert_eq!(
        db.sound_type_for_path(old_relative)
            .expect("old sound type"),
        Some(SampleSoundType::Kick)
    );
    assert_eq!(
        db.user_tag_for_path(old_relative).expect("old user tag"),
        Some(String::from("Vintage"))
    );
    assert_eq!(
        db.tag_labels_for_path(old_relative)
            .expect("old normal tags"),
        vec![String::from("Analog Kick"), String::from("Layer")]
    );
    assert_eq!(
        db.last_played_at_for_path(old_relative)
            .expect("old playback age"),
        Some(42)
    );
    assert!(db.tag_for_path(new_relative).expect("new tag").is_none());
    assert!(
        db.tag_labels_for_path(new_relative)
            .expect("new normal tags")
            .is_empty()
    );
}

#[test]
/// Successful sample rename keeps the locked flag and other metadata on the new DB row.
fn sample_rename_preserves_locked_and_metadata_on_success() {
    let (_temp, source) = setup_fixture(&["old.wav"]);
    let old_relative = Path::new("old.wav");
    let new_relative = Path::new("renamed.wav");
    let old_absolute = source.root.join(old_relative);
    let new_absolute = source.root.join(new_relative);

    let entry = perform_sample_rename(
        &source,
        &old_absolute,
        old_relative,
        new_relative,
        Rating::KEEP_3,
        RenameLoopedMetadata::DbOrFallback(false),
        false,
        None,
        Some(SampleSoundType::Kick),
        Some(String::from("Vintage")),
        None,
    )
    .expect("rename should succeed");

    assert_eq!(entry.relative_path, PathBuf::from("renamed.wav"));
    assert!(entry.looped);
    assert!(entry.locked);
    assert_eq!(entry.sound_type, Some(SampleSoundType::Kick));
    assert_eq!(entry.user_tag.as_deref(), Some("Vintage"));
    assert_eq!(entry.normal_tags, vec!["Analog Kick", "Layer"]);
    assert_eq!(entry.last_played_at, Some(42));
    assert!(!old_absolute.exists());
    assert!(new_absolute.is_file());

    let db = SourceDatabase::open(&source.root).expect("open source db");
    assert!(db.tag_for_path(old_relative).expect("old tag").is_none());
    assert_eq!(
        db.tag_for_path(new_relative).expect("new tag"),
        Some(Rating::KEEP_3)
    );
    assert_eq!(
        db.looped_for_path(new_relative).expect("new looped"),
        Some(true)
    );
    assert_eq!(
        db.locked_for_path(new_relative).expect("new locked"),
        Some(true)
    );
    assert_eq!(
        db.sound_type_for_path(new_relative)
            .expect("new sound type"),
        Some(SampleSoundType::Kick)
    );
    assert_eq!(
        db.user_tag_for_path(new_relative).expect("new user tag"),
        Some(String::from("Vintage"))
    );
    assert_eq!(
        db.tag_labels_for_path(new_relative)
            .expect("new normal tags"),
        vec![String::from("Analog Kick"), String::from("Layer")]
    );
    assert_eq!(
        db.last_played_at_for_path(new_relative)
            .expect("new playback age"),
        Some(42)
    );
}
