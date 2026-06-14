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

    let summary = enqueue_similarity_prep_inner(&source).expect("enqueue");

    assert_eq!(summary.analysis_inserted, 1);
    assert_eq!(summary.embedding_inserted, 1);
    assert_eq!(count_jobs_by_type(&source, ANALYZE_SAMPLE_JOB_TYPE), 1);
    assert_eq!(count_jobs_by_type(&source, EMBEDDING_BACKFILL_JOB_TYPE), 1);
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
