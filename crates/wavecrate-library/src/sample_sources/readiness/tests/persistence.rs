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
        entry_for(&snapshot, "changed", ReadinessStage::AnalysisFeatures).classification,
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
    let broken_targets = complete_targets(2, std::slice::from_ref(&broken));

    assert!(
        replace_readiness_targets(
            &mut connection,
            SOURCE_ID,
            2,
            2,
            SourceAvailability::Active,
            &broken_targets,
            2,
        )
        .is_err()
    );

    let snapshot = reconcile_readiness(&connection, SOURCE_ID, 3).expect("rolled back snapshot");
    assert_eq!(snapshot.source_generation, 1);
    assert_eq!(snapshot.entries.len(), 5);
    assert!(snapshot.entries.iter().all(|entry| {
        entry.target.scope_id == "original" || entry.target.scope_kind == ReadinessScopeKind::Source
    }));
}

#[test]
fn stale_same_generation_publication_cannot_reactivate_disabled_source() {
    let (_root, mut connection) = open_fixture();
    let target = file_target("guarded", ReadinessStage::PlaybackSummary, 1);
    let complete = complete_targets(1, std::slice::from_ref(&target));
    replace_readiness_targets(
        &mut connection,
        SOURCE_ID,
        1,
        1,
        SourceAvailability::Active,
        &complete,
        10,
    )
    .expect("publish active state");
    replace_readiness_targets(
        &mut connection,
        SOURCE_ID,
        1,
        2,
        SourceAvailability::Disabled,
        &complete,
        11,
    )
    .expect("disable source");

    let error = replace_readiness_targets(
        &mut connection,
        SOURCE_ID,
        1,
        1,
        SourceAvailability::Active,
        &complete,
        12,
    )
    .expect_err("reject stale active publication");
    assert!(matches!(
        error,
        ReadinessError::StaleReadinessRevision {
            attempted: 1,
            current: 2,
            ..
        }
    ));

    let snapshot = reconcile_readiness(&connection, SOURCE_ID, 13).expect("disabled snapshot");
    assert_eq!(snapshot.readiness_revision, 2);
    assert_eq!(snapshot.availability, SourceAvailability::Disabled);
    assert!(snapshot.deficits.is_empty());
}

#[test]
fn empty_content_generations_are_rejected_before_persistence() {
    let (_root, mut connection) = open_fixture();
    let mut invalid_target = file_target("invalid", ReadinessStage::AnalysisFeatures, 1);
    invalid_target.content_generation.clear();
    let error = replace_readiness_targets(
        &mut connection,
        SOURCE_ID,
        1,
        1,
        SourceAvailability::Active,
        &[invalid_target],
        10,
    )
    .expect_err("reject empty target generation");
    assert!(matches!(
        error,
        ReadinessError::InvalidContentGeneration { .. }
    ));
    assert!(
        connection
            .execute(
                "INSERT INTO source_readiness_targets (
                    source_id, scope_kind, scope_id, relative_path, stage, required_version,
                    source_generation, content_generation, eligibility, updated_at
                 ) VALUES (?1, 'file', 'raw-invalid', 'raw.wav', 'analysis_features',
                           'v1', 1, NULL, 'eligible', 10)",
                [SOURCE_ID],
            )
            .is_err(),
        "schema must reject a NULL readiness generation"
    );

    let target = file_target("valid", ReadinessStage::AnalysisFeatures, 1);
    let complete = complete_targets(1, std::slice::from_ref(&target));
    replace_readiness_targets(
        &mut connection,
        SOURCE_ID,
        1,
        1,
        SourceAvailability::Active,
        &complete,
        11,
    )
    .expect("publish valid target");
    let mut invalid_artifact = ReadinessArtifact::for_target(&target, 12);
    invalid_artifact.content_generation.clear();
    let error = publish_readiness_artifact(&mut connection, &invalid_artifact)
        .expect_err("reject empty artifact generation");
    assert!(matches!(
        error,
        ReadinessError::InvalidContentGeneration { .. }
    ));
}

