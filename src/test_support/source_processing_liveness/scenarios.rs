use super::*;

#[test]
fn liveness_oracle_rejects_actionable_deficits_without_observable_runtime_work() {
    let directory = tempfile::tempdir().expect("liveness oracle source");
    write_test_wav(&directory.path().join("pending.wav"), 0.25);
    let source = SampleSource::new_with_id(
        SourceId::from_string("liveness-oracle"),
        directory.path().to_path_buf(),
    );
    let database = source.open_db().expect("open liveness oracle database");
    audit_source_and_record(&database, None, usize::MAX, now_epoch_seconds())
        .expect("seed liveness oracle manifest");
    let mut connection = open_connection(&source).expect("open liveness oracle connection");
    publish_current_readiness_targets(&mut connection, source.id.as_str(), now_epoch_seconds())
        .expect("publish liveness oracle targets");
    let snapshot = reconcile_readiness(&connection, source.id.as_str(), now_epoch_seconds())
        .expect("reconcile liveness oracle");
    assert!(!snapshot.deficits.is_empty());

    let runtime = RuntimeObservation {
        coordinator_running: true,
        source_configured: true,
        source_active: true,
        source_dirty: false,
        source_quarantined: false,
        processing_paused: false,
        wake_generation: 1,
        settled_wake_generation: 1,
        wake_reason: "test",
        lifecycle_generation: Some(1),
        in_flight: 0,
        active_budget: false,
        queue_depth: 0,
        readiness_queue_depth: 0,
        retries_due: 0,
        sweeps: 1,
        claimed: 0,
        completed: 0,
        failed: 0,
        retried: 0,
        stale: 0,
        cancelled: 0,
        contention: 0,
        oldest_job_age_seconds: 0,
    };
    assert!(silently_idle(&snapshot, &runtime));

    let scheduled = RuntimeObservation {
        source_dirty: true,
        ..runtime
    };
    assert!(!silently_idle(&snapshot, &scheduled));
}

