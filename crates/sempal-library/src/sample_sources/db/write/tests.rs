use std::path::Path;

use tempfile::tempdir;

use super::*;

type RowSnapshot = Vec<(
    std::path::PathBuf,
    Rating,
    bool,
    bool,
    bool,
    Option<i64>,
    Option<SampleSoundType>,
)>;

fn revision_value(db: &SourceDatabase) -> i64 {
    db.connection
        .query_row(
            "SELECT value FROM metadata WHERE key = 'revision'",
            [],
            |row| row.get::<_, String>(0),
        )
        .unwrap()
        .parse::<i64>()
        .unwrap()
}

fn wav_paths_revision_value(db: &SourceDatabase) -> u64 {
    db.get_wav_paths_revision().unwrap()
}

fn row_snapshot(db: &SourceDatabase) -> RowSnapshot {
    db.list_files()
        .unwrap()
        .into_iter()
        .map(|row| {
            let relative_path = row.relative_path;
            (
                relative_path.clone(),
                row.tag,
                row.looped,
                row.locked,
                row.missing,
                row.last_played_at,
                db.sound_type_for_path(&relative_path).unwrap(),
            )
        })
        .collect()
}

#[test]
fn metadata_only_mutations_do_not_bump_wav_paths_revision() {
    let dir = tempdir().unwrap();
    let db = SourceDatabase::open(dir.path()).unwrap();

    db.upsert_file(Path::new("one.wav"), 10, 5).unwrap();
    assert_eq!(wav_paths_revision_value(&db), 1);

    db.set_tag(Path::new("one.wav"), Rating::KEEP_1).unwrap();
    db.set_locked(Path::new("one.wav"), true).unwrap();
    db.set_missing(Path::new("one.wav"), true).unwrap();
    db.set_last_played_at(Path::new("one.wav"), 42).unwrap();
    db.set_sound_type(Path::new("one.wav"), Some(SampleSoundType::Kick))
        .unwrap();

    assert_eq!(wav_paths_revision_value(&db), 1);

    db.remove_file(Path::new("one.wav")).unwrap();
    assert_eq!(wav_paths_revision_value(&db), 2);
}

#[test]
fn upsert_wrapper_matches_batch_insert_contract() {
    let single_dir = tempdir().unwrap();
    let single = SourceDatabase::open(single_dir.path()).unwrap();
    single.upsert_file(Path::new("one.wav"), 10, 5).unwrap();

    let batch_dir = tempdir().unwrap();
    let batch_db = SourceDatabase::open(batch_dir.path()).unwrap();
    let mut batch = batch_db.write_batch().unwrap();
    batch.upsert_file(Path::new("one.wav"), 10, 5).unwrap();
    batch.commit().unwrap();

    let single_rows = single.list_files().unwrap();
    let batch_rows = batch_db.list_files().unwrap();
    assert_eq!(single_rows.len(), 1);
    assert_eq!(batch_rows.len(), 1);
    assert_eq!(single_rows[0].relative_path, batch_rows[0].relative_path);
    assert_eq!(single_rows[0].file_size, batch_rows[0].file_size);
    assert_eq!(single_rows[0].modified_ns, batch_rows[0].modified_ns);
    assert_eq!(single_rows[0].content_hash, batch_rows[0].content_hash);
    assert_eq!(single_rows[0].tag, batch_rows[0].tag);
    assert_eq!(single_rows[0].looped, batch_rows[0].looped);
    assert_eq!(single_rows[0].locked, batch_rows[0].locked);
    assert_eq!(single_rows[0].missing, batch_rows[0].missing);
    assert_eq!(single_rows[0].last_played_at, batch_rows[0].last_played_at);
    assert_eq!(revision_value(&single), 1);
    assert_eq!(revision_value(&batch_db), 1);
}