#[test]
fn empty_artifact_versions_are_rejected_before_persistence() {
    let (_root, mut connection) = open_fixture();
    let mut invalid_target = file_target("invalid-version", ReadinessStage::AnalysisFeatures, 1);
    invalid_target.required_version = "  ".to_string();
    let error = replace_readiness_targets(
        &mut connection,
        SOURCE_ID,
        1,
        1,
        SourceAvailability::Active,
        &[invalid_target],
        10,
    )
    .expect_err("reject empty target version");
    assert!(matches!(
        error,
        ReadinessError::InvalidArtifactVersion { .. }
    ));

    assert!(
        connection
            .execute(
                "INSERT INTO source_readiness_targets (
                    source_id, scope_kind, scope_id, relative_path, stage, required_version,
                    source_generation, content_generation, eligibility, updated_at
                 ) VALUES (?1, 'file', 'raw-empty-version', 'raw.wav', 'analysis_features',
                           ' ', 1, 'content-v1', 'eligible', 11)",
                [SOURCE_ID],
            )
            .is_err(),
        "schema must reject an empty target version"
    );

    let target = file_target("valid-version", ReadinessStage::AnalysisFeatures, 1);
    let complete = complete_targets(1, std::slice::from_ref(&target));
    replace_readiness_targets(
        &mut connection,
        SOURCE_ID,
        1,
        2,
        SourceAvailability::Active,
        &complete,
        12,
    )
    .expect("publish valid target");
    let mut invalid_artifact = ReadinessArtifact::for_target(&target, 13);
    invalid_artifact.artifact_version.clear();
    let error = publish_readiness_artifact(&mut connection, &invalid_artifact)
        .expect_err("reject empty artifact version");
    assert!(matches!(
        error,
        ReadinessError::InvalidArtifactVersion { .. }
    ));

    assert!(
        connection
            .execute(
                "INSERT INTO source_readiness_artifacts (
                    source_id, scope_kind, scope_id, stage, artifact_version,
                    source_generation, content_generation, completed_at
                 ) VALUES (?1, 'file', 'raw-empty-version', 'analysis_features',
                           '', 1, 'content-v1', 14)",
                [SOURCE_ID],
            )
            .is_err(),
        "schema must reject an empty artifact version"
    );
}

#[test]
fn invalid_stage_scope_pairings_are_rejected() {
    let (_root, mut connection) = open_fixture();
    let invalid_file = file_target("layout", ReadinessStage::SimilarityLayout, 1);
    let error = replace_readiness_targets(
        &mut connection,
        SOURCE_ID,
        1,
        1,
        SourceAvailability::Active,
        &[invalid_file],
        10,
    )
    .expect_err("reject file-scoped layout");
    assert!(matches!(error, ReadinessError::InvalidStageScope { .. }));

    let invalid_source = ReadinessTarget::source(
        SOURCE_ID,
        ReadinessStage::PlaybackSummary,
        "v1",
        1,
        "membership-v1",
    );
    let error = replace_readiness_targets(
        &mut connection,
        SOURCE_ID,
        1,
        1,
        SourceAvailability::Active,
        &[invalid_source],
        11,
    )
    .expect_err("reject source-scoped file stage");
    assert!(matches!(error, ReadinessError::InvalidStageScope { .. }));

    assert!(
        connection
            .execute(
                "INSERT INTO source_readiness_targets (
                    source_id, scope_kind, scope_id, relative_path, stage, required_version,
                    source_generation, content_generation, eligibility, updated_at
                 ) VALUES (?1, 'file', 'raw-layout', 'raw.wav', 'similarity_layout',
                           'v1', 1, 'content-v1', 'eligible', 12)",
                [SOURCE_ID],
            )
            .is_err(),
        "schema must reject an invalid stage/scope pairing"
    );
}

