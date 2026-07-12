use super::*;
use crate::sample_sources::db::SampleSoundType;

#[test]
fn scan_detects_rename_and_preserves_tag() {
    let dir = tempdir().unwrap();
    let first_path = dir.path().join("one.wav");
    let second_path = dir.path().join("two.wav");
    std::fs::write(&first_path, b"one").unwrap();

    let db = SourceDatabase::open(dir.path()).unwrap();
    scan_once(&db).unwrap();
    db.set_tag(Path::new("one.wav"), Rating::KEEP_1).unwrap();
    db.set_sound_type(Path::new("one.wav"), Some(SampleSoundType::Kick))
        .unwrap();
    db.set_user_tag(Path::new("one.wav"), Some("Vintage FX"))
        .unwrap();
    db.assign_tag_to_path(Path::new("one.wav"), "Analog Kick")
        .unwrap();
    insert_analysis_artifacts(dir.path(), "source::one.wav", "one.wav");

    std::fs::rename(&first_path, &second_path).unwrap();
    let stats = scan_once(&db).unwrap();

    assert_eq!(stats.missing, 0);
    assert_eq!(stats.added, 0);
    assert_eq!(stats.content_changed, 0);
    assert_eq!(stats.updated, 1);

    let rows = db.list_files().unwrap();
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0].relative_path, Path::new("two.wav"));
    assert_eq!(rows[0].tag, Rating::KEEP_1);
    assert_eq!(rows[0].sound_type, Some(SampleSoundType::Kick));
    assert_eq!(rows[0].user_tag.as_deref(), Some("Vintage FX"));
    assert_eq!(rows[0].normal_tags, vec!["Analog Kick"]);
    assert!(!rows[0].missing);
    assert_eq!(
        sample_id_count(dir.path(), "features", "source::one.wav"),
        0
    );
    assert_eq!(
        sample_id_count(dir.path(), "features", "source::two.wav"),
        1
    );
    assert_eq!(
        analysis_job_relative_path(dir.path(), "source::two.wav"),
        "two.wav"
    );
}

#[test]
fn rename_apply_refreshes_metadata_changed_during_discovery() {
    let dir = tempdir().unwrap();
    let first_path = dir.path().join("one.wav");
    let second_path = dir.path().join("two.wav");
    std::fs::write(&first_path, b"one").unwrap();
    let db = SourceDatabase::open(dir.path()).unwrap();
    scan_once(&db).unwrap();
    std::fs::rename(&first_path, &second_path).unwrap();
    let mut edited = false;

    let stats = scan_with_progress(&db, ScanMode::Quick, None, &mut |_, _| {
        if !edited {
            db.set_tag(Path::new("one.wav"), Rating::KEEP_1).unwrap();
            db.set_user_tag(Path::new("one.wav"), Some("Edited during scan"))
                .unwrap();
            edited = true;
        }
    })
    .unwrap();

    assert!(edited);
    assert_eq!(stats.renames_reconciled, 1);
    let row = db.entry_for_path(Path::new("two.wav")).unwrap().unwrap();
    assert_eq!(row.tag, Rating::KEEP_1);
    assert_eq!(row.user_tag.as_deref(), Some("Edited during scan"));
}

#[test]
fn pending_rename_staging_refreshes_metadata_changed_during_discovery() {
    let dir = tempdir().unwrap();
    let removed_path = dir.path().join("removed.wav");
    std::fs::write(&removed_path, b"removed").unwrap();
    std::fs::write(dir.path().join("live.wav"), b"live").unwrap();
    let db = SourceDatabase::open(dir.path()).unwrap();
    scan_once(&db).unwrap();
    std::fs::remove_file(&removed_path).unwrap();
    let mut edited = false;

    scan_with_progress(&db, ScanMode::Quick, None, &mut |_, _| {
        if !edited {
            db.set_tag(Path::new("removed.wav"), Rating::KEEP_1)
                .unwrap();
            edited = true;
        }
    })
    .unwrap();

    let pending = db.list_pending_renames().unwrap();
    let removed = pending
        .iter()
        .find(|entry| entry.relative_path == Path::new("removed.wav"))
        .expect("removed path must be staged");
    assert_eq!(removed.tag, Rating::KEEP_1);
}

