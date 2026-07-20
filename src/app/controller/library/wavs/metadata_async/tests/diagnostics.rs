use super::*;

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
    let db =
        SourceDatabase::open_for_test_fixture_source_write(&source.root).expect("open source db");
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
