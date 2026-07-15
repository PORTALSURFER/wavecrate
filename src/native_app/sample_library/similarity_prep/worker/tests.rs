use super::*;
use wavecrate::sample_sources::{SourceId, db::META_LAST_SCAN_COMPLETED_AT};

#[test]
fn native_similarity_status_resolves_core_states() {
    assert_eq!(
        resolve_similarity_prep_facts(SimilarityPrepFacts {
            scan_completed_at: Some(20),
            prep_completed_at: Some(20),
            has_embeddings: true,
            has_aspects: true,
            has_layout: true,
            has_active_jobs: false,
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
            has_aspects: true,
            has_layout: true,
            has_active_jobs: false,
            failures: None,
        }),
        NativeSimilarityPrepStatus::Outdated
    );
    assert_eq!(
        resolve_similarity_prep_facts(SimilarityPrepFacts {
            scan_completed_at: Some(20),
            prep_completed_at: Some(10),
            has_embeddings: false,
            has_aspects: false,
            has_layout: false,
            has_active_jobs: false,
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
fn native_similarity_status_is_outdated_while_jobs_are_pending() {
    assert_eq!(
        resolve_similarity_prep_facts(SimilarityPrepFacts {
            scan_completed_at: Some(20),
            prep_completed_at: Some(20),
            has_embeddings: true,
            has_aspects: true,
            has_layout: true,
            has_active_jobs: true,
            failures: Some(SimilarityPrepFailureCounts {
                failed_count: 0,
                unsupported_count: 0,
            }),
        }),
        NativeSimilarityPrepStatus::Outdated
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

#[test]
fn native_similarity_prepare_skips_unchanged_unsupported_files() {
    let (_dir, source) = source_with_file("unsupported.wav");
    seed_failed_analysis_job_with_error(
        &source,
        "unsupported.wav",
        "unsupported feature: no suitable format reader found",
    );

    let summary = enqueue_similarity_prep_inner(&source, false).expect("enqueue");

    assert_eq!(summary.analysis_inserted, 0);
    assert_eq!(summary.embedding_inserted, 0);
    assert_eq!(summary.status, NativeSimilarityPrepStatus::Outdated);
    assert!(similarity_prep_needs_finalization(&source).expect("finalization needed"));
    let cancel = AtomicBool::new(false);
    assert!(
        finalize_similarity_prep_if_ready(&source, &cancel).expect("finalize unsupported source")
    );
    assert_eq!(
        resolve_similarity_prep_status(&source).expect("resolved status"),
        NativeSimilarityPrepStatus::UpToDate
    );
    assert_eq!(count_jobs_by_status(&source, "failed"), 1);
}

#[test]
fn native_similarity_prepare_retries_unsupported_files_after_content_changes() {
    let (dir, source) = source_with_file("unsupported.wav");
    seed_failed_analysis_job_with_error(
        &source,
        "unsupported.wav",
        "unsupported feature: no suitable format reader found",
    );
    std::fs::write(dir.path().join("unsupported.wav"), [1_u8; 9]).expect("rewrite sample");
    let db = SourceDatabase::open(&source.root).expect("source db");
    db.upsert_file(std::path::Path::new("unsupported.wav"), 9, 11)
        .expect("update file row");

    let summary = enqueue_similarity_prep_inner(&source, false).expect("enqueue");

    assert_eq!(summary.analysis_inserted, 1);
    assert_eq!(
        count_jobs_by_type_and_status(&source, ANALYZE_SAMPLE_JOB_TYPE, "pending"),
        1
    );
}

#[test]
fn automatic_native_similarity_finish_leaves_pending_jobs_for_background_processing() {
    let config_base = tempfile::tempdir().expect("config base");
    let _config_guard = wavecrate::app_dirs::ConfigBaseGuard::set(config_base.path().to_path_buf());
    let (dir, source) = source_with_valid_wav("covered-a.wav");
    write_valid_wav_with_frequency(&dir.path().join("covered-b.wav"), 660.0);
    write_valid_wav_with_frequency(&dir.path().join("covered-c.wav"), 880.0);
    write_valid_wav_with_frequency(&dir.path().join("covered-d.wav"), 990.0);
    ensure_source_database_scanned(&source).expect("scan source");
    seed_similarity_artifacts_without_features(&source, "covered-a.wav");
    seed_similarity_artifacts_without_features(&source, "covered-b.wav");
    seed_similarity_artifacts_without_features(&source, "covered-c.wav");
    seed_similarity_artifacts_without_features(&source, "covered-d.wav");

    let finished = enqueue_similarity_prep_inner(&source, true).expect("automatic enqueue");

    assert_eq!(finished.jobs_processed, 0);
    assert!(
        count_jobs_by_status(&source, "pending") >= 4,
        "automatic prep should leave queued work pending"
    );
    assert_eq!(
        count_jobs_by_type_and_status(&source, ANALYZE_SAMPLE_JOB_TYPE, "pending"),
        4
    );
    assert_ne!(finished.status, NativeSimilarityPrepStatus::UpToDate);
}

#[test]
fn automatic_native_similarity_finish_does_not_drain_pending_jobs_when_other_jobs_failed() {
    let config_base = tempfile::tempdir().expect("config base");
    let _config_guard = wavecrate::app_dirs::ConfigBaseGuard::set(config_base.path().to_path_buf());
    let (dir, source) = source_with_valid_wav("valid-a.wav");
    write_valid_wav_with_frequency(&dir.path().join("valid-b.wav"), 330.0);
    write_valid_wav_with_frequency(&dir.path().join("valid-c.wav"), 550.0);
    write_valid_wav_with_frequency(&dir.path().join("valid-d.wav"), 770.0);
    enqueue_similarity_prep_inner(&source, false).expect("initial enqueue");
    seed_failed_analysis_job(&source, "failed.wav");

    let finished = enqueue_similarity_prep_inner(&source, true).expect("automatic enqueue");
    let pending_before_finish = count_jobs_by_status(&source, "pending");
    let pending_analysis_before_finish =
        count_jobs_by_type_and_status(&source, ANALYZE_SAMPLE_JOB_TYPE, "pending");

    assert_eq!(finished.jobs_processed, 0);
    assert_eq!(
        count_jobs_by_status(&source, "pending"),
        pending_before_finish
    );
    assert_eq!(count_jobs_by_status(&source, "failed"), 1);
    assert_eq!(
        count_jobs_by_type_and_status(&source, ANALYZE_SAMPLE_JOB_TYPE, "pending"),
        pending_analysis_before_finish
    );
    assert_eq!(count_similarity_embeddings(&source, "valid-a.wav"), 0);
    assert_eq!(count_similarity_embeddings(&source, "valid-b.wav"), 0);
    assert_eq!(count_similarity_embeddings(&source, "valid-c.wav"), 0);
    assert_eq!(count_similarity_embeddings(&source, "valid-d.wav"), 0);
    assert_eq!(count_similarity_aspects(&source, "valid-a.wav"), 0);
    assert_eq!(count_similarity_aspects(&source, "valid-b.wav"), 0);
    assert_eq!(count_similarity_aspects(&source, "valid-c.wav"), 0);
    assert_eq!(count_similarity_aspects(&source, "valid-d.wav"), 0);
    assert_ne!(finished.status, NativeSimilarityPrepStatus::UpToDate);
}

#[test]
fn native_similarity_prep_recognizes_transient_database_busy_errors() {
    assert!(is_transient_database_busy("Database is busy, please retry"));
    assert!(is_transient_database_busy(
        "Open source DB failed: database is locked"
    ));
    assert!(is_transient_database_busy(
        "SQLITE_BUSY during schema check"
    ));
    assert!(!is_transient_database_busy("Unsupported WAV encoding"));
}

#[test]
fn native_similarity_supervisor_batches_process_valid_wav_jobs() {
    let config_base = tempfile::tempdir().expect("config base");
    let _config_guard = wavecrate::app_dirs::ConfigBaseGuard::set(config_base.path().to_path_buf());
    let (_dir, source) = source_with_valid_wav("valid.wav");

    let summary = enqueue_similarity_prep_inner(&source, false).expect("enqueue");
    let cancel = std::sync::atomic::AtomicBool::new(false);
    let mut drain = SimilarityPrepJobDrainSummary::default();
    while similarity_prep_has_pending_jobs(&source).expect("pending jobs") {
        let batch = run_similarity_prep_job_batch(&source, 1, &cancel).expect("run batch");
        drain.processed += batch.processed;
        drain.failed += batch.failed;
    }

    assert_eq!(summary.analysis_inserted, 1);
    assert_eq!(summary.embedding_inserted, 1);
    assert_eq!(drain.failed, 0);
    assert!(drain.processed >= 1);
    assert!(count_similarity_embeddings(&source, "valid.wav") >= 1);
    assert!(count_similarity_aspects(&source, "valid.wav") >= 1);
    assert_eq!(
        count_jobs_by_type_and_status(&source, ANALYZE_SAMPLE_JOB_TYPE, "done"),
        1
    );
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

fn source_with_valid_wav(name: &str) -> (tempfile::TempDir, SampleSource) {
    let dir = tempfile::tempdir().expect("temp source");
    write_valid_wav(&dir.path().join(name));
    let source = SampleSource::new_with_id(SourceId::from_string("native"), dir.path().into());
    (dir, source)
}

fn write_valid_wav(path: &std::path::Path) {
    write_valid_wav_with_frequency(path, 440.0);
}

fn write_valid_wav_with_frequency(path: &std::path::Path, frequency_hz: f32) {
    let spec = hound::WavSpec {
        channels: 1,
        sample_rate: wavecrate_analysis::ANALYSIS_SAMPLE_RATE,
        bits_per_sample: 16,
        sample_format: hound::SampleFormat::Int,
    };
    let mut writer = hound::WavWriter::create(path, spec).expect("create wav");
    for index in 0..wavecrate_analysis::ANALYSIS_SAMPLE_RATE / 10 {
        let phase = index as f32 / wavecrate_analysis::ANALYSIS_SAMPLE_RATE as f32;
        let sample = (phase * frequency_hz * std::f32::consts::TAU).sin() * i16::MAX as f32 * 0.2;
        writer
            .write_sample(sample as i16)
            .expect("write wav sample");
    }
    writer.finalize().expect("finalize wav");
}

fn count_jobs(source: &SampleSource) -> i64 {
    let conn = open_source_db(&source).expect("analysis db");
    conn.query_row("SELECT COUNT(*) FROM analysis_jobs", [], |row| row.get(0))
        .expect("job count")
}

fn count_jobs_by_type(source: &SampleSource, job_type: &str) -> i64 {
    let conn = open_source_db(&source).expect("analysis db");
    conn.query_row(
        "SELECT COUNT(*) FROM analysis_jobs WHERE job_type = ?1",
        [job_type],
        |row| row.get(0),
    )
    .expect("job count")
}

fn count_jobs_by_type_and_status(source: &SampleSource, job_type: &str, status: &str) -> i64 {
    let conn = open_source_db(&source).expect("analysis db");
    conn.query_row(
        "SELECT COUNT(*) FROM analysis_jobs WHERE job_type = ?1 AND status = ?2",
        [job_type, status],
        |row| row.get(0),
    )
    .expect("job count")
}

fn count_jobs_by_status(source: &SampleSource, status: &str) -> i64 {
    let conn = open_source_db(&source).expect("analysis db");
    conn.query_row(
        "SELECT COUNT(*) FROM analysis_jobs WHERE status = ?1",
        [status],
        |row| row.get(0),
    )
    .expect("job count")
}

fn count_similarity_embeddings(source: &SampleSource, relative_path: &str) -> i64 {
    let conn = open_source_db(&source).expect("analysis db");
    let sample_id = build_sample_id(source.id.as_str(), std::path::Path::new(relative_path));
    conn.query_row(
        "SELECT COUNT(*) FROM embeddings WHERE sample_id = ?1 AND model_id = ?2",
        rusqlite::params![sample_id, SIMILARITY_MODEL_ID],
        |row| row.get(0),
    )
    .expect("embedding count")
}

fn count_similarity_aspects(source: &SampleSource, relative_path: &str) -> i64 {
    let conn = open_source_db(&source).expect("analysis db");
    let sample_id = build_sample_id(source.id.as_str(), std::path::Path::new(relative_path));
    conn.query_row(
        "SELECT COUNT(*) FROM similarity_aspect_descriptors
         WHERE sample_id = ?1 AND model_id = ?2",
        rusqlite::params![sample_id, ASPECT_DESCRIPTOR_MODEL_ID],
        |row| row.get(0),
    )
    .expect("aspect count")
}

fn seed_current_analysis_artifacts(source: &SampleSource, relative_path: &str) {
    let conn = open_source_db(&source).expect("analysis db");
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
    conn.execute(
        "INSERT OR REPLACE INTO similarity_aspect_descriptors
         (sample_id, model_id, dim, dtype, l2_normed, valid_mask, vec, created_at)
         VALUES (?1, ?2, ?3, 'f32', 1, ?4, ?5, 0)",
        rusqlite::params![
            &sample_id,
            ASPECT_DESCRIPTOR_MODEL_ID,
            ASPECT_DESCRIPTOR_DIM as i64,
            wavecrate_analysis::aspects::all_aspect_mask() as i64,
            vec![0_u8; ASPECT_DESCRIPTOR_DIM * 4]
        ],
    )
    .expect("insert aspect descriptors");
}

fn seed_similarity_artifacts_without_features(source: &SampleSource, relative_path: &str) {
    seed_current_analysis_artifacts(source, relative_path);
    let conn = open_source_db(&source).expect("analysis db");
    let sample_id = build_sample_id(source.id.as_str(), std::path::Path::new(relative_path));
    conn.execute(
        "DELETE FROM features WHERE sample_id = ?1 AND feat_version = 1",
        rusqlite::params![&sample_id],
    )
    .expect("delete features");
    conn.execute(
        "INSERT OR REPLACE INTO layout_umap
         (sample_id, model_id, umap_version, x, y, created_at)
         VALUES (?1, ?2, 'v1', 0.0, 0.0, 0)",
        rusqlite::params![&sample_id, SIMILARITY_MODEL_ID],
    )
    .expect("insert layout");
}

fn seed_failed_analysis_job(source: &SampleSource, relative_path: &str) {
    seed_failed_analysis_job_with_error(source, relative_path, "decode failed");
}

fn seed_failed_analysis_job_with_error(source: &SampleSource, relative_path: &str, error: &str) {
    let conn = open_source_db(&source).expect("analysis db");
    let sample_id = build_sample_id(source.id.as_str(), std::path::Path::new(relative_path));
    conn.execute(
        "INSERT INTO analysis_jobs
         (sample_id, source_id, relative_path, job_type, content_hash, status, attempts, created_at, last_error)
         VALUES (?1, ?2, ?3, ?4, 'fast-8-10', 'failed', 1, 0, ?5)",
        rusqlite::params![
            sample_id,
            source.id.as_str(),
            relative_path,
            ANALYZE_SAMPLE_JOB_TYPE,
            error,
        ],
    )
    .expect("insert failed job");
}