#[test]
fn invalid_target_identities_and_paths_are_rejected() {
    let (_root, mut connection) = open_fixture();
    let mut invalid_source = ReadinessTarget::source(
        SOURCE_ID,
        ReadinessStage::SimilarityLayout,
        "v1",
        1,
        "membership-v1",
    );
    invalid_source.scope_id = "another-source".to_string();
    let error = replace_readiness_targets(
        &mut connection,
        SOURCE_ID,
        1,
        1,
        SourceAvailability::Active,
        &[invalid_source],
        10,
    )
    .expect_err("reject noncanonical source scope identity");
    assert!(matches!(error, ReadinessError::InvalidScopeIdentity { .. }));

    let mut invalid_path = file_target("missing-path", ReadinessStage::IndexedIdentity, 1);
    invalid_path.relative_path = Some("  ".to_string());
    let error = replace_readiness_targets(
        &mut connection,
        SOURCE_ID,
        1,
        1,
        SourceAvailability::Active,
        &[invalid_path],
        11,
    )
    .expect_err("reject empty eligible file path");
    assert!(matches!(error, ReadinessError::InvalidRelativePath { .. }));

    assert!(
        connection
            .execute(
                "INSERT INTO source_readiness_targets (
                    source_id, scope_kind, scope_id, relative_path, stage, required_version,
                    source_generation, content_generation, eligibility, updated_at
                 ) VALUES (?1, 'source', 'another-source', NULL, 'similarity_layout',
                           'v1', 1, 'membership-v1', 'eligible', 12)",
                [SOURCE_ID],
            )
            .is_err(),
        "schema must reject noncanonical source scope identity"
    );
    assert!(
        connection
            .execute(
                "INSERT INTO source_readiness_targets (
                    source_id, scope_kind, scope_id, relative_path, stage, required_version,
                    source_generation, content_generation, eligibility, updated_at
                 ) VALUES (?1, 'file', 'missing-path', '', 'indexed_identity',
                           'v1', 1, 'content-v1', 'eligible', 13)",
                [SOURCE_ID],
            )
            .is_err(),
        "schema must reject an empty eligible file path"
    );
}

#[test]
fn incomplete_target_matrices_are_rejected() {
    let (_root, mut connection) = open_fixture();
    let seed = file_target("matrix", ReadinessStage::IndexedIdentity, 1);
    let complete = complete_targets(1, std::slice::from_ref(&seed));

    let without_similarity = complete
        .iter()
        .filter(|target| target.stage != ReadinessStage::SimilarityLayout)
        .cloned()
        .collect::<Vec<_>>();
    let error = replace_readiness_targets(
        &mut connection,
        SOURCE_ID,
        1,
        1,
        SourceAvailability::Active,
        &without_similarity,
        10,
    )
    .expect_err("reject matrix without source layout");
    assert!(matches!(
        error,
        ReadinessError::IncompleteTargetMatrix {
            stage: ReadinessStage::SimilarityLayout,
            ..
        }
    ));

    let without_embedding = complete
        .iter()
        .filter(|target| {
            target.scope_kind != ReadinessScopeKind::File
                || target.stage != ReadinessStage::EmbeddingAspects
        })
        .cloned()
        .collect::<Vec<_>>();
    let error = replace_readiness_targets(
        &mut connection,
        SOURCE_ID,
        1,
        1,
        SourceAvailability::Active,
        &without_embedding,
        11,
    )
    .expect_err("reject incomplete file stage matrix");
    assert!(matches!(
        error,
        ReadinessError::IncompleteTargetMatrix {
            stage: ReadinessStage::EmbeddingAspects,
            ..
        }
    ));
}

