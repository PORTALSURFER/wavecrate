#[test]
fn identical_source_refresh_is_a_noop_and_descriptor_changes_are_source_local() {
    let (_first_directory, first) = unhashed_source("refresh-first");
    let (_second_directory, second) = unhashed_source("refresh-second");
    let mut supervisor = SourceProcessingSupervisor::dormant();
    supervisor
        .replace_sources(vec![first.clone(), second.clone()])
        .expect("configure sources");
    let (first_generation, second_generation, wake_generation) = {
        let mut control = supervisor.shared.control();
        control.dirty_sources.clear();
        control
            .priority
            .immediate_paths
            .insert((first.id.to_string(), "first.wav".to_string()));
        control
            .priority
            .immediate_paths
            .insert((second.id.to_string(), "second.wav".to_string()));
        control.priority.selected_source = Some(second.id.to_string());
        (
            Arc::clone(&control.source_work_cancels[first.id.as_str()]),
            Arc::clone(&control.source_work_cancels[second.id.as_str()]),
            control.wake_generation,
        )
    };

    supervisor
        .replace_sources(vec![first.clone(), second.clone()])
        .expect("refresh identical sources");

    {
        let control = supervisor.shared.control();
        assert_eq!(control.wake_generation, wake_generation);
        assert!(control.dirty_sources.is_empty());
        assert!(Arc::ptr_eq(
            &first_generation,
            &control.source_work_cancels[first.id.as_str()]
        ));
        assert!(Arc::ptr_eq(
            &second_generation,
            &control.source_work_cancels[second.id.as_str()]
        ));
    }

    let replacement_directory = tempfile::tempdir().expect("replacement source root");
    let replacement =
        SampleSource::new_with_id(first.id.clone(), replacement_directory.path().to_path_buf());
    supervisor
        .replace_sources(vec![replacement, second.clone()])
        .expect("replace changed source");

    assert!(first_generation.load(Ordering::Acquire));
    assert!(!second_generation.load(Ordering::Acquire));
    let control = supervisor.shared.control();
    assert_eq!(
        control.dirty_sources,
        BTreeSet::from([first.id.to_string()])
    );
    assert!(Arc::ptr_eq(
        &second_generation,
        &control.source_work_cancels[second.id.as_str()]
    ));
    assert_eq!(
        control.priority.immediate_paths,
        BTreeSet::from([(second.id.to_string(), "second.wav".to_string())])
    );
    assert_eq!(
        control.priority.selected_source.as_deref(),
        Some(second.id.as_str())
    );
    drop(control);
    assert_eq!(supervisor.shutdown()["joined"], true);
}

#[test]
fn priority_only_wakes_reuse_candidates_without_source_rediscovery() {
    let directory = tempfile::tempdir().expect("priority source");
    let source = SampleSource::new_with_id(
        SourceId::from_string(format!("priority-cache-{}", uuid::Uuid::new_v4())),
        directory.path().to_path_buf(),
    );
    source.open_db().expect("create priority source database");
    let mut supervisor = SourceProcessingSupervisor::start(vec![source.clone()]);
    // The empty-source fixture converges through startup, manifest-audit, and readiness
    // handoffs. Do not capture the baseline at a transient queue-empty boundary between them.
    wait_until(Duration::from_secs(5), || {
        let telemetry = supervisor.shared.telemetry();
        let completed = telemetry.completed;
        let queue_depth = telemetry.queue_depth;
        let settled_wake_generation = telemetry.settled_wake_generation;
        drop(telemetry);
        let control = supervisor.shared.control();
        completed >= 2
            && queue_depth == 0
            && settled_wake_generation == control.wake_generation
            && control.dirty_sources.is_empty()
    });
    let discoveries_before = supervisor.shared.telemetry().source_discoveries;

    for index in 0..128 {
        supervisor.prioritize_path(
            source.id.as_str(),
            format!("visible/sample-{index}.wav").as_str(),
            true,
        );
    }
    thread::sleep(Duration::from_millis(150));

    assert_eq!(
        supervisor.shared.telemetry().source_discoveries,
        discoveries_before,
        "priority-only wakes must reschedule the retained batch without database discovery"
    );
    assert_eq!(supervisor.shutdown()["joined"], true);
}

