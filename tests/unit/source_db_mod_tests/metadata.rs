use super::*;

#[test]
fn tags_default_and_persist() {
    let dir = tempdir().unwrap();
    let db = SourceDatabase::open(dir.path()).unwrap();
    db.upsert_file(Path::new("one.wav"), 10, 5).unwrap();

    let first = db.list_files().unwrap();
    assert_eq!(first[0].tag, Rating::NEUTRAL);
    assert!(!first[0].looped);
    assert!(!first[0].missing);

    db.set_tag(Path::new("one.wav"), Rating::KEEP_1).unwrap();
    let second = db.list_files().unwrap();
    assert_eq!(second[0].tag, Rating::KEEP_1);
    assert!(!second[0].looped);
    assert!(!second[0].missing);

    db.upsert_file(Path::new("one.wav"), 12, 6).unwrap();
    let third = db.list_files().unwrap();
    assert_eq!(third[0].tag, Rating::KEEP_1);
    assert!(!third[0].missing);

    let reopened = SourceDatabase::open(dir.path()).unwrap();
    let fourth = reopened.list_files().unwrap();
    assert_eq!(fourth[0].tag, Rating::KEEP_1);
    assert!(!fourth[0].looped);
    assert!(!fourth[0].missing);
}

#[test]
fn loop_markers_default_and_persist() {
    let dir = tempdir().unwrap();
    let db = SourceDatabase::open(dir.path()).unwrap();
    db.upsert_file(Path::new("loop.wav"), 10, 5).unwrap();

    let first = db.list_files().unwrap();
    assert!(!first[0].looped);

    db.set_looped(Path::new("loop.wav"), true).unwrap();
    let second = db.list_files().unwrap();
    assert!(second[0].looped);

    db.upsert_file(Path::new("loop.wav"), 12, 6).unwrap();
    let third = db.list_files().unwrap();
    assert!(third[0].looped);

    let reopened = SourceDatabase::open(dir.path()).unwrap();
    let fourth = reopened.list_files().unwrap();
    assert!(fourth[0].looped);
}

#[test]
fn lock_markers_default_and_persist() {
    let dir = tempdir().unwrap();
    let db = SourceDatabase::open(dir.path()).unwrap();
    db.upsert_file(Path::new("lock.wav"), 10, 5).unwrap();

    let first = db.list_files().unwrap();
    assert!(!first[0].locked);

    db.set_locked(Path::new("lock.wav"), true).unwrap();
    let second = db.list_files().unwrap();
    assert!(second[0].locked);

    db.upsert_file(Path::new("lock.wav"), 12, 6).unwrap();
    let third = db.list_files().unwrap();
    assert!(third[0].locked);

    let reopened = SourceDatabase::open(dir.path()).unwrap();
    let fourth = reopened.list_files().unwrap();
    assert!(fourth[0].locked);
}

#[test]
fn batch_tag_updates_coalesce_to_latest_value() {
    let dir = tempdir().unwrap();
    let db = SourceDatabase::open(dir.path()).unwrap();
    db.upsert_file(Path::new("one.wav"), 10, 5).unwrap();

    db.set_tags_batch(&[
        (PathBuf::from("one.wav"), Rating::KEEP_1),
        (PathBuf::from("one.wav"), Rating::TRASH_1),
    ])
    .unwrap();

    let rows = db.list_files().unwrap();
    assert_eq!(rows[0].tag, Rating::TRASH_1);
}

#[test]
fn missing_flag_round_trips() {
    let dir = tempdir().unwrap();
    let db = SourceDatabase::open(dir.path()).unwrap();
    db.upsert_file(Path::new("one.wav"), 10, 5).unwrap();
    db.set_missing(Path::new("one.wav"), true).unwrap();
    let rows = db.list_files().unwrap();
    assert!(rows[0].missing);
    db.set_missing(Path::new("one.wav"), false).unwrap();
    let rows = db.list_files().unwrap();
    assert!(!rows[0].missing);
}

#[test]
fn list_and_count_only_show_supported_audio() {
    let dir = tempdir().unwrap();
    let db = SourceDatabase::open(dir.path()).unwrap();
    db.upsert_file(Path::new("one.wav"), 10, 5).unwrap();
    db.upsert_file(Path::new("notes.txt"), 1, 1).unwrap();

    let rows = db.list_files().unwrap();
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0].relative_path, PathBuf::from("one.wav"));
    assert_eq!(db.count_files().unwrap(), 1);
    assert!(db.index_for_path(Path::new("notes.txt")).unwrap().is_none());
}

#[test]
fn batch_bpm_lookup_returns_requested_sample_rows() {
    let dir = tempdir().unwrap();
    let db = SourceDatabase::open(dir.path()).unwrap();
    db.connection
        .execute(
            "INSERT INTO samples (sample_id, content_hash, size, mtime_ns, bpm)
                 VALUES (?1, 'h1', 1, 1, ?2)",
            params!["source::one.wav", 124.0f64],
        )
        .unwrap();
    db.connection
        .execute(
            "INSERT INTO samples (sample_id, content_hash, size, mtime_ns, bpm)
                 VALUES (?1, 'h2', 1, 1, NULL)",
            params!["source::two.wav"],
        )
        .unwrap();

    let lookup = db
        .bpms_for_sample_ids(&[
            String::from("source::one.wav"),
            String::from("source::two.wav"),
            String::from("source::missing.wav"),
        ])
        .unwrap();

    assert_eq!(lookup.get("source::one.wav"), Some(&Some(124.0)));
    assert_eq!(lookup.get("source::two.wav"), Some(&None));
    assert!(!lookup.contains_key("source::missing.wav"));
}