#[test]
fn quick_scan_defers_hash_for_large_file() {
    let dir = tempdir().unwrap();
    let file_path = dir.path().join("large.wav");
    std::fs::write(&file_path, vec![0u8; 9 * 1024 * 1024]).unwrap();

    let db = SourceDatabase::open(dir.path()).unwrap();
    let stats = scan_once(&db).unwrap();
    assert_eq!(stats.hashes_pending, 1);
    assert_eq!(stats.hashes_computed, 0);

    let rows = db.list_files().unwrap();
    assert_eq!(rows.len(), 1);
    assert!(rows[0].content_hash.is_none());
}

#[test]
fn quick_scan_reconciles_large_rename_and_preserves_tag() {
    let dir = tempdir().unwrap();
    let first_path = dir.path().join("one.wav");
    let second_path = dir.path().join("two.wav");
    std::fs::write(&first_path, vec![0u8; 9 * 1024 * 1024]).unwrap();

    let db = SourceDatabase::open(dir.path()).unwrap();
    hard_rescan(&db).unwrap();
    db.set_tag(Path::new("one.wav"), Rating::KEEP_1).unwrap();
    db.set_sound_type(Path::new("one.wav"), Some(SampleSoundType::Texture))
        .unwrap();
    db.set_user_tag(Path::new("one.wav"), Some("Night Pad"))
        .unwrap();
    db.assign_tag_to_path(Path::new("one.wav"), "Night Pad")
        .unwrap();

    std::fs::rename(&first_path, &second_path).unwrap();
    let stats = scan_once(&db).unwrap();
    assert_eq!(stats.renames_reconciled, 1);
    assert_eq!(stats.hashes_pending, 1);

    let rows = db.list_files().unwrap();
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0].relative_path, Path::new("two.wav"));
    assert_eq!(rows[0].tag, Rating::KEEP_1);
    assert_eq!(rows[0].sound_type, Some(SampleSoundType::Texture));
    assert_eq!(rows[0].user_tag.as_deref(), Some("Night Pad"));
    assert_eq!(rows[0].normal_tags, vec!["Night Pad"]);
    assert!(!rows[0].missing);
    assert!(rows[0].content_hash.is_none());

    let deep_stats = crate::sample_sources::scanner::scan_hash::deep_hash_scan(&db, None).unwrap();
    assert_eq!(deep_stats.hashes_computed, 1);
    assert_eq!(deep_stats.renames_reconciled, 0);

    let rows = db.list_files().unwrap();
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0].relative_path, Path::new("two.wav"));
    assert_eq!(rows[0].tag, Rating::KEEP_1);
    assert_eq!(rows[0].sound_type, Some(SampleSoundType::Texture));
    assert_eq!(rows[0].user_tag.as_deref(), Some("Night Pad"));
    assert_eq!(rows[0].normal_tags, vec!["Night Pad"]);
    assert!(!rows[0].missing);
    assert!(rows[0].content_hash.is_some());
}

#[test]
fn deep_hash_scan_replays_pending_rename_metadata() {
    let dir = tempdir().unwrap();
    let old_path = dir.path().join("one.wav");
    let new_path = dir.path().join("two.wav");
    std::fs::write(&old_path, vec![0u8; 9 * 1024 * 1024]).unwrap();

    let db = SourceDatabase::open(dir.path()).unwrap();
    hard_rescan(&db).unwrap();
    db.set_tag(Path::new("one.wav"), Rating::KEEP_1).unwrap();
    db.set_sound_type(Path::new("one.wav"), Some(SampleSoundType::Fx))
        .unwrap();
    db.set_user_tag(Path::new("one.wav"), Some("Sweep"))
        .unwrap();
    db.assign_tag_to_path(Path::new("one.wav"), "Sweep")
        .unwrap();

    {
        let entry = db.entry_for_path(Path::new("one.wav")).unwrap().unwrap();
        let mut batch = db.write_batch().unwrap();
        batch.stage_pending_rename(&entry).unwrap();
        batch.remove_file(Path::new("one.wav")).unwrap();
        batch.commit().unwrap();
    }
    std::fs::rename(&old_path, &new_path).unwrap();
    let mut batch = db.write_batch().unwrap();
    batch
        .upsert_file_without_hash(Path::new("two.wav"), 9 * 1024 * 1024, 0)
        .unwrap();
    batch.commit().unwrap();

    let pending_before_deep = db.list_pending_renames().unwrap();
    assert_eq!(pending_before_deep.len(), 1);
    assert_eq!(pending_before_deep[0].sound_type, Some(SampleSoundType::Fx));
    assert_eq!(pending_before_deep[0].user_tag.as_deref(), Some("Sweep"));
    assert_eq!(pending_before_deep[0].normal_tags, vec!["Sweep"]);

    let deep_stats = crate::sample_sources::scanner::scan_hash::deep_hash_scan(&db, None).unwrap();
    assert_eq!(deep_stats.renames_reconciled, 1);

    let rows = db.list_files().unwrap();
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0].relative_path, Path::new("two.wav"));
    assert_eq!(rows[0].tag, Rating::KEEP_1);
    assert_eq!(rows[0].sound_type, Some(SampleSoundType::Fx));
    assert_eq!(rows[0].user_tag.as_deref(), Some("Sweep"));
    assert_eq!(rows[0].normal_tags, vec!["Sweep"]);
    assert!(db.list_pending_renames().unwrap().is_empty());
}

