
use super::*;
use crate::app::controller::library::source_write_priority;
use std::io;
use std::path::Path;
use std::sync::{Arc, LazyLock, Mutex, MutexGuard};
use tracing_subscriber::fmt::MakeWriter;

static METADATA_ASYNC_TEST_LOCK: LazyLock<Mutex<()>> = LazyLock::new(|| Mutex::new(()));

fn metadata_async_test_lock() -> MutexGuard<'static, ()> {
    METADATA_ASYNC_TEST_LOCK
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
}

#[derive(Clone, Default)]
struct SharedBuffer(Arc<Mutex<Vec<u8>>>);

impl SharedBuffer {
    fn captured(&self) -> String {
        String::from_utf8(self.0.lock().unwrap().clone()).unwrap()
    }
}

impl<'a> MakeWriter<'a> for SharedBuffer {
    type Writer = SharedBufferWriter;

    fn make_writer(&'a self) -> Self::Writer {
        SharedBufferWriter(self.0.clone())
    }
}

struct SharedBufferWriter(Arc<Mutex<Vec<u8>>>);

impl io::Write for SharedBufferWriter {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        self.0.lock().unwrap().extend_from_slice(buf);
        Ok(buf.len())
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

fn capture_info_logs<F>(run: F) -> String
where
    F: FnOnce(),
{
    let buffer = SharedBuffer::default();
    let subscriber = tracing_subscriber::fmt()
        .with_ansi(false)
        .without_time()
        .with_max_level(tracing::Level::INFO)
        .with_writer(buffer.clone())
        .finish();
    tracing::subscriber::with_default(subscriber, run);
    buffer.captured()
}

#[test]
fn metadata_mutation_paths_dedup_across_source_and_analysis_ops() {
    let _lock = metadata_async_test_lock();
    let paths = metadata_mutation_paths(
        &[
            SourceMetadataMutationOp::SetLooped {
                relative_path: PathBuf::from("one.wav"),
                looped: true,
            },
            SourceMetadataMutationOp::SetLastPlayedAt {
                relative_path: PathBuf::from("two.wav"),
                played_at: 5,
            },
        ],
        &[
            AnalysisMetadataMutationOp::SetBpm {
                relative_path: PathBuf::from("one.wav"),
                bpm: Some(120.0),
            },
            AnalysisMetadataMutationOp::SetLoadedDuration {
                relative_path: PathBuf::from("two.wav"),
                duration_seconds: 1.0,
                sample_rate: 44_100,
                long_sample_mark: Some(false),
            },
        ],
    );

    assert_eq!(
        paths.into_iter().collect::<Vec<_>>(),
        vec![PathBuf::from("one.wav"), PathBuf::from("two.wav")]
    );
}

#[test]
fn metadata_mutation_waits_behind_same_source_file_op_priority() {
    let _lock = metadata_async_test_lock();
    let temp = tempfile::tempdir().expect("create temp dir");
    let source = SampleSource::new(temp.path().join("source"));
    std::fs::create_dir_all(&source.root).expect("create source root");
    let relative_path = PathBuf::from("alpha.wav");
    let db = SourceDatabase::open(&source.root).expect("open source db");
    db.upsert_file(&relative_path, 1, 1)
        .expect("insert source row");
    source_write_priority::begin_file_op_write_priority(&source.id);
    let release_source_id = source.id.clone();
    std::thread::spawn(move || {
        std::thread::sleep(Duration::from_millis(260));
        source_write_priority::finish_file_op_write_priority(&release_source_id);
    });

    let result = run_metadata_mutation_job(MetadataMutationJob {
        request_id: 7,
        source_id: source.id.clone(),
        source_root: source.root.clone(),
        paths: [relative_path.clone()].into_iter().collect(),
        source_ops: vec![SourceMetadataMutationOp::SetUserTag {
            relative_path: relative_path.clone(),
            user_tag: Some(String::from("Vintage")),
        }],
        analysis_ops: Vec::new(),
    });

    assert!(result.elapsed >= Duration::from_millis(200));
    assert!(result.result.is_ok(), "{:?}", result.result);
    assert_eq!(
        db.user_tag_for_path(&relative_path).expect("read user tag"),
        Some(String::from("Vintage"))
    );
}

fn persisted_duration_seconds(source: &SampleSource, relative_path: &Path) -> Option<f64> {
    let sample_id = analysis_jobs::build_sample_id(source.id.as_str(), relative_path);
    let conn = analysis_jobs::open_source_db(&source.root).expect("open analysis db");
    conn.query_row(
        "SELECT duration_seconds FROM samples WHERE sample_id = ?1",
        rusqlite::params![sample_id],
        |row| row.get::<_, Option<f64>>(0),
    )
    .ok()
    .flatten()
}

#[test]
fn loaded_duration_metadata_job_follows_completed_browser_rename() {
    let _lock = metadata_async_test_lock();
    let temp = tempfile::tempdir().expect("create temp dir");
    let source = SampleSource::new(temp.path().join("source"));
    std::fs::create_dir_all(&source.root).expect("create source root");
    let old_relative = PathBuf::from("old-name.wav");
    let new_relative = PathBuf::from("new-name.wav");
    let old_absolute = source.root.join(&old_relative);
    let new_absolute = source.root.join(&new_relative);
    std::fs::write(&old_absolute, b"metadata-fixture").expect("write fixture");

    let db = SourceDatabase::open(&source.root).expect("open source db");
    let (old_size, old_modified_ns) =
        crate::app::controller::library::wav_io::file_metadata(&old_absolute)
            .expect("old metadata");
    db.upsert_file(&old_relative, old_size, old_modified_ns)
        .expect("insert old row");
    std::fs::rename(&old_absolute, &new_absolute).expect("rename fixture");
    let (new_size, new_modified_ns) =
        crate::app::controller::library::wav_io::file_metadata(&new_absolute)
            .expect("new metadata");
    let mut batch = db.write_batch().expect("start rename batch");
    batch.remove_file(&old_relative).expect("remove old row");
    batch
        .upsert_file(&new_relative, new_size, new_modified_ns)
        .expect("insert new row");
    batch
        .remap_analysis_sample_identity(&old_relative, &new_relative)
        .expect("remap analysis identity");
    batch.commit().expect("commit rename batch");
    source_write_priority::record_completed_browser_rename(
        &source.id,
        &old_relative,
        &new_relative,
    );

    let result = run_metadata_mutation_job(MetadataMutationJob {
        request_id: 11,
        source_id: source.id.clone(),
        source_root: source.root.clone(),
        paths: [old_relative.clone()].into_iter().collect(),
        source_ops: Vec::new(),
        analysis_ops: vec![AnalysisMetadataMutationOp::SetLoadedDuration {
            relative_path: old_relative.clone(),
            duration_seconds: 2.5,
            sample_rate: 44_100,
            long_sample_mark: Some(false),
        }],
    });

    assert!(result.result.is_ok(), "{:?}", result.result);
    assert!(persisted_duration_seconds(&source, &old_relative).is_none());
    assert_eq!(
        persisted_duration_seconds(&source, &new_relative),
        Some(2.5)
    );
}

#[test]
#[cfg(debug_assertions)]
fn analysis_metadata_rename_resolution_reuses_one_source_db_open() {
    let _lock = metadata_async_test_lock();
    let temp = tempfile::tempdir().expect("create temp dir");
    let source = SampleSource::new(temp.path().join("source"));
    std::fs::create_dir_all(&source.root).expect("create source root");
    let old_relative = PathBuf::from("old-name.wav");
    let new_relative = PathBuf::from("new-name.wav");
    let other_old_relative = PathBuf::from("other-old-name.wav");
    let other_new_relative = PathBuf::from("other-new-name.wav");
    let old_absolute = source.root.join(&old_relative);
    let new_absolute = source.root.join(&new_relative);
    let other_old_absolute = source.root.join(&other_old_relative);
    let other_new_absolute = source.root.join(&other_new_relative);
    std::fs::write(&old_absolute, b"metadata-fixture").expect("write fixture");
    std::fs::write(&other_old_absolute, b"other-metadata-fixture").expect("write fixture");

    let db = SourceDatabase::open(&source.root).expect("open source db");
    let (old_size, old_modified_ns) =
        crate::app::controller::library::wav_io::file_metadata(&old_absolute)
            .expect("old metadata");
    db.upsert_file(&old_relative, old_size, old_modified_ns)
        .expect("insert old row");
    let (other_old_size, other_old_modified_ns) =
        crate::app::controller::library::wav_io::file_metadata(&other_old_absolute)
            .expect("other old metadata");
    db.upsert_file(&other_old_relative, other_old_size, other_old_modified_ns)
        .expect("insert other old row");
    std::fs::rename(&old_absolute, &new_absolute).expect("rename fixture");
    std::fs::rename(&other_old_absolute, &other_new_absolute).expect("rename fixture");
    let (new_size, new_modified_ns) =
        crate::app::controller::library::wav_io::file_metadata(&new_absolute)
            .expect("new metadata");
    let (other_new_size, other_new_modified_ns) =
        crate::app::controller::library::wav_io::file_metadata(&other_new_absolute)
            .expect("other new metadata");
    let mut batch = db.write_batch().expect("start rename batch");
    batch.remove_file(&old_relative).expect("remove old row");
    batch
        .upsert_file(&new_relative, new_size, new_modified_ns)
        .expect("insert new row");
    batch
        .remap_analysis_sample_identity(&old_relative, &new_relative)
        .expect("remap analysis identity");
    batch
        .remove_file(&other_old_relative)
        .expect("remove other old row");
    batch
        .upsert_file(&other_new_relative, other_new_size, other_new_modified_ns)
        .expect("insert other new row");
    batch
        .remap_analysis_sample_identity(&other_old_relative, &other_new_relative)
        .expect("remap other analysis identity");
    batch.commit().expect("commit rename batch");
    source_write_priority::record_completed_browser_rename(
        &source.id,
        &old_relative,
        &new_relative,
    );
    source_write_priority::record_completed_browser_rename(
        &source.id,
        &other_old_relative,
        &other_new_relative,
    );

    crate::sample_sources::db::test_reset_source_db_open_total_count(&source.root);
    let result = run_metadata_mutation_job(MetadataMutationJob {
        request_id: 19,
        source_id: source.id.clone(),
        source_root: source.root.clone(),
        paths: [old_relative.clone(), other_old_relative.clone()]
            .into_iter()
            .collect(),
        source_ops: Vec::new(),
        analysis_ops: vec![
            AnalysisMetadataMutationOp::SetLoadedDuration {
                relative_path: old_relative.clone(),
                duration_seconds: 3.0,
                sample_rate: 44_100,
                long_sample_mark: Some(false),
            },
            AnalysisMetadataMutationOp::SetLoadedDuration {
                relative_path: other_old_relative.clone(),
                duration_seconds: 4.0,
                sample_rate: 44_100,
                long_sample_mark: Some(false),
            },
        ],
    });

    assert!(result.result.is_ok(), "{:?}", result.result);
    assert_eq!(
        crate::sample_sources::db::test_source_db_open_total_count(&source.root),
        2,
        "analysis metadata should open once for writes and once for all rename-resolution reads"
    );
    assert_eq!(
        persisted_duration_seconds(&source, &new_relative),
        Some(3.0)
    );
    assert_eq!(
        persisted_duration_seconds(&source, &other_new_relative),
        Some(4.0)
    );
}

#[test]
fn source_metadata_job_follows_completed_browser_rename() {
    let _lock = metadata_async_test_lock();
    let temp = tempfile::tempdir().expect("create temp dir");
    let source = SampleSource::new(temp.path().join("source"));
    std::fs::create_dir_all(&source.root).expect("create source root");
    let old_relative = PathBuf::from("old-name.wav");
    let new_relative = PathBuf::from("new-name.wav");
    let old_absolute = source.root.join(&old_relative);
    let new_absolute = source.root.join(&new_relative);
    std::fs::write(&old_absolute, b"metadata-fixture").expect("write fixture");

    let db = SourceDatabase::open(&source.root).expect("open source db");
    let (old_size, old_modified_ns) =
        crate::app::controller::library::wav_io::file_metadata(&old_absolute)
            .expect("old metadata");
    db.upsert_file(&old_relative, old_size, old_modified_ns)
        .expect("insert old row");
    std::fs::rename(&old_absolute, &new_absolute).expect("rename fixture");
    let (new_size, new_modified_ns) =
        crate::app::controller::library::wav_io::file_metadata(&new_absolute)
            .expect("new metadata");
    let mut batch = db.write_batch().expect("start rename batch");
    batch.remove_file(&old_relative).expect("remove old row");
    batch
        .upsert_file(&new_relative, new_size, new_modified_ns)
        .expect("insert new row");
    batch.commit().expect("commit rename batch");
    source_write_priority::record_completed_browser_rename(
        &source.id,
        &old_relative,
        &new_relative,
    );

    let result = run_metadata_mutation_job(MetadataMutationJob {
        request_id: 13,
        source_id: source.id.clone(),
        source_root: source.root.clone(),
        paths: [old_relative.clone()].into_iter().collect(),
        source_ops: vec![
            SourceMetadataMutationOp::SetLooped {
                relative_path: old_relative.clone(),
                looped: true,
            },
            SourceMetadataMutationOp::AssignNormalTag {
                relative_path: old_relative.clone(),
                label: String::from("Vintage Loop"),
            },
        ],
        analysis_ops: Vec::new(),
    });

    assert!(result.result.is_ok(), "{:?}", result.result);
    assert_eq!(db.looped_for_path(&old_relative).expect("old looped"), None);
    assert_eq!(
        db.looped_for_path(&new_relative).expect("new looped"),
        Some(true)
    );
    assert_eq!(
        db.tags_for_path(&new_relative)
            .expect("new tags")
            .into_iter()
            .map(|tag| tag.display_label)
            .collect::<Vec<_>>(),
        vec![String::from("Vintage Loop")]
    );
}

#[test]
fn source_metadata_job_logs_operation_path_remap_and_result() {
    let _lock = metadata_async_test_lock();
    let temp = tempfile::tempdir().expect("create temp dir");
    let source = SampleSource::new(temp.path().join("source"));
    std::fs::create_dir_all(&source.root).expect("create source root");
    let old_relative = PathBuf::from("old-name.wav");
    let new_relative = PathBuf::from("new-name.wav");
    let old_absolute = source.root.join(&old_relative);
    let new_absolute = source.root.join(&new_relative);
    std::fs::write(&old_absolute, b"metadata-fixture").expect("write fixture");

    let db = SourceDatabase::open(&source.root).expect("open source db");
    let (old_size, old_modified_ns) =
        crate::app::controller::library::wav_io::file_metadata(&old_absolute)
            .expect("old metadata");
    db.upsert_file(&old_relative, old_size, old_modified_ns)
        .expect("insert old row");
    std::fs::rename(&old_absolute, &new_absolute).expect("rename fixture");
    let (new_size, new_modified_ns) =
        crate::app::controller::library::wav_io::file_metadata(&new_absolute)
            .expect("new metadata");
    let mut batch = db.write_batch().expect("start rename batch");
    batch.remove_file(&old_relative).expect("remove old row");
    batch
        .upsert_file(&new_relative, new_size, new_modified_ns)
        .expect("insert new row");
    batch.commit().expect("commit rename batch");
    source_write_priority::record_completed_browser_rename(
        &source.id,
        &old_relative,
        &new_relative,
    );

    let captured = capture_info_logs(|| {
        let result = run_metadata_mutation_job(MetadataMutationJob {
            request_id: 17,
            source_id: source.id.clone(),
            source_root: source.root.clone(),
            paths: [old_relative.clone()].into_iter().collect(),
            source_ops: vec![SourceMetadataMutationOp::SetLooped {
                relative_path: old_relative.clone(),
                looped: true,
            }],
            analysis_ops: Vec::new(),
        });
        assert!(result.result.is_ok(), "{:?}", result.result);
    });

    assert!(
        captured.contains("source metadata mutation: source ops resolved"),
        "source metadata batch should log resolved operations: {captured}"
    );
    assert!(
        captured.contains("request_id=17")
            && captured.contains("op_count=1")
            && captured.contains("result=\"ok\"")
            && captured.contains("SetLooped old-name.wav -> new-name.wav remapped=true"),
        "log should include op name, original path, resolved path, and result: {captured}"
    );
}

#[test]
fn source_metadata_job_reports_operation_and_path_when_row_is_missing() {
    let _lock = metadata_async_test_lock();
    let temp = tempfile::tempdir().expect("create temp dir");
    let source = SampleSource::new(temp.path().join("source"));
    std::fs::create_dir_all(&source.root).expect("create source root");
    let relative_path = PathBuf::from("missing.wav");

    let result = run_metadata_mutation_job(MetadataMutationJob {
        request_id: 14,
        source_id: source.id.clone(),
        source_root: source.root.clone(),
        paths: [relative_path.clone()].into_iter().collect(),
        source_ops: vec![SourceMetadataMutationOp::SetLooped {
            relative_path: relative_path.clone(),
            looped: true,
        }],
        analysis_ops: Vec::new(),
    });

    let err = result.result.expect_err("missing source row should fail");
    assert!(
        err.contains("SetLooped")
            && err.contains("missing.wav")
            && err.contains("SQLite returned an unexpected result"),
        "expected operation and path context, got: {err}"
    );
}

#[test]
fn loaded_duration_metadata_job_reports_missing_file_without_rename_mapping() {
    let _lock = metadata_async_test_lock();
    let temp = tempfile::tempdir().expect("create temp dir");
    let source = SampleSource::new(temp.path().join("source"));
    std::fs::create_dir_all(&source.root).expect("create source root");
    let relative_path = PathBuf::from("missing.wav");
    let db = SourceDatabase::open(&source.root).expect("open source db");
    db.upsert_file(&relative_path, 1, 1)
        .expect("insert source row");

    let result = run_metadata_mutation_job(MetadataMutationJob {
        request_id: 12,
        source_id: source.id.clone(),
        source_root: source.root.clone(),
        paths: [relative_path.clone()].into_iter().collect(),
        source_ops: Vec::new(),
        analysis_ops: vec![AnalysisMetadataMutationOp::SetLoadedDuration {
            relative_path: relative_path.clone(),
            duration_seconds: 1.0,
            sample_rate: 44_100,
            long_sample_mark: None,
        }],
    });

    let err = result.result.expect_err("missing file should still fail");
    assert!(
        err.contains("Failed to read") && err.contains("missing.wav"),
        "expected actionable missing-file error, got: {err}"
    );
}
