#[test]
fn sustained_manifest_audit_activates_source_row_after_grace_period() {
    assert!(
        !manifest_audit_source_row_active(Instant::now()),
        "brief manifest maintenance must not flash the source row"
    );
    assert!(
        manifest_audit_source_row_active(Instant::now() - DISCOVERY_PROGRESS_EVENT_GRACE_INTERVAL),
        "a sustained manifest audit must identify its active source row"
    );
}

#[test]
fn production_supervisor_publishes_claims_and_completes_readiness_without_manual_seed() {
    let (_directory, source) = ready_analysis_source("readiness");

    let mut supervisor = SourceProcessingSupervisor::start(vec![source.clone()]);
    wait_until(Duration::from_secs(20), || {
        let database_root = source.database_root().expect("database root");
        let Ok(connection) = SourceDatabase::open_connection_with_role_and_database_root(
            &source.root,
            &database_root,
            SourceDatabaseConnectionRole::JobWorker,
        ) else {
            return false;
        };
        connection
            .query_row(
                "SELECT COUNT(*) = 4
                        FROM source_readiness_sources AS source
                        JOIN source_readiness_targets AS target
                          ON target.source_id = source.source_id
                        JOIN source_readiness_artifacts AS artifact
                          ON artifact.source_id = target.source_id
                         AND artifact.scope_kind = target.scope_kind
                         AND artifact.scope_id = target.scope_id
                         AND artifact.stage = target.stage
                        WHERE source.source_id = ?1
                          AND source.availability = 'active'
                          AND artifact.artifact_version = target.required_version
                          AND artifact.content_generation = target.content_generation
                          AND target.stage != 'playback_summary'
                          AND (
                              target.scope_kind = 'file'
                              OR artifact.source_generation = target.source_generation
                          )
                          AND EXISTS (
                              SELECT 1 FROM layout_umap
                              WHERE sample_id = ?1 || '::ready.wav'
                          )
                          AND EXISTS (
                              SELECT 1 FROM hdbscan_clusters
                              WHERE sample_id = ?1 || '::ready.wav'
                          )
                          AND EXISTS (
                              SELECT 1 FROM ann_index_meta WHERE count = 1
                          )
                          AND EXISTS (
                              SELECT 1 FROM metadata
                              WHERE key = 'similarity_artifact_state_v1'
                                AND json_extract(value, '$.state') = 'current'
                                AND json_extract(value, '$.artifact_contract_version') = ?2
                          )",
                params![source.id.as_str(), native_similarity_artifact_version()],
                |row| row.get::<_, bool>(0),
            )
            .unwrap_or(false)
    });
    let report = supervisor.shutdown();
    assert_eq!(report["joined"], true);
    assert!(report["claimed"].as_u64().unwrap_or_default() >= 1);
    assert!(report["completed"].as_u64().unwrap_or_default() >= 4);
    let database_root = source.database_root().expect("database root");
    let connection = SourceDatabase::open_connection_with_role_and_database_root(
        &source.root,
        &database_root,
        SourceDatabaseConnectionRole::JobWorker,
    )
    .expect("open converged readiness database");
    let playback_rows = connection
        .query_row(
            "SELECT
                    (SELECT COUNT(*) FROM source_readiness_targets
                     WHERE source_id = ?1 AND stage = 'playback_summary')
                  + (SELECT COUNT(*) FROM source_readiness_artifacts
                     WHERE source_id = ?1 AND stage = 'playback_summary')",
            [source.id.as_str()],
            |row| row.get::<_, i64>(0),
        )
        .expect("count playback readiness rows");
    assert_eq!(
        playback_rows, 0,
        "source convergence must not create persistent playback work"
    );
}

