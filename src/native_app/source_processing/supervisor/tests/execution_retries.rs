#[test]
fn unavailable_hash_path_backs_off_without_starving_later_files() {
    let directory = tempfile::tempdir().expect("temporary hash source");
    let good_path = directory.path().join("z-good.wav");
    std::fs::write(&good_path, [7_u8; 64]).expect("write hashable sample");
    let source = SampleSource::new_with_id(
        SourceId::from_string("hash-fairness"),
        directory.path().to_path_buf(),
    );
    let db = source.open_db().expect("open hash source database");
    db.upsert_file(Path::new("a-unavailable.wav"), 64, 1)
        .expect("insert unavailable hash row");
    db.upsert_file(Path::new("z-good.wav"), 64, 1)
        .expect("insert good hash row");
    let database_root = source.database_root().expect("database root");
    let connection = SourceDatabase::open_connection_with_role_and_database_root(
        &source.root,
        &database_root,
        SourceDatabaseConnectionRole::JobWorker,
    )
    .expect("open hash database");
    connection
        .execute_batch(
            "UPDATE wav_files SET file_identity = 'identity-bad'
                 WHERE path = 'a-unavailable.wav';
                 UPDATE wav_files SET file_identity = 'identity-good'
                 WHERE path = 'z-good.wav';",
        )
        .expect("assign hash identities");
    connection
        .execute(
            "INSERT INTO metadata (key, value) VALUES (?1, ?2)
                 ON CONFLICT(key) DO UPDATE SET value = excluded.value",
            params![META_LAST_MANIFEST_AUDIT_AT, now_epoch_seconds().to_string()],
        )
        .expect("mark the fixture manifest audit current");
    drop(connection);

    let mut supervisor =
        SourceProcessingSupervisor::start_without_forced_manifest_audit(vec![source.clone()]);
    let deadline = Instant::now() + Duration::from_secs(10);
    loop {
        let good_hashed = source
            .open_db()
            .expect("open hash source")
            .entry_for_path(Path::new("z-good.wav"))
            .expect("read good hash row")
            .and_then(|entry| entry.content_hash)
            .is_some();
        let failure_recorded = SourceDatabase::open_connection_with_role_and_database_root(
            &source.root,
            &database_root,
            SourceDatabaseConnectionRole::JobWorker,
        )
        .ok()
        .and_then(|connection| {
            connection
                .query_row(
                    "SELECT status
                         FROM analysis_jobs
                         WHERE readiness_managed = 1
                           AND readiness_stage = 'indexed_identity'
                           AND relative_path = 'a-unavailable.wav'",
                    [],
                    |row| row.get::<_, String>(0),
                )
                .optional()
                .ok()
                .flatten()
        })
        .as_deref()
            == Some("failed");
        if good_hashed && failure_recorded {
            break;
        }
        if Instant::now() >= deadline {
            let telemetry = supervisor.shared.telemetry();
            panic!(
                "hash fairness did not converge: good_hashed={good_hashed} \
                     unavailable_status={failure_recorded} claimed={} completed={} failed={} \
                     retried={} stale={} queue_depth={}",
                telemetry.claimed,
                telemetry.completed,
                telemetry.failed,
                telemetry.retried,
                telemetry.stale,
                telemetry.queue_depth,
            );
        }
        thread::sleep(Duration::from_millis(20));
    }
    assert_eq!(supervisor.shutdown()["joined"], true);

    let connection = SourceDatabase::open_connection_with_role_and_database_root(
        &source.root,
        &database_root,
        SourceDatabaseConnectionRole::JobWorker,
    )
    .expect("reopen hash database");
    let failure: (String, Option<String>, Option<i64>, i64) = connection
        .query_row(
            "SELECT status, failure_kind, retry_at, attempts
                 FROM analysis_jobs
                 WHERE readiness_managed = 1
                   AND readiness_stage = 'indexed_identity'
                   AND relative_path = 'a-unavailable.wav'",
            [],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?)),
        )
        .expect("read durable hash failure");
    assert_eq!(failure.0, "failed");
    assert_eq!(failure.1.as_deref(), Some("retryable"));
    assert!(failure.2.is_some());
    assert_eq!(failure.3, 1);
}

#[test]
fn legacy_source_schema_is_not_eligible_for_automatic_processing() {
    let mut connection = rusqlite::Connection::open_in_memory().unwrap();
    connection
        .execute_batch(
            "CREATE TABLE wav_files (
                    path TEXT PRIMARY KEY,
                    file_size INTEGER NOT NULL,
                    modified_ns INTEGER NOT NULL
                 );
                 CREATE TABLE analysis_jobs (
                    id INTEGER PRIMARY KEY,
                    relative_path TEXT NOT NULL,
                    job_type TEXT NOT NULL,
                    created_at INTEGER NOT NULL,
                    status TEXT NOT NULL
                 );
                 CREATE TABLE metadata (key TEXT PRIMARY KEY, value TEXT NOT NULL);",
        )
        .unwrap();

    assert!(!source_processing_schema_available(&mut connection).unwrap());
}

