use std::path::Path;

use tempfile::tempdir;

use super::super::super::{Rating, SampleCollection, SourceDatabase};

#[test]
fn rename_identity_remap_preserves_analysis_artifacts_and_jobs() {
    let dir = tempdir().unwrap();
    let db = SourceDatabase::open(dir.path()).unwrap();
    let old = Path::new("old.wav");
    let new = Path::new("renamed.wav");
    let old_sample_id = "source::old.wav";
    let new_sample_id = "source::renamed.wav";

    db.upsert_file(old, 10, 5).unwrap();
    insert_analysis_artifacts(&db, old_sample_id);

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
        "similarity_aspect_descriptors",
        "layout_umap",
        "hdbscan_clusters",
        "analysis_jobs",
    ] {
        assert_eq!(sample_id_count(&db, table, new_sample_id), 1, "{table}");
    }
    assert_eq!(job_relative_path(&db, new_sample_id), "renamed.wav");
    assert_eq!(analysis_version(&db, new_sample_id), "analysis_v1_test");
}

#[test]
fn rename_identity_remap_replaces_stale_destination_analysis_artifacts() {
    let dir = tempdir().unwrap();
    let db = SourceDatabase::open(dir.path()).unwrap();
    let old = Path::new("old.wav");
    let new = Path::new("renamed.wav");
    let old_sample_id = "source::old.wav";
    let new_sample_id = "source::renamed.wav";

    insert_analysis_artifacts_with_version(&db, old_sample_id, "analysis_from_moved_file");
    insert_analysis_artifacts_with_version(&db, new_sample_id, "stale_destination_analysis");

    let mut batch = db.write_batch().unwrap();
    batch.remap_analysis_sample_identity(old, new).unwrap();
    batch.commit().unwrap();

    for table in [
        "samples",
        "analysis_features",
        "features",
        "embeddings",
        "similarity_aspect_descriptors",
        "layout_umap",
        "hdbscan_clusters",
        "analysis_jobs",
    ] {
        assert_eq!(sample_id_count(&db, table, old_sample_id), 0, "{table}");
        assert_eq!(sample_id_count(&db, table, new_sample_id), 1, "{table}");
    }
    assert_eq!(job_relative_path(&db, new_sample_id), "renamed.wav");
    assert_eq!(
        analysis_version(&db, new_sample_id),
        "analysis_from_moved_file"
    );
}

#[test]
fn wav_path_remap_preserves_user_metadata_and_collection_memberships() {
    let dir = tempdir().unwrap();
    let db = SourceDatabase::open(dir.path()).unwrap();
    let old = Path::new("old.wav");
    let new = Path::new("renamed.wav");
    let first = SampleCollection::new(0).unwrap();
    let second = SampleCollection::new(1).unwrap();

    db.upsert_file(old, 10, 5).unwrap();
    let mut batch = db.write_batch().unwrap();
    batch.set_tag(old, Rating::KEEP_1).unwrap();
    batch.set_locked(old, true).unwrap();
    batch.add_collection(old, second).unwrap();
    batch.add_collection(old, first).unwrap();
    batch.assign_tag_to_path(old, "break").unwrap();
    batch.remap_wav_file_path(old, new).unwrap();
    batch.commit().unwrap();

    assert!(db.entry_for_path(old).unwrap().is_none());
    let renamed = db.entry_for_path(new).unwrap().expect("renamed row");
    assert_eq!(renamed.tag, Rating::KEEP_1);
    assert!(renamed.locked);
    assert_eq!(db.collections_for_path(old).unwrap(), Vec::new());
    assert_eq!(db.collections_for_path(new).unwrap(), vec![first, second]);
    assert_eq!(
        db.tag_labels_for_path(new).unwrap(),
        vec![String::from("break")]
    );
}

fn insert_analysis_artifacts(db: &SourceDatabase, sample_id: &str) {
    insert_analysis_artifacts_with_version(db, sample_id, "analysis_v1_test");
}

fn insert_analysis_artifacts_with_version(
    db: &SourceDatabase,
    sample_id: &str,
    analysis_version: &str,
) {
    db.connection
        .execute(
            "INSERT INTO samples (
                 sample_id, content_hash, size, mtime_ns, duration_seconds, sr_used,
                 analysis_version, bpm, long_sample_mark
             ) VALUES (?1, 'hash-a', 10, 5, 1.25, 48000, ?2, 123.0, 1)",
            rusqlite::params![sample_id, analysis_version],
        )
        .unwrap();
    db.connection
        .execute(
            "INSERT INTO analysis_features (sample_id, content_hash, features)
             VALUES (?1, 'hash-a', x'01')",
            [sample_id],
        )
        .unwrap();
    db.connection
        .execute(
            "INSERT INTO features (sample_id, feat_version, vec_blob, light_dsp_blob, rms, computed_at)
             VALUES (?1, 1, x'02', x'03', 0.5, 7)",
            [sample_id],
        )
        .unwrap();
    db.connection
        .execute(
            "INSERT INTO embeddings (sample_id, model_id, dim, dtype, l2_normed, vec, created_at)
             VALUES (?1, 'model', 1, 'f32', 1, x'04', 8)",
            [sample_id],
        )
        .unwrap();
    db.connection
        .execute(
            "INSERT INTO similarity_aspect_descriptors (
                 sample_id, model_id, dim, dtype, l2_normed, valid_mask, vec, created_at
             ) VALUES (?1, 'aspect-model', 1, 'f32', 1, 1, x'07', 8)",
            [sample_id],
        )
        .unwrap();
    db.connection
        .execute(
            "INSERT INTO layout_umap (sample_id, model_id, umap_version, x, y, created_at)
             VALUES (?1, 'model', 'umap', 1.0, 2.0, 9)",
            [sample_id],
        )
        .unwrap();
    db.connection
        .execute(
            "INSERT INTO hdbscan_clusters (sample_id, model_id, method, umap_version, cluster_id, created_at)
             VALUES (?1, 'model', 'hdbscan', 'umap', 3, 10)",
            [sample_id],
        )
        .unwrap();
    db.connection
        .execute(
            "INSERT INTO analysis_jobs (
                 sample_id, source_id, relative_path, job_type, content_hash, status, attempts, created_at
             ) VALUES (?1, 'source', 'old.wav', 'analyze_sample', 'hash-a', 'done', 1, 11)",
            [sample_id],
        )
        .unwrap();
}

fn job_relative_path(db: &SourceDatabase, sample_id: &str) -> String {
    db.connection
        .query_row(
            "SELECT relative_path FROM analysis_jobs WHERE sample_id = ?1",
            [sample_id],
            |row| row.get(0),
        )
        .unwrap()
}

fn analysis_version(db: &SourceDatabase, sample_id: &str) -> String {
    db.connection
        .query_row(
            "SELECT analysis_version FROM samples WHERE sample_id = ?1",
            [sample_id],
            |row| row.get(0),
        )
        .unwrap()
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