#[test]
fn single_write_wrappers_match_batch_results_and_revision_behavior() {
    let single_dir = tempdir().unwrap();
    let single = SourceDatabase::open(single_dir.path()).unwrap();
    single.upsert_file(Path::new("one.wav"), 10, 5).unwrap();
    assert_eq!(revision_value(&single), 1);
    single
        .set_tag(Path::new("one.wav"), Rating::KEEP_1)
        .unwrap();
    assert_eq!(revision_value(&single), 2);
    single.set_looped(Path::new("one.wav"), true).unwrap();
    assert_eq!(revision_value(&single), 3);
    single.set_locked(Path::new("one.wav"), true).unwrap();
    assert_eq!(revision_value(&single), 4);
    single.set_missing(Path::new("one.wav"), true).unwrap();
    assert_eq!(revision_value(&single), 5);
    single.set_last_played_at(Path::new("one.wav"), 42).unwrap();
    assert_eq!(revision_value(&single), 6);
    single
        .set_sound_type(Path::new("one.wav"), Some(SampleSoundType::Kick))
        .unwrap();
    assert_eq!(revision_value(&single), 7);

    let batch_dir = tempdir().unwrap();
    let batch_db = SourceDatabase::open(batch_dir.path()).unwrap();
    batch_db.upsert_file(Path::new("one.wav"), 10, 5).unwrap();
    assert_eq!(revision_value(&batch_db), 1);
    let mut batch = batch_db.write_batch().unwrap();
    batch.set_tag(Path::new("one.wav"), Rating::KEEP_1).unwrap();
    batch.set_looped(Path::new("one.wav"), true).unwrap();
    batch.set_locked(Path::new("one.wav"), true).unwrap();
    batch.set_missing(Path::new("one.wav"), true).unwrap();
    batch.set_last_played_at(Path::new("one.wav"), 42).unwrap();
    batch
        .set_sound_type(Path::new("one.wav"), Some(SampleSoundType::Kick))
        .unwrap();
    batch.commit().unwrap();
    assert_eq!(revision_value(&batch_db), 2);

    let single_rows = single.list_files().unwrap();
    let batch_rows = batch_db.list_files().unwrap();
    assert_eq!(single_rows.len(), 1);
    assert_eq!(single_rows[0].relative_path, batch_rows[0].relative_path);
    assert_eq!(single_rows[0].tag, batch_rows[0].tag);
    assert_eq!(single_rows[0].looped, batch_rows[0].looped);
    assert_eq!(single_rows[0].locked, batch_rows[0].locked);
    assert_eq!(single_rows[0].missing, batch_rows[0].missing);
    assert_eq!(single_rows[0].last_played_at, batch_rows[0].last_played_at);
    assert_eq!(
        single.sound_type_for_path(Path::new("one.wav")).unwrap(),
        batch_db.sound_type_for_path(Path::new("one.wav")).unwrap()
    );

    single.remove_file(Path::new("one.wav")).unwrap();
    assert_eq!(revision_value(&single), 8);
    let mut batch = batch_db.write_batch().unwrap();
    batch.remove_file(Path::new("one.wav")).unwrap();
    batch.commit().unwrap();
    assert_eq!(revision_value(&batch_db), 3);

    assert!(single.list_files().unwrap().is_empty());
    assert!(batch_db.list_files().unwrap().is_empty());
}

#[test]
fn failed_write_wrappers_leave_rows_and_revision_unchanged() {
    let dir = tempdir().unwrap();
    let db = SourceDatabase::open(dir.path()).unwrap();
    db.upsert_file(Path::new("one.wav"), 10, 5).unwrap();
    let before_rows = row_snapshot(&db);
    let before_revision = revision_value(&db);

    let err = db.set_looped(Path::new("../escape.wav"), true).unwrap_err();
    assert!(matches!(err, SourceDbError::InvalidRelativePath(_)));

    let after_revision = revision_value(&db);
    assert_eq!(row_snapshot(&db), before_rows);
    assert_eq!(before_revision, after_revision);
}

#[test]
fn wav_upsert_variants_preserve_hash_tag_and_missing_contracts() {
    let dir = tempdir().unwrap();
    let db = SourceDatabase::open(dir.path()).unwrap();

    let mut batch = db.write_batch().unwrap();
    batch
        .upsert_file_with_hash_and_tag(Path::new("one.wav"), 10, 5, "hash-a", Rating::KEEP_1, true)
        .unwrap();
    batch.commit().unwrap();

    let first = db.list_files().unwrap();
    assert_eq!(first.len(), 1);
    assert_eq!(first[0].content_hash.as_deref(), Some("hash-a"));
    assert_eq!(first[0].tag, Rating::KEEP_1);
    assert!(first[0].missing);

    let mut batch = db.write_batch().unwrap();
    batch
        .upsert_file_without_hash(Path::new("one.wav"), 12, 6)
        .unwrap();
    batch.commit().unwrap();

    let second = db.list_files().unwrap();
    assert_eq!(second[0].content_hash, None);
    assert_eq!(second[0].tag, Rating::KEEP_1);
    assert!(!second[0].missing);
}

#[test]
fn empty_tag_batch_is_a_noop() {
    let dir = tempdir().unwrap();
    let db = SourceDatabase::open(dir.path()).unwrap();

    db.set_tags_batch(&[]).unwrap();

    assert_eq!(db.get_revision().unwrap(), 0);
    assert!(db.list_files().unwrap().is_empty());
}

#[test]
fn batch_errors_roll_back_prior_mutations() {
    let dir = tempdir().unwrap();
    let db = SourceDatabase::open(dir.path()).unwrap();
    db.upsert_file(Path::new("one.wav"), 10, 5).unwrap();
    let before = row_snapshot(&db);
    let before_revision = revision_value(&db);

    let mut batch = db.write_batch().unwrap();
    batch.set_tag(Path::new("one.wav"), Rating::KEEP_1).unwrap();
    let err = batch
        .set_missing(Path::new("../escape.wav"), true)
        .unwrap_err();
    assert!(matches!(err, SourceDbError::InvalidRelativePath(_)));
    drop(batch);

    assert_eq!(row_snapshot(&db), before);
    assert_eq!(revision_value(&db), before_revision);
}

