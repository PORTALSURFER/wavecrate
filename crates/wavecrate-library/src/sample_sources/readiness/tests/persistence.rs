use super::*;

#[test]
fn stale_completion_cannot_overwrite_a_new_generation() {
    let (_root, mut connection) = open_fixture();
    let generation_one = file_target("changed", ReadinessStage::AnalysisFeatures, 1);
    replace(&mut connection, 1, std::slice::from_ref(&generation_one));
    let stale_completion = ReadinessArtifact::for_target(&generation_one, 10);
    let generation_two = file_target("changed", ReadinessStage::AnalysisFeatures, 2);
    replace(&mut connection, 2, std::slice::from_ref(&generation_two));

    assert_eq!(
        publish_readiness_artifact(&mut connection, &stale_completion)
            .expect("reject stale completion"),
        ArtifactPublishOutcome::RejectedStale
    );
    assert_eq!(
        publish_readiness_artifact(
            &mut connection,
            &ReadinessArtifact::for_target(&generation_two, 11),
        )
        .expect("publish current completion"),
        ArtifactPublishOutcome::Recorded
    );
    let snapshot = reconcile_readiness(&connection, SOURCE_ID, 12).expect("snapshot");
    assert_eq!(
        snapshot.entries[0].classification,
        ReadinessClassification::Current
    );
}

#[test]
fn target_replacement_is_failure_atomic() {
    let (_root, mut connection) = open_fixture();
    let original = file_target("original", ReadinessStage::IndexedIdentity, 1);
    replace(&mut connection, 1, std::slice::from_ref(&original));
    connection
        .execute_batch(
            "CREATE TRIGGER reject_broken_readiness_target
             BEFORE INSERT ON source_readiness_targets
             WHEN NEW.scope_id = 'broken'
             BEGIN
                 SELECT RAISE(ABORT, 'injected readiness write failure');
             END;",
        )
        .expect("create failure trigger");
    let broken = file_target("broken", ReadinessStage::IndexedIdentity, 2);

    assert!(
        replace_readiness_targets(
            &mut connection,
            SOURCE_ID,
            2,
            SourceAvailability::Active,
            &[broken],
            2,
        )
        .is_err()
    );

    let snapshot = reconcile_readiness(&connection, SOURCE_ID, 3).expect("rolled back snapshot");
    assert_eq!(snapshot.source_generation, 1);
    assert_eq!(snapshot.entries.len(), 1);
    assert_eq!(snapshot.entries[0].target.scope_id, "original");
}

#[test]
fn read_only_reconciliation_never_enqueues_work() {
    let (root, mut connection) = open_fixture();
    let target = file_target("read-only", ReadinessStage::PlaybackSummary, 1);
    replace(&mut connection, 1, &[target]);
    drop(connection);
    let read_only = SourceDatabase::open_connection_with_role(
        root.path(),
        SourceDatabaseConnectionRole::UiRead,
    )
    .expect("read-only source db");

    let snapshot = reconcile_readiness(&read_only, SOURCE_ID, 2).expect("read snapshot");
    assert_eq!(snapshot.deficits.len(), 1);
    let jobs: i64 = read_only
        .query_row("SELECT COUNT(*) FROM analysis_jobs", [], |row| row.get(0))
        .expect("count jobs");
    assert_eq!(jobs, 0);
}

#[test]
fn legacy_read_only_source_reports_unavailable_readiness_schema() {
    let root = tempfile::tempdir().expect("source root");
    let database_path = root.path().join(crate::sample_sources::db::DB_FILE_NAME);
    let legacy = Connection::open(&database_path).expect("legacy source db");
    legacy
        .execute_batch(
            "CREATE TABLE wav_files (
                path TEXT PRIMARY KEY,
                file_size INTEGER NOT NULL,
                modified_ns INTEGER NOT NULL
            );",
        )
        .expect("legacy schema");
    drop(legacy);
    let read_only = SourceDatabase::open_connection_with_role(
        root.path(),
        SourceDatabaseConnectionRole::UiRead,
    )
    .expect("legacy read-only source db");

    let error = reconcile_readiness(&read_only, SOURCE_ID, 1).expect_err("schema unavailable");

    assert!(matches!(error, ReadinessError::SchemaUnavailable));
}
