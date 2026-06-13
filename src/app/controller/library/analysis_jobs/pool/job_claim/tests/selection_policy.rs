use super::*;

#[test]
fn claim_selection_orders_sources_round_robin() {
    let dir_a = TempDir::new().unwrap();
    let dir_b = TempDir::new().unwrap();
    let source_a = SampleSource::new(dir_a.path().to_path_buf());
    let source_b = SampleSource::new(dir_b.path().to_path_buf());
    let conn_a = analysis_db::open_source_db(&source_a.root).unwrap();
    let conn_b = analysis_db::open_source_db(&source_b.root).unwrap();
    let sample_a = format!("{}::a.wav", source_a.id);
    let sample_b = format!("{}::b.wav", source_b.id);
    conn_a
        .execute(
            "INSERT INTO analysis_jobs (sample_id, source_id, relative_path, job_type, status, attempts, created_at)
             VALUES (?1, ?2, ?3, ?4, 'pending', 0, 0)",
            rusqlite::params![
                sample_a,
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
                sample_b,
                source_b.id.to_string(),
                "b.wav",
                analysis_db::ANALYZE_SAMPLE_JOB_TYPE
            ],
        )
        .unwrap();
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
    let mut wake_counter = 0;
    assert!(
        claim_wakeup
            .acquire_probe_or_wait(&mut wake_counter)
            .is_none()
    );
    let first = match selector.select_next(None, &claim_wakeup) {
        selection::ClaimSelection::Job(job) => job,
        _ => panic!("expected a job from first source"),
    };
    assert!(
        claim_wakeup
            .acquire_probe_or_wait(&mut wake_counter)
            .is_none()
    );
    let second = match selector.select_next(None, &claim_wakeup) {
        selection::ClaimSelection::Job(job) => job,
        _ => panic!("expected a job from second source"),
    };

    assert!(first.sample_id.ends_with("a.wav"));
    assert!(second.sample_id.ends_with("b.wav"));
}
