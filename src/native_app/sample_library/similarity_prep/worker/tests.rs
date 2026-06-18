use super::*;
use wavecrate::sample_sources::{SourceId, db::META_LAST_SCAN_COMPLETED_AT};

#[test]
fn native_similarity_status_resolves_core_states() {
    assert_eq!(
        resolve_similarity_prep_facts(SimilarityPrepFacts {
            scan_completed_at: Some(20),
            prep_completed_at: Some(20),
            has_embeddings: true,
            has_layout: true,
            failures: Some(SimilarityPrepFailureCounts {
                failed_count: 0,
                unsupported_count: 0,
            }),
        }),
        NativeSimilarityPrepStatus::UpToDate
    );
    assert_eq!(
        resolve_similarity_prep_facts(SimilarityPrepFacts {
            scan_completed_at: Some(20),
            prep_completed_at: Some(10),
            has_embeddings: true,
            has_layout: true,
            failures: None,
        }),
        NativeSimilarityPrepStatus::Outdated
    );
    assert_eq!(
        resolve_similarity_prep_facts(SimilarityPrepFacts {
            scan_completed_at: Some(20),
            prep_completed_at: Some(10),
            has_embeddings: false,
            has_layout: false,
            failures: Some(SimilarityPrepFailureCounts {
                failed_count: 2,
                unsupported_count: 1,
            }),
        }),
        NativeSimilarityPrepStatus::Blocked {
            failed_count: 2,
            unsupported_count: 1,
        }
    );
}

#[test]
fn native_similarity_status_read_does_not_enqueue_jobs() {
    let (_dir, source) = source_with_file("read-only.wav");

    let status = resolve_similarity_prep_status(&source).expect("status");

    assert_eq!(count_jobs(&source), 0);
    assert_eq!(status, NativeSimilarityPrepStatus::Outdated);
}

#[test]
fn native_similarity_prepare_enqueues_analysis_and_embedding_jobs() {
    let (_dir, source) = source_with_file("queued.wav");

    let summary = enqueue_similarity_prep_inner(&source, false).expect("enqueue");

    assert_eq!(summary.analysis_inserted, 1);
    assert_eq!(summary.embedding_inserted, 1);
    assert_eq!(count_jobs_by_type(&source, ANALYZE_SAMPLE_JOB_TYPE), 1);
    assert_eq!(count_jobs_by_type(&source, EMBEDDING_BACKFILL_JOB_TYPE), 1);
}

#[test]
fn native_similarity_prepare_skips_current_analysis_artifacts() {
    let (_dir, source) = source_with_file("current.wav");
    seed_current_analysis_artifacts(&source, "current.wav");

    assert_eq!(
        enqueue_analysis_backfill(&source).expect("analysis enqueue"),
        0
    );
    assert_eq!(
        enqueue_embedding_backfill(&source).expect("embedding enqueue"),
        0
    );
    assert_eq!(count_jobs(&source), 0);
}

#[test]
fn automatic_native_similarity_prepare_does_not_retry_blocked_failures() {
    let (_dir, source) = source_with_file("failed.wav");
    seed_failed_analysis_job(&source, "failed.wav");

    let summary = enqueue_similarity_prep_inner(&source, true).expect("automatic enqueue");

    assert_eq!(summary.analysis_inserted, 0);
    assert_eq!(summary.embedding_inserted, 0);
    assert!(matches!(
        summary.status,
        NativeSimilarityPrepStatus::Blocked {
            failed_count: 1,
            unsupported_count: 0,
        }
    ));
    assert_eq!(count_jobs_by_status(&source, "pending"), 0);
}

fn source_with_file(name: &str) -> (tempfile::TempDir, SampleSource) {
    let dir = tempfile::tempdir().expect("temp source");
    std::fs::write(dir.path().join(name), [0_u8; 8]).expect("sample file");
    let source = SampleSource::new_with_id(SourceId::from_string("native"), dir.path().into());
    let db = SourceDatabase::open(&source.root).expect("source db");
    db.upsert_file(std::path::Path::new(name), 8, 10)
        .expect("file row");
    db.set_metadata(META_LAST_SCAN_COMPLETED_AT, "20")
        .expect("scan timestamp");
    (dir, source)
}

fn count_jobs(source: &SampleSource) -> i64 {
    let conn = open_source_db(&source.root).expect("analysis db");
    conn.query_row("SELECT COUNT(*) FROM analysis_jobs", [], |row| row.get(0))
        .expect("job count")
}

fn count_jobs_by_type(source: &SampleSource, job_type: &str) -> i64 {
    let conn = open_source_db(&source.root).expect("analysis db");
    conn.query_row(
        "SELECT COUNT(*) FROM analysis_jobs WHERE job_type = ?1",
        [job_type],
        |row| row.get(0),
    )
    .expect("job count")
}

fn count_jobs_by_status(source: &SampleSource, status: &str) -> i64 {
    let conn = open_source_db(&source.root).expect("analysis db");
    conn.query_row(
        "SELECT COUNT(*) FROM analysis_jobs WHERE status = ?1",
        [status],
        |row| row.get(0),
    )
    .expect("job count")
}

fn seed_current_analysis_artifacts(source: &SampleSource, relative_path: &str) {
    let conn = open_source_db(&source.root).expect("analysis db");
    let sample_id = build_sample_id(source.id.as_str(), std::path::Path::new(relative_path));
    conn.execute(
        "INSERT OR REPLACE INTO samples
         (sample_id, content_hash, size, mtime_ns, duration_seconds, sr_used, analysis_version)
         VALUES (?1, ?2, 8, 10, 1.0, 44100, ?3)",
        rusqlite::params![
            &sample_id,
            "fast-8-10",
            wavecrate_analysis::analysis_version()
        ],
    )
    .expect("insert sample");
    conn.execute(
        "INSERT OR REPLACE INTO features
         (sample_id, feat_version, vec_blob, light_dsp_blob, rms, computed_at)
         VALUES (?1, 1, ?2, NULL, 0.5, 0)",
        rusqlite::params![&sample_id, vec![0_u8; 8]],
    )
    .expect("insert features");
    conn.execute(
        "INSERT OR REPLACE INTO embeddings
         (sample_id, model_id, dim, dtype, l2_normed, vec, created_at)
         VALUES (?1, ?2, 2, 'f32', 1, ?3, 0)",
        rusqlite::params![
            &sample_id,
            SIMILARITY_MODEL_ID,
            wavecrate_analysis::vector::encode_f32_le_blob(&[1.0, 0.0])
        ],
    )
    .expect("insert embedding");
}

fn seed_failed_analysis_job(source: &SampleSource, relative_path: &str) {
    let conn = open_source_db(&source.root).expect("analysis db");
    let sample_id = build_sample_id(source.id.as_str(), std::path::Path::new(relative_path));
    conn.execute(
        "INSERT INTO analysis_jobs
         (sample_id, source_id, relative_path, job_type, content_hash, status, attempts, created_at, last_error)
         VALUES (?1, ?2, ?3, ?4, 'fast-8-10', 'failed', 1, 0, 'decode failed')",
        rusqlite::params![
            sample_id,
            source.id.as_str(),
            relative_path,
            ANALYZE_SAMPLE_JOB_TYPE,
        ],
    )
    .expect("insert failed job");
}