#[test]
#[ignore = "explicit deterministic end-to-end source-processing liveness lane"]
fn source_processing_liveness_harness_converges_restart_churn_and_root_recovery() {
    let mut harness = LivenessHarness::new("source-processing-liveness");
    let initial = harness.await_fully_ready();
    let initial_generation = initial.source_generation;

    let nested = PathBuf::from("nested/snare.wav");
    fs::create_dir_all(harness.source.root.join("nested")).expect("create nested folder");
    write_test_wav(&harness.source.root.join(&nested), 0.5);
    harness.commit_targeted_paths(vec![nested.clone()], WatcherStimulus::Targeted);
    let after_nested_create = harness.await_fully_ready();
    assert!(after_nested_create.source_generation > initial_generation);

    let kick = harness.source.root.join("kick.wav");
    let original_modified = fs::metadata(&kick)
        .expect("kick metadata")
        .modified()
        .expect("kick modified time");
    write_test_wav(&kick, 1.0);
    let file = fs::OpenOptions::new()
        .write(true)
        .open(&kick)
        .expect("reopen same-size kick");
    file.set_times(fs::FileTimes::new().set_modified(original_modified))
        .expect("restore same-size kick timestamp");
    harness.commit_targeted_paths(vec![PathBuf::from("kick.wav")], WatcherStimulus::Targeted);
    harness.await_fully_ready();

    let moved = PathBuf::from("nested/moved-snare.wav");
    fs::rename(
        harness.source.root.join(&nested),
        harness.source.root.join(&moved),
    )
    .expect("rename nested sample");
    harness.commit_targeted_paths(vec![nested, moved.clone()], WatcherStimulus::Targeted);
    harness.await_fully_ready();

    fs::remove_file(harness.source.root.join(&moved)).expect("delete moved sample");
    harness.commit_targeted_paths(vec![moved], WatcherStimulus::Targeted);
    harness.await_fully_ready();

    for index in 0..4 {
        let relative = PathBuf::from(format!("storm/sample-{index}.wav"));
        fs::create_dir_all(
            harness
                .source
                .root
                .join(relative.parent().expect("storm parent")),
        )
        .expect("create storm folder");
        write_test_wav(&harness.source.root.join(&relative), index as f32 * 0.125);
    }
    harness.commit_overflow_audit(WatcherStimulus::Overflow);
    harness.await_fully_ready();

    harness.supervisor.set_playback_active(true);
    let playback_churn = harness.source.root.join("playback-churn.wav");
    write_test_wav(&playback_churn, 0.75);
    harness.commit_internal_mutation(
        FileMutationOperation::Duplicate,
        vec![FileMutationChange::created(playback_churn)],
    );
    let paused_snapshot = readiness_snapshot(&harness.source).expect("paused readiness snapshot");
    let paused_runtime = runtime_observation(&harness.supervisor, harness.source.id.as_str());
    assert!(paused_runtime.processing_paused);
    assert!(!silently_idle(&paused_snapshot, &paused_runtime));
    harness.supervisor.set_playback_active(false);
    harness.await_fully_ready();

    let report_before_restart = harness.shutdown();
    assert_eq!(report_before_restart["joined"], true);
    let kick_before_restart = fs::read(&kick).expect("read kick before closed-app change");
    write_test_wav(&kick, 1.5);
    assert_eq!(
        fs::metadata(&kick).expect("changed kick metadata").len(),
        kick_before_restart.len() as u64,
        "closed-app mutation should preserve file size"
    );
    let database = harness
        .source
        .open_db()
        .expect("open source before restart");
    harness.expected_source_generation = database
        .get_wav_paths_revision()
        .expect("read pre-restart source generation")
        .saturating_add(1) as i64;
    database
        .set_metadata(META_LAST_MANIFEST_AUDIT_AT, "0")
        .expect("make closed-app audit due");
    harness.supervisor = SourceProcessingSupervisor::start(vec![harness.source.clone()]);
    harness.watcher_stimulus = WatcherStimulus::ClosedAppAudit;
    harness.await_fully_ready();

    let offline_root = harness.source_parent.path().join("source-offline");
    fs::rename(&harness.source.root, &offline_root).expect("remove source root");
    harness.watcher_stimulus = WatcherStimulus::RootUnavailable;
    harness
        .supervisor
        .wake_source(harness.source.id.as_str(), "liveness_root_unavailable");
    harness.await_availability(SourceAvailability::Offline);

    fs::rename(&offline_root, &harness.source.root).expect("restore source root");
    harness.watcher_stimulus = WatcherStimulus::RootAvailable;
    harness
        .supervisor
        .wake_source(harness.source.id.as_str(), "liveness_root_available");
    harness.await_fully_ready();

    harness
        .supervisor
        .replace_sources(Vec::new())
        .expect("remove source during liveness lane");
    harness
        .supervisor
        .replace_sources(vec![harness.source.clone()])
        .expect("re-add retained source");
    harness.watcher_stimulus = WatcherStimulus::WatcherRestart;
    harness.await_fully_ready();

    let report = harness.shutdown();
    assert_eq!(report["joined"], true);
    assert!(report["completed"].as_u64().unwrap_or_default() > 0);
    assert_eq!(report["queue_depth"], 0);
    assert_eq!(report["readiness_queue_depth"], 0);
}