#[test]
fn legacy_playback_readiness_is_retired_without_requeueing_source_work() {
    let (_directory, source) = ready_analysis_source("legacy-playback-retirement");
    let database_root = source.database_root().expect("database root");
    let (cache_ref, _) = seed_legacy_playback_artifact(&source);

    let mut connection = SourceDatabase::open_connection_with_role_and_database_root(
        &source.root,
        &database_root,
        SourceDatabaseConnectionRole::JobWorker,
    )
    .expect("reopen source with legacy playback rows");

    let snapshot = reconcile_readiness_with_cancel_and_progress(
        &connection,
        source.id.as_str(),
        now_epoch_seconds(),
        &AtomicBool::new(false),
        &mut || {},
    )
    .expect("ignore legacy playback rows during reconciliation");
    assert_eq!(snapshot.entries.len(), 4);

    assert!(matches!(
        retire_legacy_playback_readiness(&source, &mut connection, &AtomicBool::new(false))
            .expect("retire legacy playback readiness"),
        Cancellable::Completed(2)
    ));
    let playback_rows = connection
        .query_row(
            "SELECT
                    (SELECT COUNT(*) FROM source_readiness_targets
                     WHERE source_id = ?1 AND stage = 'playback_summary')
                  + (SELECT COUNT(*) FROM source_readiness_artifacts
                     WHERE source_id = ?1 AND stage = 'playback_summary')
                  + (SELECT COUNT(*) FROM analysis_jobs
                     WHERE source_id = ?1
                       AND readiness_managed = 1
                       AND readiness_stage = 'playback_summary')",
            [source.id.as_str()],
            |row| row.get::<_, i64>(0),
        )
        .expect("count retired playback rows");
    assert_eq!(playback_rows, 0);
    assert!(!cache_ref.exists());
    assert!(matches!(
        retire_legacy_playback_readiness(&source, &mut connection, &AtomicBool::new(false))
            .expect("repeat legacy retirement"),
        Cancellable::Completed(0)
    ));
}

#[test]
fn post_commit_cancellation_still_retires_every_legacy_cache_ref() {
    let (_directory, source) = ready_analysis_source("legacy-playback-cancellation");
    let database_root = source.database_root().expect("database root");
    let (first_cache_ref, now) = seed_legacy_playback_artifact(&source);
    let second_cache_ref = seed_managed_legacy_cache_ref(&source, "second", now);
    let mut connection = SourceDatabase::open_connection_with_role_and_database_root(
        &source.root,
        &database_root,
        SourceDatabaseConnectionRole::JobWorker,
    )
    .expect("reopen source with multiple legacy playback rows");
    connection
        .execute(
            "INSERT INTO source_readiness_targets (
                    source_id, scope_kind, scope_id, relative_path, stage, required_version,
                    source_generation, content_generation, eligibility, updated_at
                 )
                 SELECT source_id, scope_kind, 'legacy-second', 'legacy-second.wav', stage,
                        required_version, source_generation, content_generation, eligibility,
                        updated_at
                 FROM source_readiness_targets
                 WHERE source_id = ?1 AND stage = 'playback_summary'",
            [source.id.as_str()],
        )
        .expect("seed second legacy playback target");
    connection
        .execute(
            "INSERT INTO source_readiness_artifacts (
                    source_id, scope_kind, scope_id, relative_path, stage, artifact_version,
                    source_generation, content_generation, artifact_ref, completed_at
                 )
                 SELECT source_id, scope_kind, scope_id, relative_path, stage, required_version,
                        source_generation, content_generation, ?2, ?3
                 FROM source_readiness_targets
                 WHERE source_id = ?1
                   AND scope_id = 'legacy-second'
                   AND stage = 'playback_summary'",
            params![source.id.as_str(), second_cache_ref.to_string_lossy(), now],
        )
        .expect("seed second legacy playback artifact");

    let cancel = AtomicBool::new(false);
    assert!(matches!(
        retire_legacy_playback_readiness_with_post_commit_hook(
            &source,
            &mut connection,
            &cancel,
            || cancel.store(true, Ordering::Release),
        )
        .expect("retire every captured legacy playback reference"),
        Cancellable::Completed(4)
    ));
    assert!(cancelled(&cancel));
    assert!(!first_cache_ref.exists());
    assert!(!second_cache_ref.exists());
}