#[test]
fn metadata_written_inside_batch_commits_with_other_changes() {
    let dir = tempdir().unwrap();
    let db = SourceDatabase::open(dir.path()).unwrap();

    let mut batch = db.write_batch().unwrap();
    batch.upsert_file(Path::new("one.wav"), 10, 5).unwrap();
    batch.set_metadata("custom_key", "custom_value").unwrap();
    batch.commit().unwrap();

    let metadata = db.get_metadata("custom_key").unwrap();
    assert_eq!(metadata.as_deref(), Some("custom_value"));
    assert_eq!(revision_value(&db), 1);
}

#[test]
fn metadata_wrapper_uses_batch_revision_policy_without_wav_path_churn() {
    let dir = tempdir().unwrap();
    let db = SourceDatabase::open(dir.path()).unwrap();

    db.set_metadata("custom_key", "custom_value").unwrap();

    assert_eq!(
        db.get_metadata("custom_key").unwrap().as_deref(),
        Some("custom_value")
    );
    assert_eq!(revision_value(&db), 1);
    assert_eq!(wav_paths_revision_value(&db), 0);
}

#[test]
fn rename_identity_remap_preserves_analysis_artifacts_and_jobs() {
    let dir = tempdir().unwrap();
    let db = SourceDatabase::open(dir.path()).unwrap();
    let old = Path::new("old.wav");
    let new = Path::new("renamed.wav");
    let old_sample_id = "source::old.wav";
    let new_sample_id = "source::renamed.wav";

    db.upsert_file(old, 10, 5).unwrap();
    db.connection
        .execute(
            "INSERT INTO samples (
                 sample_id, content_hash, size, mtime_ns, duration_seconds, sr_used,
                 analysis_version, bpm, long_sample_mark
             ) VALUES (?1, 'hash-a', 10, 5, 1.25, 48000, 'analysis_v1_test', 123.0, 1)",
            [old_sample_id],
        )
        .unwrap();
    db.connection
        .execute(
            "INSERT INTO analysis_features (sample_id, content_hash, features)
             VALUES (?1, 'hash-a', x'01')",
            [old_sample_id],
        )
        .unwrap();
    db.connection
        .execute(
            "INSERT INTO features (sample_id, feat_version, vec_blob, light_dsp_blob, rms, computed_at)
             VALUES (?1, 1, x'02', x'03', 0.5, 7)",
            [old_sample_id],
        )
        .unwrap();
    db.connection
        .execute(
            "INSERT INTO embeddings (sample_id, model_id, dim, dtype, l2_normed, vec, created_at)
             VALUES (?1, 'model', 1, 'f32', 1, x'04', 8)",
            [old_sample_id],
        )
        .unwrap();
    db.connection
        .execute(
            "INSERT INTO layout_umap (sample_id, model_id, umap_version, x, y, created_at)
             VALUES (?1, 'model', 'umap', 1.0, 2.0, 9)",
            [old_sample_id],
        )
        .unwrap();
    db.connection
        .execute(
            "INSERT INTO hdbscan_clusters (sample_id, model_id, method, umap_version, cluster_id, created_at)
             VALUES (?1, 'model', 'hdbscan', 'umap', 3, 10)",
            [old_sample_id],
        )
        .unwrap();
    db.connection
        .execute(
            "INSERT INTO analysis_jobs (
                 sample_id, source_id, relative_path, job_type, content_hash, status, attempts, created_at
             ) VALUES (?1, 'source', 'old.wav', 'analyze_sample', 'hash-a', 'done', 1, 11)",
            [old_sample_id],
        )
        .unwrap();

    let mut batch = db.write_batch().unwrap();
    batch.remove_file(old).unwrap();
    batch.upsert_file(new, 10, 5).unwrap();
    batch.remap_analysis_sample_identity(old, new).unwrap();
    batch.commit().unwrap();

    assert_eq!(sample_id_count(&db, "samples", old_sample_id), 0);
    for table in [
        "samples",
        "analysis_features",
        "features",
        "embeddings",
        "layout_umap",
        "hdbscan_clusters",
        "analysis_jobs",
    ] {
        assert_eq!(sample_id_count(&db, table, new_sample_id), 1, "{table}");
    }
    let job_relative: String = db
        .connection
        .query_row(
            "SELECT relative_path FROM analysis_jobs WHERE sample_id = ?1",
            [new_sample_id],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(job_relative, "renamed.wav");
    let analysis_version: String = db
        .connection
        .query_row(
            "SELECT analysis_version FROM samples WHERE sample_id = ?1",
            [new_sample_id],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(analysis_version, "analysis_v1_test");
}

fn sample_id_count(db: &SourceDatabase, table: &str, sample_id: &str) -> i64 {
    db.connection
        .query_row(
            &format!("SELECT COUNT(*) FROM {table} WHERE sample_id = ?1"),
            [sample_id],
            |row| row.get(0),
        )
        .unwrap()
}