#[test]
fn playback_and_foreground_resumes_reuse_the_retained_source_snapshot() {
    let directory = tempfile::tempdir().expect("resume source");
    let source = SampleSource::new_with_id(
        SourceId::from_string(format!("resume-cache-{}", uuid::Uuid::new_v4())),
        directory.path().to_path_buf(),
    );
    source.open_db().expect("create resume source database");
    let mut supervisor = SourceProcessingSupervisor::start(vec![source]);
    // The empty-source fixture converges through startup, manifest-audit, and readiness
    // handoffs. Do not capture the baseline at a transient queue-empty boundary between them.
    wait_until(Duration::from_secs(5), || {
        let telemetry = supervisor.shared.telemetry();
        let completed = telemetry.completed;
        let queue_depth = telemetry.queue_depth;
        let settled_wake_generation = telemetry.settled_wake_generation;
        drop(telemetry);
        let control = supervisor.shared.control();
        completed >= 2
            && queue_depth == 0
            && settled_wake_generation == control.wake_generation
            && control.dirty_sources.is_empty()
    });
    let discoveries_before = supervisor.shared.telemetry().source_discoveries;

    for _ in 0..64 {
        supervisor.set_playback_active(true);
        supervisor.set_playback_active(false);
        supervisor.set_foreground_activity(true);
        supervisor.set_foreground_activity(false);
    }
    thread::sleep(Duration::from_millis(150));

    assert_eq!(
        supervisor.shared.telemetry().source_discoveries,
        discoveries_before,
        "resume must reuse retained candidates instead of restarting manifest discovery"
    );
    assert_eq!(supervisor.shutdown()["joined"], true);
}

#[test]
fn embedding_readiness_rejects_rows_from_the_previous_content_generation() {
    let (_directory, source) = ready_analysis_source("embedding-generation");
    let database_root = source.database_root().expect("database root");
    let connection = SourceDatabase::open_connection_with_role_and_database_root(
        &source.root,
        &database_root,
        SourceDatabaseConnectionRole::JobWorker,
    )
    .expect("open embedding database");
    let sample_id = format!("{}::ready.wav", source.id);
    let embedding_dim = wavecrate_analysis::similarity::SIMILARITY_DIM as i64;
    let aspect_dim = wavecrate_analysis::aspects::ASPECT_DESCRIPTOR_DIM as i64;
    let embedding_a = vec![1_u8, 2, 3, 4];
    let embedding_b = vec![5_u8, 6, 7, 8];
    let aspects_a = vec![9_u8, 10, 11, 12];
    let aspects_b = vec![13_u8, 14, 15, 16];
    connection
        .execute(
            "INSERT INTO samples (sample_id, content_hash, size, mtime_ns)
                 VALUES (?1, 'content-a', 1, 1)",
            [&sample_id],
        )
        .expect("insert sample");
    for (content_hash, embedding, aspects) in [
        ("content-a", embedding_a.as_slice(), aspects_a.as_slice()),
        ("content-b", embedding_b.as_slice(), aspects_b.as_slice()),
    ] {
        connection
            .execute(
                "INSERT INTO analysis_cache_embeddings (
                        content_hash, analysis_version, model_id, dim, dtype,
                        l2_normed, vec, created_at
                     ) VALUES (?1, ?2, ?3, ?4, ?5, 1, ?6, 1)",
                params![
                    content_hash,
                    wavecrate_analysis::analysis_version(),
                    wavecrate_analysis::similarity::SIMILARITY_MODEL_ID,
                    embedding_dim,
                    wavecrate_analysis::similarity::SIMILARITY_DTYPE_F32,
                    embedding,
                ],
            )
            .expect("insert cached embedding");
        connection
            .execute(
                "INSERT INTO analysis_cache_aspect_descriptors (
                        content_hash, analysis_version, model_id, dim, dtype,
                        l2_normed, valid_mask, vec, created_at
                     ) VALUES (?1, ?2, ?3, ?4, ?5, 1, 7, ?6, 1)",
                params![
                    content_hash,
                    wavecrate_analysis::analysis_version(),
                    wavecrate_analysis::aspects::ASPECT_DESCRIPTOR_MODEL_ID,
                    aspect_dim,
                    wavecrate_analysis::aspects::ASPECT_DESCRIPTOR_DTYPE_F32,
                    aspects,
                ],
            )
            .expect("insert cached aspects");
    }
    connection
        .execute(
            "INSERT INTO embeddings (
                    sample_id, model_id, dim, dtype, l2_normed, vec, created_at
                 ) VALUES (?1, ?2, ?3, ?4, 1, ?5, 1)",
            params![
                sample_id,
                wavecrate_analysis::similarity::SIMILARITY_MODEL_ID,
                embedding_dim,
                wavecrate_analysis::similarity::SIMILARITY_DTYPE_F32,
                embedding_a,
            ],
        )
        .expect("insert materialized embedding");
    connection
        .execute(
            "INSERT INTO similarity_aspect_descriptors (
                    sample_id, model_id, dim, dtype, l2_normed, valid_mask, vec, created_at
                 ) VALUES (?1, ?2, ?3, ?4, 1, 7, ?5, 1)",
            params![
                sample_id,
                wavecrate_analysis::aspects::ASPECT_DESCRIPTOR_MODEL_ID,
                aspect_dim,
                wavecrate_analysis::aspects::ASPECT_DESCRIPTOR_DTYPE_F32,
                aspects_a,
            ],
        )
        .expect("insert materialized aspects");
    connection
        .execute(
            "UPDATE samples SET content_hash = 'content-b' WHERE sample_id = ?1",
            [&sample_id],
        )
        .expect("advance sample content");
    let target = ReadinessTarget::file(
        source.id.as_str(),
        "identity-1",
        "ready.wav",
        ReadinessStage::EmbeddingAspects,
        "embedding-v1",
        1,
        "content-b",
    );

    assert!(!embedding_aspects_are_current(&connection, &target).unwrap());

    connection
        .execute(
            "UPDATE embeddings SET vec = ?2 WHERE sample_id = ?1",
            params![sample_id, embedding_b],
        )
        .expect("materialize current embedding");
    connection
        .execute(
            "UPDATE similarity_aspect_descriptors SET vec = ?2 WHERE sample_id = ?1",
            params![sample_id, aspects_b],
        )
        .expect("materialize current aspects");
    assert!(embedding_aspects_are_current(&connection, &target).unwrap());
}