#[cfg(unix)]
#[test]
fn quick_scan_avoids_ambiguous_large_rename() {
    let dir = tempdir().unwrap();
    let first_path = dir.path().join("one.wav");
    let second_path = dir.path().join("two.wav");
    let third_path = dir.path().join("three.wav");
    let payload = vec![0u8; 9 * 1024 * 1024];
    std::fs::write(&first_path, &payload).unwrap();
    std::fs::write(&second_path, &payload).unwrap();

    let timestamp = 1_700_000_000i64;
    set_file_times(&first_path, timestamp, 0);
    set_file_times(&second_path, timestamp, 0);

    let db = SourceDatabase::open(dir.path()).unwrap();
    hard_rescan(&db).unwrap();
    db.set_tag(Path::new("one.wav"), Rating::KEEP_1).unwrap();

    std::fs::remove_file(&first_path).unwrap();
    std::fs::remove_file(&second_path).unwrap();
    std::fs::write(&third_path, &payload).unwrap();
    set_file_times(&third_path, timestamp, 0);

    let stats = scan_once(&db).unwrap();
    assert_eq!(stats.renames_reconciled, 0);
    assert_eq!(stats.added, 1);
    assert_eq!(stats.missing, 2);

    let rows = db.list_files().unwrap();
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0].relative_path, Path::new("three.wav"));
    assert_eq!(rows[0].tag, Rating::NEUTRAL);
    let pending = db.list_pending_renames().unwrap();
    assert_eq!(pending.len(), 2);
    assert!(pending
        .iter()
        .any(|entry| entry.relative_path == Path::new("one.wav") && entry.tag == Rating::KEEP_1));
}

fn insert_analysis_artifacts(root: &Path, sample_id: &str, relative_path: &str) {
    let conn = SourceDatabase::open_connection(root).unwrap();
    conn.execute(
        "INSERT INTO samples (
             sample_id, content_hash, size, mtime_ns, duration_seconds, sr_used, analysis_version
         ) VALUES (?1, 'hash-a', 1, 1, 1.0, 48000, 'analysis_v1_test')",
        [sample_id],
    )
    .unwrap();
    conn.execute(
        "INSERT INTO features (sample_id, feat_version, vec_blob, light_dsp_blob, rms, computed_at)
         VALUES (?1, 1, x'00', x'00', 0.0, 1)",
        [sample_id],
    )
    .unwrap();
    conn.execute(
        "INSERT INTO embeddings (sample_id, model_id, dim, dtype, l2_normed, vec, created_at)
         VALUES (?1, 'model', 1, 'f32', 1, x'00', 1)",
        [sample_id],
    )
    .unwrap();
    conn.execute(
        "INSERT INTO analysis_jobs (
             sample_id, source_id, relative_path, job_type, content_hash, status, attempts, created_at
         ) VALUES (?1, 'source', ?2, 'analyze_sample', 'hash-a', 'done', 0, 1)",
        [sample_id, relative_path],
    )
    .unwrap();
}

fn sample_id_count(root: &Path, table: &str, sample_id: &str) -> i64 {
    let conn = SourceDatabase::open_connection(root).unwrap();
    conn.query_row(
        &format!("SELECT COUNT(*) FROM {table} WHERE sample_id = ?1"),
        [sample_id],
        |row| row.get(0),
    )
    .unwrap()
}

fn analysis_job_relative_path(root: &Path, sample_id: &str) -> String {
    let conn = SourceDatabase::open_connection(root).unwrap();
    conn.query_row(
        "SELECT relative_path FROM analysis_jobs WHERE sample_id = ?1",
        [sample_id],
        |row| row.get(0),
    )
    .unwrap()
}