#[test]
fn legacy_playback_cache_owner_is_retired_after_committed_delete() {
    let (_directory, source) = ready_analysis_source("playback-delete");
    let database_root = source.database_root().expect("database root");
    let (owned_cache_ref, _) = seed_legacy_playback_artifact(&source);

    std::fs::remove_file(source.root.join("ready.wav")).expect("delete source sample");
    let db = source.open_db().expect("open source after delete");
    wavecrate::sample_sources::scanner::sync_paths(&db, &[PathBuf::from("ready.wav")])
        .expect("commit source deletion");
    let mut supervisor = SourceProcessingSupervisor::start(vec![source.clone()]);
    supervisor.wake_source(source.id.as_str(), "test_committed_delete");

    wait_until(Duration::from_secs(10), || {
        let Ok(connection) = SourceDatabase::open_connection_with_role_and_database_root(
            &source.root,
            &database_root,
            SourceDatabaseConnectionRole::JobWorker,
        ) else {
            return false;
        };
        let ownership_removed = connection
            .query_row(
                "SELECT COUNT(*) = 0
                     FROM source_readiness_artifacts
                     WHERE source_id = ?1 AND stage = 'playback_summary'",
                [source.id.as_str()],
                |row| row.get::<_, bool>(0),
            )
            .unwrap_or(false);
        ownership_removed && !owned_cache_ref.exists()
    });
    let report = supervisor.shutdown();
    assert_eq!(report["joined"], true);
}

#[test]
fn stale_analysis_hash_triggers_targeted_reconciliation_and_converges() {
    let (_directory, source) = ready_analysis_source("stale-analysis-input");
    let relative = Path::new("ready.wav");
    let db = source.open_db().expect("open stale analysis source");
    wavecrate::sample_sources::scanner::sync_paths(&db, &[relative.to_path_buf()])
        .expect("normalize source manifest");
    db.set_metadata(
        META_LAST_MANIFEST_AUDIT_AT,
        &now_epoch_seconds().to_string(),
    )
    .expect("defer periodic audit");

    let path = source.root.join(relative);
    let original_modified = std::fs::metadata(&path)
        .expect("read original metadata")
        .modified()
        .expect("read original modified time");
    let mut bytes = std::fs::read(&path).expect("read readiness wav");
    let last = bytes.last_mut().expect("readiness wav has audio data");
    *last ^= 0x01;
    std::fs::write(&path, &bytes).expect("mutate readiness wav");
    let file = std::fs::OpenOptions::new()
        .write(true)
        .open(&path)
        .expect("reopen mutated readiness wav");
    file.set_times(std::fs::FileTimes::new().set_modified(original_modified))
        .expect("restore readiness modified time");
    let current_hash = blake3::hash(&bytes).to_hex().to_string();

    let mut supervisor = SourceProcessingSupervisor::start(vec![source.clone()]);
    wait_until(Duration::from_secs(15), || {
        let manifest_is_current = source
            .open_db()
            .ok()
            .and_then(|db| db.entry_for_path(relative).ok().flatten())
            .and_then(|entry| entry.content_hash)
            .as_deref()
            == Some(current_hash.as_str());
        if !manifest_is_current {
            return false;
        }
        let database_root = source.database_root().expect("database root");
        let Ok(connection) = SourceDatabase::open_connection_with_role_and_database_root(
            &source.root,
            &database_root,
            SourceDatabaseConnectionRole::JobWorker,
        ) else {
            return false;
        };
        let sample_id = format!("{}::ready.wav", source.id);
        connection
            .query_row(
                "SELECT EXISTS(
                        SELECT 1
                        FROM samples AS sample
                        JOIN features AS feature ON feature.sample_id = sample.sample_id
                        JOIN embeddings AS embedding ON embedding.sample_id = sample.sample_id
                        JOIN similarity_aspect_descriptors AS aspects
                          ON aspects.sample_id = sample.sample_id
                        WHERE sample.sample_id = ?1
                          AND sample.content_hash = ?2
                    )",
                params![sample_id, current_hash],
                |row| row.get::<_, bool>(0),
            )
            .unwrap_or(false)
    });
    assert_eq!(supervisor.shutdown()["joined"], true);
}
