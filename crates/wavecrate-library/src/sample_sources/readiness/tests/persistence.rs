use std::collections::BTreeSet;

use super::*;

#[test]
fn explicit_reanalysis_fences_running_work_and_requeues_exact_current_targets() {
    let (_root, mut connection) = open_fixture();
    let indexed = file_target("reanalysis", ReadinessStage::IndexedIdentity, 1);
    let analysis = file_target("reanalysis", ReadinessStage::AnalysisFeatures, 1);
    let embedding = file_target("reanalysis", ReadinessStage::EmbeddingAspects, 1);
    let layout = ReadinessTarget::source(
        SOURCE_ID,
        ReadinessStage::SimilarityLayout,
        "layout-v1",
        1,
        "membership-v1",
    );
    let targets = vec![
        indexed.clone(),
        analysis.clone(),
        embedding.clone(),
        layout.clone(),
    ];
    sync_manifest(&connection, &targets);
    replace_readiness_targets(
        &mut connection,
        SOURCE_ID,
        1,
        1,
        SourceAvailability::Active,
        &targets,
        1,
    )
    .expect("publish targets");
    let deficits = reconcile_readiness(&connection, SOURCE_ID, 2)
        .expect("initial deficits")
        .deficits;
    persist_readiness_deficits(&mut connection, &deficits, 2).expect("persist readiness work");
    let stale_claim = claim_readiness_target(&mut connection, &analysis, 3, 30)
        .expect("claim analysis")
        .expect("analysis claim available");
    for target in &targets {
        publish_readiness_artifact(&mut connection, &ReadinessArtifact::for_target(target, 4))
            .expect("seed current artifact");
    }

    let sample_id = format!("{SOURCE_ID}::Pack/reanalysis.wav");
    connection
        .execute_batch(&format!(
            "INSERT INTO samples (sample_id, content_hash, size, mtime_ns)
             VALUES ('{sample_id}', 'content', 1, 1);
             INSERT INTO analysis_features (sample_id, content_hash, features)
             VALUES ('{sample_id}', 'content', X'00');
             INSERT INTO features (sample_id, feat_version, vec_blob, computed_at)
             VALUES ('{sample_id}', 1, X'00', 1);
             INSERT INTO embeddings
                 (sample_id, model_id, dim, dtype, l2_normed, vec, created_at)
             VALUES ('{sample_id}', 'model', 1, 'f32', 1, X'00', 1);
             INSERT INTO similarity_aspect_descriptors
                 (sample_id, model_id, dim, dtype, l2_normed, valid_mask, vec, created_at)
             VALUES ('{sample_id}', 'aspects', 1, 'f32', 1, 0, X'00', 1);
             INSERT INTO layout_umap
                 (sample_id, model_id, umap_version, x, y, created_at)
             VALUES ('{sample_id}', 'model', 'layout', 0.0, 0.0, 1);
             INSERT INTO hdbscan_clusters
                 (sample_id, model_id, method, umap_version, cluster_id, created_at)
             VALUES ('{sample_id}', 'model', 'umap', 'layout', 0, 1);
             INSERT INTO analysis_cache_features
                 (content_hash, analysis_version, feat_version, vec_blob, computed_at,
                  duration_seconds, sr_used)
             VALUES ('content', 'v1', 1, X'00', 1, 1.0, 48000);"
        ))
        .expect("seed current derived outputs");

    ReadinessStore::new(&mut connection)
        .requeue_source_analysis(SOURCE_ID, 10)
        .expect("requeue source analysis");

    for table in [
        "analysis_features",
        "features",
        "embeddings",
        "similarity_aspect_descriptors",
        "layout_umap",
        "hdbscan_clusters",
    ] {
        assert_eq!(
            connection
                .query_row(&format!("SELECT COUNT(*) FROM {table}"), [], |row| {
                    row.get::<_, i64>(0)
                })
                .unwrap(),
            0,
            "{table} must be invalidated"
        );
    }
    assert_eq!(
        connection
            .query_row("SELECT COUNT(*) FROM analysis_cache_features", [], |row| {
                row.get::<_, i64>(0)
            })
            .unwrap(),
        1,
        "content-addressed caches remain reusable"
    );
    assert_eq!(
        connection
            .query_row(
                "SELECT COUNT(*) FROM source_readiness_artifacts
                 WHERE stage = 'indexed_identity'",
                [],
                |row| row.get::<_, i64>(0),
            )
            .unwrap(),
        1,
        "index identity remains current"
    );
    assert_eq!(
        connection
            .query_row(
                "SELECT COUNT(*) FROM source_readiness_artifacts
                 WHERE stage IN ('analysis_features', 'embedding_aspects', 'similarity_layout')",
                [],
                |row| row.get::<_, i64>(0),
            )
            .unwrap(),
        0
    );
    assert_eq!(
        complete_readiness_work(&mut connection, &stale_claim, 11)
            .expect("reject pre-reanalysis completion"),
        ArtifactPublishOutcome::RejectedStale
    );

    let snapshot = reconcile_readiness(&connection, SOURCE_ID, 11).expect("requeued snapshot");
    assert_eq!(
        entry_for(&snapshot, "reanalysis", ReadinessStage::IndexedIdentity).classification,
        ReadinessClassification::Current
    );
    for stage in [
        ReadinessStage::AnalysisFeatures,
        ReadinessStage::EmbeddingAspects,
    ] {
        assert_eq!(
            entry_for(&snapshot, "reanalysis", stage).classification,
            ReadinessClassification::Pending
        );
    }
    assert_eq!(
        entry_for(&snapshot, SOURCE_ID, ReadinessStage::SimilarityLayout).classification,
        ReadinessClassification::Pending
    );
    let fresh_claim = claim_readiness_target(&mut connection, &analysis, 11, 30)
        .expect("claim requeued analysis")
        .expect("requeued analysis available");
    assert!(fresh_claim.claim_generation > stale_claim.claim_generation);
    assert_eq!(
        complete_readiness_work(&mut connection, &fresh_claim, 12)
            .expect("publish exact reanalysis completion"),
        ArtifactPublishOutcome::Recorded
    );
}

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
fn invalidation_is_fenced_to_the_exact_current_artifact() {
    let (_root, mut connection) = open_fixture();
    let current = file_target("repair", ReadinessStage::AnalysisFeatures, 1);
    replace(&mut connection, 1, std::slice::from_ref(&current));
    publish_readiness_artifact(
        &mut connection,
        &ReadinessArtifact::for_target(&current, 10),
    )
    .expect("publish current artifact");

    let mut stale = current.clone();
    stale.content_generation = String::from("stale-content");
    assert!(
        !invalidate_readiness_artifact(&mut connection, &stale).expect("reject stale invalidation")
    );
    assert!(
        invalidate_readiness_artifact(&mut connection, &current)
            .expect("invalidate current artifact")
    );

    let snapshot = reconcile_readiness(&connection, SOURCE_ID, 11).expect("reconcile invalidation");
    assert_eq!(
        entry_for(&snapshot, "repair", ReadinessStage::AnalysisFeatures).classification,
        ReadinessClassification::Pending
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
    sync_manifest(&connection, &broken_targets);

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
    assert_eq!(snapshot.entries.len(), 4);
    assert!(snapshot.entries.iter().all(|entry| {
        entry.target.scope_id == "original" || entry.target.scope_kind == ReadinessScopeKind::Source
    }));
}

#[test]
fn stale_same_generation_publication_cannot_reactivate_disabled_source() {
    let (_root, mut connection) = open_fixture();
    let target = file_target("guarded", ReadinessStage::AnalysisFeatures, 1);
    let complete = complete_targets(1, std::slice::from_ref(&target));
    sync_manifest(&connection, &complete);
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
    sync_manifest(&connection, &complete);
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
    sync_manifest(&connection, &complete);
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
        ReadinessStage::AnalysisFeatures,
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
fn desired_targets_must_cover_every_current_manifest_identity() {
    let (_root, mut connection) = open_fixture();
    let included = file_target("included", ReadinessStage::IndexedIdentity, 1);
    let omitted = file_target("omitted", ReadinessStage::IndexedIdentity, 1);
    let full_manifest = complete_targets(1, &[included.clone(), omitted]);
    sync_manifest(&connection, &full_manifest);
    let incomplete = complete_targets(1, &[included]);

    let error = replace_readiness_targets(
        &mut connection,
        SOURCE_ID,
        1,
        1,
        SourceAvailability::Active,
        &incomplete,
        10,
    )
    .expect_err("reject omitted current file identity");
    assert!(matches!(
        error,
        ReadinessError::ManifestMembershipMismatch { missing, unexpected }
            if missing == ["omitted"] && unexpected.is_empty()
    ));
}

#[test]
fn desired_targets_must_match_authoritative_manifest_paths_one_to_one() {
    let (_root, mut connection) = open_fixture();
    let seed = file_target("renamed", ReadinessStage::IndexedIdentity, 1);
    let targets = complete_targets(1, std::slice::from_ref(&seed));
    sync_manifest(&connection, &targets);
    connection
        .execute(
            "UPDATE wav_files SET path = 'Pack/current-name.wav' WHERE file_identity = 'renamed'",
            [],
        )
        .expect("rename manifest path");
    let error = replace_readiness_targets(
        &mut connection,
        SOURCE_ID,
        1,
        1,
        SourceAvailability::Active,
        &targets,
        10,
    )
    .expect_err("reject stale target path after rename");
    assert!(matches!(error, ReadinessError::ManifestPathMismatch { .. }));

    sync_manifest(&connection, &targets);
    connection
        .execute(
            "INSERT INTO wav_files (
                path, file_size, modified_ns, extension, missing, file_identity
             ) VALUES ('Pack/hard-link.wav', 1, 1, 'wav', 0, 'renamed')",
            [],
        )
        .expect("seed duplicate current identity");
    let error = replace_readiness_targets(
        &mut connection,
        SOURCE_ID,
        1,
        1,
        SourceAvailability::Active,
        &targets,
        11,
    )
    .expect_err("reject duplicate current identity");
    assert!(matches!(
        error,
        ReadinessError::DuplicateManifestIdentity { .. }
    ));

    sync_manifest(&connection, &targets);
    let mut inconsistent = targets;
    inconsistent
        .iter_mut()
        .find(|target| target.stage == ReadinessStage::AnalysisFeatures)
        .expect("analysis target")
        .relative_path = Some("Pack/another-name.wav".to_string());
    let error = replace_readiness_targets(
        &mut connection,
        SOURCE_ID,
        1,
        1,
        SourceAvailability::Active,
        &inconsistent,
        12,
    )
    .expect_err("reject inconsistent stage paths");
    assert!(matches!(
        error,
        ReadinessError::InconsistentTargetPath { .. }
    ));
}

#[test]
fn delayed_deficit_cannot_enqueue_after_disable_or_delete() {
    let (_root, mut connection) = open_fixture();
    let target = file_target("inactive", ReadinessStage::AnalysisFeatures, 1);
    let complete = complete_targets(1, std::slice::from_ref(&target));
    sync_manifest(&connection, &complete);
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
    let deleted_targets = complete_targets(1, &[deleted]);
    sync_manifest(&connection, &deleted_targets);
    replace_readiness_targets(
        &mut connection,
        SOURCE_ID,
        1,
        3,
        SourceAvailability::Active,
        &deleted_targets,
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
    let target = file_target("completed", ReadinessStage::AnalysisFeatures, 1);
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
fn one_identity_delta_preserves_unchanged_targets_and_running_work() {
    let (_root, mut connection) = open_fixture();
    let mut initial = Vec::new();
    initial.extend(eligible_file_targets(
        "keep",
        "Pack/keep.wav",
        1,
        "keep-hash",
    ));
    initial.extend(eligible_file_targets(
        "change",
        "Pack/change.wav",
        1,
        "old-hash",
    ));
    initial.extend(eligible_file_targets(
        "delete",
        "Pack/delete.wav",
        1,
        "delete-hash",
    ));
    let mut membership = ReadinessMembership::default();
    membership.add("keep", "keep-hash");
    membership.add("change", "old-hash");
    membership.add("delete", "delete-hash");
    initial.push(ReadinessTarget::source(
        SOURCE_ID,
        ReadinessStage::SimilarityLayout,
        "layout-v1",
        1,
        membership.generation(),
    ));
    sync_manifest(&connection, &initial);
    replace_readiness_targets(
        &mut connection,
        SOURCE_ID,
        1,
        101,
        SourceAvailability::Active,
        &initial,
        100,
    )
    .expect("publish initial targets");
    let initial_snapshot =
        reconcile_readiness(&connection, SOURCE_ID, 101).expect("initial deficits");
    persist_readiness_deficits(&mut connection, &initial_snapshot.deficits, 101)
        .expect("persist initial work");
    let keep_analysis = initial
        .iter()
        .find(|target| {
            target.scope_id == "keep" && target.stage == ReadinessStage::AnalysisFeatures
        })
        .expect("keep analysis target")
        .clone();
    let keep_claim = claim_readiness_target(&mut connection, &keep_analysis, 102, 300)
        .expect("claim keep analysis")
        .expect("keep analysis claim");
    let keep_embedding = initial
        .iter()
        .find(|target| {
            target.scope_id == "keep" && target.stage == ReadinessStage::EmbeddingAspects
        })
        .expect("keep embedding target");
    assert_eq!(
        publish_readiness_artifact(
            &mut connection,
            &ReadinessArtifact::for_target(keep_embedding, 103),
        )
        .expect("publish unchanged embedding artifact"),
        ArtifactPublishOutcome::Recorded
    );

    let changed = eligible_file_targets("change", "Pack/change.wav", 2, "new-hash").to_vec();
    let mut current_manifest_targets =
        eligible_file_targets("keep", "Pack/keep.wav", 1, "keep-hash").to_vec();
    current_manifest_targets.extend(changed.iter().cloned());
    sync_manifest(&connection, &current_manifest_targets);
    let deleted_scope_ids = [String::from("delete")];
    let publication = ReadinessTargetDeltaPublication::new(
        SOURCE_ID,
        2,
        102,
        SourceAvailability::Active,
        "readiness-test-contract-v1",
        &changed,
        &deleted_scope_ids,
        "layout-v1",
        200,
    );
    let outcome = ReadinessStore::new(&mut connection)
        .publish_target_delta_with_cancel(&publication, &std::sync::atomic::AtomicBool::new(false))
        .expect("publish readiness delta");
    membership.remove("change", "old-hash");
    membership.remove("delete", "delete-hash");
    membership.add("change", "new-hash");
    let ReadinessDeltaPublicationOutcome::Applied {
        membership_generation,
        changed: changed_rows_count,
    } = outcome
    else {
        panic!("one-generation delta unexpectedly required a full publication");
    };
    assert_eq!(membership_generation, membership.generation());
    assert!(changed_rows_count <= 15);
    let delta_snapshot = ReadinessStore::new(&mut connection)
        .reconcile_scopes_with_cancel_and_progress(
            SOURCE_ID,
            &BTreeSet::from([String::from("change"), String::from("delete")]),
            200,
            &std::sync::atomic::AtomicBool::new(false),
            &mut || {},
        )
        .expect("reconcile only affected readiness scopes");
    assert_eq!(delta_snapshot.entries.len(), 4);
    assert!(
        delta_snapshot
            .entries
            .iter()
            .all(|entry| entry.target.scope_id != "keep")
    );

    let keep_rows = connection
        .query_row(
            "SELECT COUNT(*), MIN(source_generation), MAX(updated_at)
             FROM source_readiness_targets
             WHERE source_id = ?1 AND scope_kind = 'file' AND scope_id = 'keep'",
            [SOURCE_ID],
            |row| {
                Ok((
                    row.get::<_, i64>(0)?,
                    row.get::<_, i64>(1)?,
                    row.get::<_, i64>(2)?,
                ))
            },
        )
        .expect("read unchanged targets");
    assert_eq!(keep_rows, (3, 1, 100));
    let changed_rows = connection
        .query_row(
            "SELECT COUNT(*), MIN(source_generation), MIN(content_generation)
             FROM source_readiness_targets
             WHERE source_id = ?1 AND scope_kind = 'file' AND scope_id = 'change'",
            [SOURCE_ID],
            |row| {
                Ok((
                    row.get::<_, i64>(0)?,
                    row.get::<_, i64>(1)?,
                    row.get::<_, String>(2)?,
                ))
            },
        )
        .expect("read changed targets");
    assert_eq!(changed_rows, (3, 2, String::from("new-hash")));
    let deleted_rows: i64 = connection
        .query_row(
            "SELECT COUNT(*) FROM source_readiness_targets
             WHERE source_id = ?1 AND scope_id = 'delete'",
            [SOURCE_ID],
            |row| row.get(0),
        )
        .expect("count deleted targets");
    assert_eq!(deleted_rows, 0);
    let changed_embedding = changed
        .iter()
        .find(|target| target.stage == ReadinessStage::EmbeddingAspects)
        .expect("changed embedding target");
    assert_eq!(
        publish_readiness_artifact(
            &mut connection,
            &ReadinessArtifact::for_target(changed_embedding, 201),
        )
        .expect("publish changed embedding artifact"),
        ArtifactPublishOutcome::Recorded
    );
    let embedding_targets = ReadinessStore::new(&mut connection)
        .embedding_artifact_targets(SOURCE_ID, 2)
        .expect("load exact cross-generation embedding membership");
    assert_eq!(
        embedding_targets
            .iter()
            .map(|target| (target.scope_id.as_str(), target.source_generation))
            .collect::<BTreeSet<_>>(),
        BTreeSet::from([("change", 2), ("keep", 1)])
    );
    let keep_work = connection
        .query_row(
            "SELECT status, readiness_claim_generation
             FROM analysis_jobs
             WHERE source_id = ?1
               AND readiness_scope_id = 'keep'
               AND readiness_stage = 'analysis_features'",
            [SOURCE_ID],
            |row| Ok((row.get::<_, String>(0)?, row.get::<_, u32>(1)?)),
        )
        .expect("read preserved running work");
    assert_eq!(
        keep_work,
        (String::from("running"), keep_claim.claim_generation())
    );
    assert_eq!(
        complete_readiness_work(&mut connection, &keep_claim, 201)
            .expect("complete unchanged file work after unrelated delta"),
        ArtifactPublishOutcome::Recorded
    );
}

#[test]
fn readiness_delta_generation_gap_requires_complete_publication() {
    let (_root, mut connection) = open_fixture();
    let initial = eligible_file_targets("file", "Pack/file.wav", 1, "old-hash").to_vec();
    replace(&mut connection, 1, &initial);
    let changed = eligible_file_targets("file", "Pack/file.wav", 3, "new-hash").to_vec();
    sync_manifest(&connection, &changed);
    let publication = ReadinessTargetDeltaPublication::new(
        SOURCE_ID,
        3,
        102,
        SourceAvailability::Active,
        "readiness-test-contract-v1",
        &changed,
        &[],
        "layout-v1",
        200,
    );
    assert_eq!(
        ReadinessStore::new(&mut connection)
            .publish_target_delta_with_cancel(
                &publication,
                &std::sync::atomic::AtomicBool::new(false),
            )
            .expect("attempt readiness delta"),
        ReadinessDeltaPublicationOutcome::RequiresFullPublication
    );
    let state = ReadinessStore::new(&mut connection)
        .source_state(SOURCE_ID)
        .expect("read source state")
        .expect("source state exists");
    assert_eq!(state.source_generation, 1);
}

#[test]
fn target_replacement_prunes_removed_legacy_playback_work() {
    let (_root, mut connection) = open_fixture();
    let current = file_target("legacy-playback", ReadinessStage::AnalysisFeatures, 1);
    replace(&mut connection, 1, std::slice::from_ref(&current));
    let pending = reconcile_readiness(&connection, SOURCE_ID, 10).expect("current snapshot");
    persist_readiness_deficits(&mut connection, &pending.deficits, 10)
        .expect("persist current work");
    connection
        .execute(
            "INSERT INTO source_readiness_targets (
                source_id, scope_kind, scope_id, relative_path, stage, required_version,
                source_generation, content_generation, eligibility, updated_at
             )
             SELECT source_id, scope_kind, scope_id, relative_path, 'playback_summary',
                    'legacy-playback-v1', source_generation, content_generation,
                    eligibility, updated_at
             FROM source_readiness_targets
             WHERE source_id = ?1
               AND scope_id = 'legacy-playback'
               AND stage = 'analysis_features'",
            [SOURCE_ID],
        )
        .expect("seed legacy playback target");
    connection
        .execute(
            "UPDATE analysis_jobs
             SET job_type = 'readiness_playback_summary_v1',
                 readiness_stage = 'playback_summary'
             WHERE source_id = ?1
               AND readiness_scope_id = 'legacy-playback'
               AND readiness_stage = 'analysis_features'",
            [SOURCE_ID],
        )
        .expect("seed legacy playback work");

    let replacement = file_target("legacy-playback", ReadinessStage::AnalysisFeatures, 2);
    replace(&mut connection, 2, &[replacement]);

    let playback_rows: i64 = connection
        .query_row(
            "SELECT
                (SELECT COUNT(*) FROM source_readiness_targets
                 WHERE source_id = ?1 AND stage = 'playback_summary')
              + (SELECT COUNT(*) FROM analysis_jobs
                 WHERE source_id = ?1
                   AND readiness_managed = 1
                   AND readiness_stage = 'playback_summary')",
            [SOURCE_ID],
            |row| row.get(0),
        )
        .expect("count removed legacy playback rows");
    assert_eq!(playback_rows, 0);
}

#[test]
fn read_only_reconciliation_ignores_legacy_playback_rows_without_enqueuing() {
    let (root, mut connection) = open_fixture();
    let target = file_target("read-only", ReadinessStage::AnalysisFeatures, 1);
    replace(&mut connection, 1, &[target]);
    connection
        .execute(
            "INSERT INTO source_readiness_targets (
                source_id, scope_kind, scope_id, relative_path, stage, required_version,
                source_generation, content_generation, eligibility, updated_at
             )
             SELECT source_id, scope_kind, scope_id, relative_path, 'playback_summary',
                    'legacy-playback-v1', source_generation, content_generation,
                    eligibility, updated_at
             FROM source_readiness_targets
             WHERE source_id = ?1
               AND scope_id = 'read-only'
               AND stage = 'analysis_features'",
            [SOURCE_ID],
        )
        .expect("seed legacy read-only target");
    connection
        .execute(
            "INSERT INTO source_readiness_artifacts (
                source_id, scope_kind, scope_id, relative_path, stage, artifact_version,
                source_generation, content_generation, completed_at
             )
             SELECT source_id, scope_kind, scope_id, relative_path, stage, required_version,
                    source_generation, content_generation, updated_at
             FROM source_readiness_targets
             WHERE source_id = ?1
               AND scope_id = 'read-only'
               AND stage = 'playback_summary'",
            [SOURCE_ID],
        )
        .expect("seed legacy read-only artifact");
    drop(connection);
    let read_only = SourceDatabase::open_connection_with_role(
        root.path(),
        SourceDatabaseConnectionRole::UiRead,
    )
    .expect("read-only source db");

    let snapshot = reconcile_readiness(&read_only, SOURCE_ID, 2).expect("read snapshot");
    assert_eq!(snapshot.deficits.len(), 1);
    assert_eq!(snapshot.entries.len(), 4);
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
