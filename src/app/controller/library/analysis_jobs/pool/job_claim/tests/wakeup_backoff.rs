use super::*;

#[test]
fn claim_wakeup_allows_only_one_idle_probe_per_backoff_window() {
    let wakeup = ClaimWakeup::new();
    let mut worker_a = 0;
    let mut worker_b = 0;

    assert!(wakeup.acquire_probe_or_wait(&mut worker_a).is_none());
    let wait = wakeup
        .acquire_probe_or_wait(&mut worker_b)
        .expect("second worker should back off");
    assert!(wait <= Duration::from_millis(5));
    assert!(wakeup.probe_inflight());

    wakeup.finish_probe(Duration::from_millis(200));
    let wait = wakeup
        .acquire_probe_or_wait(&mut worker_b)
        .expect("backoff should remain active");
    assert!(wait > Duration::from_millis(0));

    wakeup.notify();
    assert!(wakeup.acquire_probe_or_wait(&mut worker_b).is_none());
}

#[test]
fn notified_claim_wakeup_immediately_rechecks_sources_after_idle_backoff() {
    let dir = TempDir::new().unwrap();
    let source = SampleSource::new(dir.path().to_path_buf());
    let selector_conn = analysis_db::open_source_db(&source.root).unwrap();
    let reset_done = Arc::new(Mutex::new(HashSet::new()));
    let claim_wakeup = ClaimWakeup::new();
    let mut selector = selection::ClaimSelector::with_sources_for_tests(
        vec![super::super::claim::SourceClaimDb {
            source: source.clone(),
            conn: selector_conn,
        }],
        1,
        reset_done,
    );
    let mut wake_counter = 0u64;

    assert!(
        claim_wakeup
            .acquire_probe_or_wait(&mut wake_counter)
            .is_none()
    );
    assert!(matches!(
        selector.select_next(None, &claim_wakeup),
        selection::ClaimSelection::Idle
    ));
    let wait = claim_wakeup
        .acquire_probe_or_wait(&mut wake_counter)
        .expect("idle backoff should start after an empty probe");
    assert!(wait > Duration::ZERO);

    let conn = analysis_db::open_source_db(&source.root).unwrap();
    conn.execute(
        "INSERT INTO analysis_jobs (sample_id, source_id, relative_path, job_type, status, attempts, created_at)
         VALUES (?1, ?2, ?3, ?4, 'pending', 0, 0)",
        rusqlite::params![
            format!("{}::fresh.wav", source.id),
            source.id.to_string(),
            "fresh.wav",
            analysis_db::ANALYZE_SAMPLE_JOB_TYPE
        ],
    )
    .unwrap();

    claim_wakeup.notify();
    assert!(
        claim_wakeup
            .acquire_probe_or_wait(&mut wake_counter)
            .is_none(),
        "new work should bypass the idle backoff immediately"
    );
    let claimed = match selector.select_next(None, &claim_wakeup) {
        selection::ClaimSelection::Job(job) => job,
        _ => panic!("expected notify to let the next probe claim new work"),
    };

    assert!(claimed.sample_id.ends_with("fresh.wav"));
}