#[test]
#[ignore = "explicit calibrated large-library source-processing profile"]
fn profile_source_processing_churn_under_playback_and_browser_priority() {
    const FILE_COUNT: usize = 10_000;
    const DISCOVERY_BUDGET: Duration = Duration::from_secs(180);
    const MIN_MATERIALIZATION_THROUGHPUT: f64 = 200.0;
    const PRIORITY_P99_BUDGET: Duration = Duration::from_millis(10);
    const MEMORY_GROWTH_BUDGET_BYTES: u64 = 512 * 1024 * 1024;
    const CPU_CORE_EQUIVALENT_BUDGET: f64 = 2.0;
    const DISK_IO_BUDGET_BYTES: u64 = 1024 * 1024 * 1024;

    let directory = tempfile::tempdir().expect("large liveness profile source");
    let source = SampleSource::new_with_id(
        SourceId::from_string("source-processing-profile"),
        directory.path().to_path_buf(),
    );
    source.open_db().expect("create profile source database");
    let mut connection = open_connection(&source).expect("open profile database");
    seed_profile_manifest(&mut connection, FILE_COUNT);
    drop(connection);

    let resources_before = process_resource_snapshot();
    let mut supervisor =
        SourceProcessingSupervisor::start_with_playback_state(vec![source.clone()], true);
    let mut priority_samples = Vec::with_capacity(256);
    for index in 0..256 {
        let started = Instant::now();
        supervisor.set_selected_source(Some(source.id.as_str()));
        supervisor.set_current_folder(source.id.as_str(), "profile");
        supervisor.prioritize_path(
            source.id.as_str(),
            &format!("profile/sample-{:05}.wav", index % FILE_COUNT),
            index % 2 == 0,
        );
        priority_samples.push(started.elapsed());
    }
    priority_samples.sort_unstable();
    let priority_p99 = priority_samples[priority_samples.len() * 99 / 100];
    assert!(priority_p99 <= PRIORITY_P99_BUDGET);

    let started = Instant::now();
    let cancel = AtomicBool::new(false);
    let Cancellable::Completed((candidates, stats)) =
        discover_source_candidates(&source, now_epoch_seconds(), false, &cancel)
            .expect("discover large liveness source")
    else {
        panic!("large liveness discovery unexpectedly cancelled");
    };
    let discovery_elapsed = started.elapsed();
    let materialization_throughput =
        candidates.len() as f64 / discovery_elapsed.as_secs_f64().max(f64::EPSILON);
    assert!(discovery_elapsed <= DISCOVERY_BUDGET);
    assert!(materialization_throughput >= MIN_MATERIALIZATION_THROUGHPUT);
    assert_eq!(candidates.len(), FILE_COUNT * 4 + 1);
    assert_eq!(stats.readiness_queue_depth, FILE_COUNT * 4);
    assert_eq!(stats.prerequisites_blocked, FILE_COUNT * 3);

    let resources_after = process_resource_snapshot();
    let memory_growth = resources_after
        .memory_bytes
        .saturating_sub(resources_before.memory_bytes);
    let cpu_time_ms = resources_after
        .cpu_time_ms
        .saturating_sub(resources_before.cpu_time_ms);
    let cpu_core_equivalent =
        cpu_time_ms as f64 / discovery_elapsed.as_secs_f64().max(f64::EPSILON) / 1_000.0;
    let disk_read_bytes = resources_after
        .disk_read_bytes
        .saturating_sub(resources_before.disk_read_bytes);
    let disk_written_bytes = resources_after
        .disk_written_bytes
        .saturating_sub(resources_before.disk_written_bytes);
    assert!(memory_growth <= MEMORY_GROWTH_BUDGET_BYTES);
    assert!(cpu_core_equivalent <= CPU_CORE_EQUIVALENT_BUDGET);
    assert!(disk_read_bytes <= DISK_IO_BUDGET_BYTES);
    assert!(disk_written_bytes <= DISK_IO_BUDGET_BYTES);
    let report = supervisor.shutdown();
    assert_eq!(report["joined"], true);
    assert_eq!(report["claimed"], 0, "playback must keep work paused");
    assert_eq!(report["contention"], 0);

    eprintln!(
        "{}",
        serde_json::to_string_pretty(&serde_json::json!({
            "file_count": FILE_COUNT,
            "candidate_count": candidates.len(),
            "discovery_elapsed_ms": discovery_elapsed.as_secs_f64() * 1_000.0,
            "materialization_candidates_per_second": materialization_throughput,
            "priority_p99_us": priority_p99.as_micros(),
            "memory_growth_bytes": memory_growth,
            "cpu_time_ms": cpu_time_ms,
            "cpu_core_equivalent": cpu_core_equivalent,
            "disk_read_bytes": disk_read_bytes,
            "disk_written_bytes": disk_written_bytes,
            "budgets": {
                "discovery_elapsed_ms": DISCOVERY_BUDGET.as_millis(),
                "materialization_candidates_per_second": MIN_MATERIALIZATION_THROUGHPUT,
                "priority_p99_us": PRIORITY_P99_BUDGET.as_micros(),
                "memory_growth_bytes": MEMORY_GROWTH_BUDGET_BYTES,
                "cpu_core_equivalent": CPU_CORE_EQUIVALENT_BUDGET,
                "disk_read_bytes": DISK_IO_BUDGET_BYTES,
                "disk_written_bytes": DISK_IO_BUDGET_BYTES,
                "db_contention_events": 0,
            },
            "supervisor": report,
        }))
        .expect("serialize source-processing profile")
    );
}