#[test]
fn source_removal_returns_immediately_and_retires_after_exact_epoch_drains() {
    let (_directory, source) = unhashed_source("retired-fence");
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
                 SET file_identity = 'retired-identity', content_hash = 'retired-content'
                 WHERE path = 'pending.wav'",
            [],
        )
        .expect("assign readiness identity");
    publish_current_readiness_targets(&mut connection, source.id.as_str(), 1)
        .expect("publish readiness targets");
    drop(connection);

    let mut supervisor = SourceProcessingSupervisor::dormant();
    supervisor
        .replace_sources(vec![source.clone()])
        .expect("configure source");
    let generation = {
        let control = supervisor.shared.control();
        Arc::clone(&control.source_work_cancels[source.id.as_str()])
    };
    let in_flight = supervisor
        .shared
        .begin_in_flight_work(source.id.as_str(), &generation)
        .expect("register in-flight source publication");
    let started = Instant::now();
    supervisor
        .replace_sources(Vec::new())
        .expect("remove configured source");
    assert!(started.elapsed() < Duration::from_millis(50));
    process_ready_source_retirements(&supervisor.shared);
    let active_before_drain: String = SourceDatabase::open_connection_with_role_and_database_root(
        &source.root,
        &database_root,
        SourceDatabaseConnectionRole::JobWorker,
    )
    .expect("reopen readiness before old work drains")
    .query_row(
        "SELECT availability FROM source_readiness_sources WHERE source_id = ?1",
        [source.id.as_str()],
        |row| row.get(0),
    )
    .expect("read readiness before old work drains");
    assert_eq!(active_before_drain, "active");
    drop(in_flight);
    process_ready_source_retirements(&supervisor.shared);

    let connection = SourceDatabase::open_connection_with_role_and_database_root(
        &source.root,
        &database_root,
        SourceDatabaseConnectionRole::JobWorker,
    )
    .expect("reopen readiness database");
    let availability: String = connection
        .query_row(
            "SELECT availability FROM source_readiness_sources WHERE source_id = ?1",
            [source.id.as_str()],
            |row| row.get(0),
        )
        .expect("read retired source readiness");
    assert_eq!(availability, "disabled");
    assert_eq!(supervisor.shutdown()["joined"], true);
}
