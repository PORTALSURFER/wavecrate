use super::*;

#[test]
fn claim_selection_skips_source_with_file_op_write_priority() {
    let dir_a = TempDir::new().unwrap();
    let dir_b = TempDir::new().unwrap();
    let source_a = SampleSource::new(dir_a.path().to_path_buf());
    let source_b = SampleSource::new(dir_b.path().to_path_buf());
    let conn_a = analysis_db::open_source_db(&source_a.root).unwrap();
    let conn_b = analysis_db::open_source_db(&source_b.root).unwrap();
    conn_a
        .execute(
            "INSERT INTO analysis_jobs (sample_id, source_id, relative_path, job_type, status, attempts, created_at)
             VALUES (?1, ?2, ?3, ?4, 'pending', 0, 0)",
            rusqlite::params![
                format!("{}::a.wav", source_a.id),
                source_a.id.to_string(),
                "a.wav",
                analysis_db::ANALYZE_SAMPLE_JOB_TYPE
            ],
        )
        .unwrap();
    conn_b
        .execute(
            "INSERT INTO analysis_jobs (sample_id, source_id, relative_path, job_type, status, attempts, created_at)
             VALUES (?1, ?2, ?3, ?4, 'pending', 0, 0)",
            rusqlite::params![
                format!("{}::b.wav", source_b.id),
                source_b.id.to_string(),
                "b.wav",
                analysis_db::ANALYZE_SAMPLE_JOB_TYPE
            ],
        )
        .unwrap();
    let _guard =
        crate::app::controller::library::source_write_priority::FileOpWritePriorityGuard::new(
            &source_a.id,
        );
    let reset_done = Arc::new(Mutex::new(HashSet::new()));
    let claim_wakeup = ClaimWakeup::new();
    let mut selector = selection::ClaimSelector::with_sources_for_tests(
        vec![
            super::super::claim::SourceClaimDb {
                source: source_a,
                conn: conn_a,
            },
            super::super::claim::SourceClaimDb {
                source: source_b,
                conn: conn_b,
            },
        ],
        1,
        reset_done,
    );

    let claimed = match selector.select_next(None, &claim_wakeup) {
        selection::ClaimSelection::Job(job) => job,
        _ => panic!("expected a job from the unrelated source"),
    };

    assert!(claimed.sample_id.ends_with("b.wav"));
}