#[test]
fn delayed_deficit_cannot_enqueue_after_disable_or_delete() {
    let (_root, mut connection) = open_fixture();
    let target = file_target("inactive", ReadinessStage::PlaybackSummary, 1);
    let complete = complete_targets(1, std::slice::from_ref(&target));
    replace_readiness_targets(
        &mut connection,
        SOURCE_ID,
        1,
        1,
        SourceAvailability::Active,
        &complete,
        10,
    )
    .expect("publish active target");
    let active_snapshot = reconcile_readiness(&connection, SOURCE_ID, 11).expect("active snapshot");

    replace_readiness_targets(
        &mut connection,
        SOURCE_ID,
        1,
        2,
        SourceAvailability::Disabled,
        &complete,
        12,
    )
    .expect("disable source");
    assert_eq!(
        persist_readiness_deficits(&mut connection, &active_snapshot.deficits, 13)
            .expect("ignore disabled deficit"),
        0
    );

    let deleted = target.with_eligibility(ReadinessEligibility::Deleted);
    replace_readiness_targets(
        &mut connection,
        SOURCE_ID,
        1,
        3,
        SourceAvailability::Active,
        &complete_targets(1, &[deleted]),
        14,
    )
    .expect("publish deleted target");
    assert_eq!(
        persist_readiness_deficits(&mut connection, &active_snapshot.deficits, 15)
            .expect("ignore deleted deficit"),
        0
    );
    let jobs: i64 = connection
        .query_row(
            "SELECT COUNT(*) FROM analysis_jobs WHERE readiness_managed = 1",
            [],
            |row| row.get(0),
        )
        .expect("count readiness work");
    assert_eq!(jobs, 0);
}

#[test]
fn delayed_deficit_cannot_recreate_completed_work() {
    let (_root, mut connection) = open_fixture();
    let target = file_target("completed", ReadinessStage::PlaybackSummary, 1);
    replace(&mut connection, 1, std::slice::from_ref(&target));
    let pending = reconcile_readiness(&connection, SOURCE_ID, 10).expect("pending snapshot");
    publish_readiness_artifact(&mut connection, &ReadinessArtifact::for_target(&target, 11))
        .expect("publish completion");

    assert_eq!(
        persist_readiness_deficits(&mut connection, &pending.deficits, 12)
            .expect("ignore completed deficit"),
        0
    );
    let jobs: i64 = connection
        .query_row(
            "SELECT COUNT(*) FROM analysis_jobs WHERE readiness_managed = 1",
            [],
            |row| row.get(0),
        )
        .expect("count readiness work");
    assert_eq!(jobs, 0);
    let current = reconcile_readiness(&connection, SOURCE_ID, 13).expect("current snapshot");
    assert!(current.is_fully_ready());
    assert!(current.is_idle());
}

#[test]
fn delayed_deficit_preserves_terminal_failure_and_future_retry() {
    let (_root, mut connection) = open_fixture();
    let targets = [
        file_target("permanent", ReadinessStage::AnalysisFeatures, 1),
        file_target("retry", ReadinessStage::AnalysisFeatures, 1),
    ];
    replace(&mut connection, 1, &targets);
    let pending = reconcile_readiness(&connection, SOURCE_ID, 10).expect("pending snapshot");
    persist_readiness_deficits(&mut connection, &pending.deficits, 10)
        .expect("persist initial work");
    connection
        .execute(
            "UPDATE analysis_jobs
             SET status = 'failed', failure_kind = 'permanent', last_error = 'bad audio'
             WHERE readiness_scope_id = 'permanent'",
            [],
        )
        .expect("record permanent failure");
    connection
        .execute(
            "UPDATE analysis_jobs
             SET status = 'failed', failure_kind = 'retryable', retry_at = 50,
                 last_error = 'source busy'
             WHERE readiness_scope_id = 'retry'",
            [],
        )
        .expect("record future retry");

    assert_eq!(
        persist_readiness_deficits(&mut connection, &pending.deficits, 20)
            .expect("ignore non-actionable work"),
        0
    );
    let states: Vec<(String, String, Option<String>, Option<i64>)> = {
        let mut statement = connection
            .prepare(
                "SELECT readiness_scope_id, status, failure_kind, retry_at
                 FROM analysis_jobs
                 WHERE readiness_managed = 1
                 ORDER BY readiness_scope_id",
            )
            .expect("prepare state query");
        statement
            .query_map([], |row| {
                Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?))
            })
            .expect("query work state")
            .collect::<Result<_, _>>()
            .expect("collect work state")
    };
    assert_eq!(
        states,
        vec![
            (
                "permanent".to_string(),
                "failed".to_string(),
                Some("permanent".to_string()),
                None,
            ),
            (
                "retry".to_string(),
                "failed".to_string(),
                Some("retryable".to_string()),
                Some(50),
            ),
        ]
    );
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