#[test]
fn real_hash_execution_waits_for_shared_scan_database_budget() {
    let (_directory, source) = unhashed_source("shared-budget");
    let (sender, receiver) = std::sync::mpsc::channel();
    let shared = Arc::new(Shared::new(vec![source.clone()], Some(Arc::new(sender))));
    let permit = SourceProcessingBudgetHandle {
        shared: Arc::clone(&shared),
    }
    .acquire_scan(source.id.as_str())
    .expect("acquire external scan budget");
    let coordinator_shared = Arc::clone(&shared);
    let coordinator = thread::Builder::new()
        .name(String::from("wavecrate-source-supervisor-test"))
        .spawn(move || run_coordinator(coordinator_shared))
        .expect("spawn source processing supervisor");
    let mut supervisor = SourceProcessingSupervisor {
        shared,
        coordinator: Some(coordinator),
        retirement_worker: None,
    };
    thread::sleep(Duration::from_millis(150));
    assert!(!source_is_hashed(&source));
    assert!(
        supervisor.shared.telemetry().sweeps <= 2,
        "a source blocked by an external scan must park instead of rediscovering in a tight loop"
    );
    assert!(
        receiver
            .try_iter()
            .all(|event| matches!(event, SourceProcessingEvent::Completed)),
        "queued work must not publish active progress while foreground admission owns the lane"
    );

    drop(permit);
    wait_until(Duration::from_secs(3), || source_is_hashed(&source));
    assert_eq!(supervisor.shutdown()["joined"], true);
}

#[test]
fn external_scan_tokens_survive_playback_but_cancel_for_removal_and_shutdown() {
    let (_directory, source) = unhashed_source("external-cancel");
    let mut supervisor = SourceProcessingSupervisor::dormant();
    supervisor
        .replace_sources(vec![source.clone()])
        .expect("configure source");

    let playback_permit = supervisor
        .budget_handle()
        .acquire_scan(source.id.as_str())
        .expect("acquire playback permit");
    let playback_cancel = playback_permit.cancel_token();
    assert!(!playback_cancel.load(Ordering::Acquire));
    supervisor.set_playback_active(true);
    assert!(
        !playback_cancel.load(Ordering::Acquire),
        "playback must not cancel source scanning"
    );
    drop(playback_permit);

    supervisor.set_playback_active(false);
    let removal_permit = supervisor
        .budget_handle()
        .acquire_scan(source.id.as_str())
        .expect("acquire removal permit");
    let removal_cancel = removal_permit.cancel_token();
    let removal_cancel_for_releaser = Arc::clone(&removal_cancel);
    let releaser = thread::spawn(move || {
        while !removal_cancel_for_releaser.load(Ordering::Acquire) {
            thread::yield_now();
        }
        drop(removal_permit);
    });
    supervisor
        .replace_sources(Vec::new())
        .expect("remove configured source");
    releaser.join().expect("join removal scan releaser");
    assert!(removal_cancel.load(Ordering::Acquire));

    supervisor
        .replace_sources(vec![source.clone()])
        .expect("restore configured source");
    process_ready_source_retirements(&supervisor.shared);
    let shutdown_permit = supervisor
        .budget_handle()
        .acquire_scan(source.id.as_str())
        .expect("acquire shutdown permit");
    let shutdown_cancel = shutdown_permit.cancel_token();
    let release_cancel = Arc::clone(&shutdown_cancel);
    let releaser = thread::spawn(move || {
        while !release_cancel.load(Ordering::Acquire) {
            thread::yield_now();
        }
        drop(shutdown_permit);
    });
    let wake_generation = supervisor.shared.control().wake_generation;
    let report = supervisor.shutdown();
    releaser.join().expect("join external scan releaser");
    assert_eq!(report["joined"], true);
    assert_eq!(report["external_scans_joined"], true);
    assert!(shutdown_cancel.load(Ordering::Acquire));
    assert!(supervisor.shared.control().wake_generation > wake_generation);
}

#[test]
fn readiness_candidates_preserve_durable_queue_creation_time() {
    let (_directory, source) = unhashed_source("queue-age");
    let database_root = source.database_root().expect("database root");
    let mut connection = SourceDatabase::open_connection_with_role_and_database_root(
        &source.root,
        &database_root,
        SourceDatabaseConnectionRole::JobWorker,
    )
    .expect("open readiness database");
    connection
        .execute(
            "UPDATE wav_files
                 SET file_identity = 'queue-identity', content_hash = 'queue-content'
                 WHERE path = 'pending.wav'",
            [],
        )
        .expect("assign queue identity");
    assert!(
        publish_current_readiness_targets(&mut connection, source.id.as_str(), 100)
            .expect("publish current targets")
    );
    let snapshot =
        reconcile_readiness(&connection, source.id.as_str(), 100).expect("reconcile targets");
    persist_readiness_deficits(&mut connection, &snapshot.deficits, 100).expect("persist deficits");
    drop(connection);

    let cancel = AtomicBool::new(false);
    let Cancellable::Completed((candidates, _)) =
        discover_source_candidates(&source, 250, false, &cancel).expect("rediscover work")
    else {
        panic!("source rediscovery unexpectedly cancelled");
    };
    let readiness = candidates
        .iter()
        .filter(|candidate| matches!(candidate.task, RuntimeTask::Readiness(_)))
        .collect::<Vec<_>>();
    assert!(!readiness.is_empty());
    assert!(
        readiness
            .iter()
            .all(|candidate| candidate.schedule.enqueued_at == 100)
    );
    assert!(
        readiness.iter().all(|candidate| !matches!(
            candidate.task,
            RuntimeTask::Readiness(ref target)
                if target.stage == ReadinessStage::SimilarityLayout
        )),
        "similarity layout must stay parked behind the pending embedding target"
    );
    assert_eq!(oldest_job_age_seconds(&candidates, 250), 150);
}
