use super::*;

fn persist_target(connection: &mut Connection, target: &ReadinessTarget, now: i64) {
    let snapshot = reconcile_readiness(connection, SOURCE_ID, now).expect("reconcile target");
    let deficit = snapshot
        .deficits
        .iter()
        .find(|deficit| {
            deficit.target.scope_id == target.scope_id && deficit.target.stage == target.stage
        })
        .expect("target deficit")
        .clone();
    persist_readiness_deficits(connection, &[deficit], now).expect("persist target work");
}

#[test]
fn exact_target_claim_is_deduplicated_and_generation_fenced() {
    let (_root, mut connection) = open_fixture();
    let target = file_target("claimed", ReadinessStage::AnalysisFeatures, 1);
    replace(&mut connection, 1, std::slice::from_ref(&target));
    persist_target(&mut connection, &target, 10);
    persist_target(&mut connection, &target, 10);

    let claim = claim_readiness_target(&mut connection, &target, 10, 30)
        .expect("claim target")
        .expect("claim available");
    assert_eq!(claim.target, target);
    assert_eq!(claim.attempt, 1);
    assert_eq!(claim.lease_expires_at, 40);
    assert_eq!(
        claim_readiness_target(&mut connection, &target, 11, 30).expect("duplicate claim"),
        None
    );

    let stats = readiness_work_stats(&connection, 11).expect("work stats");
    assert_eq!(stats.total, 1);
    assert_eq!(stats.running, 1);
    assert_eq!(stats.pending, 0);
}

#[test]
fn expired_claim_is_recovered_after_restart_with_a_new_attempt() {
    let (root, mut connection) = open_fixture();
    let target = file_target("restart", ReadinessStage::PlaybackSummary, 1);
    replace(&mut connection, 1, std::slice::from_ref(&target));
    persist_target(&mut connection, &target, 10);
    let first = claim_readiness_target(&mut connection, &target, 10, 10)
        .expect("first claim")
        .expect("first claim available");
    drop(connection);

    let mut reopened = SourceDatabase::open_connection(root.path()).expect("reopen source db");
    assert_eq!(
        claim_readiness_target(&mut reopened, &target, 19, 10).expect("lease still active"),
        None
    );
    let expired = readiness_work_stats(&reopened, 20).expect("expired stats");
    assert_eq!(expired.expired_leases, 1);
    let recovered = claim_readiness_target(&mut reopened, &target, 20, 10)
        .expect("recover claim")
        .expect("expired claim available");
    assert_eq!(recovered.attempt, first.attempt + 1);
    assert_eq!(recovered.lease_expires_at, 30);
    assert_eq!(
        complete_readiness_work(&mut reopened, &first, 21).expect("stale completion"),
        ArtifactPublishOutcome::RejectedStale
    );
}

#[test]
fn stale_completion_cannot_publish_over_a_changed_exact_target() {
    let (_root, mut connection) = open_fixture();
    let original = file_target("changed", ReadinessStage::EmbeddingAspects, 1);
    replace(&mut connection, 1, std::slice::from_ref(&original));
    persist_target(&mut connection, &original, 0);
    let stale_claim = claim_readiness_target(&mut connection, &original, 0, 100)
        .expect("claim original")
        .expect("original available");

    let current = file_target("changed", ReadinessStage::EmbeddingAspects, 2);
    replace(&mut connection, 2, std::slice::from_ref(&current));
    persist_target(&mut connection, &current, 1);
    assert_eq!(
        complete_readiness_work(&mut connection, &stale_claim, 2).expect("reject stale completion"),
        ArtifactPublishOutcome::RejectedStale
    );

    let current_claim = claim_readiness_target(&mut connection, &current, 2, 100)
        .expect("claim current")
        .expect("current available");
    assert_eq!(
        complete_readiness_work(&mut connection, &current_claim, 3).expect("complete current"),
        ArtifactPublishOutcome::Recorded
    );
    assert!(
        reconcile_readiness(&connection, SOURCE_ID, 4)
            .expect("current snapshot")
            .is_fully_ready()
    );
}

#[test]
fn file_completion_survives_unrelated_source_generation_changes() {
    let (_root, mut connection) = open_fixture();
    let original = file_target("stable", ReadinessStage::AnalysisFeatures, 1);
    replace(&mut connection, 1, std::slice::from_ref(&original));
    persist_target(&mut connection, &original, 0);
    let claim = claim_readiness_target(&mut connection, &original, 0, 100)
        .expect("claim original")
        .expect("original available");

    let mut unchanged = original.clone();
    unchanged.source_generation = 2;
    unchanged.relative_path = Some("Renamed/stable.wav".to_string());
    replace(&mut connection, 2, std::slice::from_ref(&unchanged));
    assert_eq!(
        complete_readiness_work(&mut connection, &claim, 1).expect("complete stable file"),
        ArtifactPublishOutcome::Recorded
    );
    assert_eq!(
        entry_for(
            &reconcile_readiness(&connection, SOURCE_ID, 2).expect("reconcile renamed file"),
            "stable",
            ReadinessStage::AnalysisFeatures,
        )
        .classification,
        ReadinessClassification::Current
    );
}

#[test]
fn retry_backoff_is_bounded_and_exhaustion_becomes_terminal() {
    let (_root, mut connection) = open_fixture();
    let target = file_target("retry", ReadinessStage::PlaybackSummary, 1);
    replace(&mut connection, 1, std::slice::from_ref(&target));
    persist_target(&mut connection, &target, 0);
    let policy = ReadinessRetryPolicy::new(5, 20, 3).expect("valid retry policy");
    assert_eq!(policy.delay_for_attempt(1), 5);
    assert_eq!(policy.delay_for_attempt(2), 10);
    assert_eq!(policy.delay_for_attempt(20), 20);

    let first = claim_readiness_target(&mut connection, &target, 0, 100)
        .expect("claim first")
        .expect("first available");
    assert_eq!(
        fail_readiness_work(
            &mut connection,
            &first,
            ReadinessFailureClassification::Retryable,
            "database busy",
            0,
            policy,
        )
        .expect("fail first"),
        ReadinessFailureOutcome::RetryScheduled { retry_at: 5 }
    );
    assert_eq!(
        claim_readiness_target(&mut connection, &target, 4, 100).expect("retry not due"),
        None
    );
    assert_eq!(
        readiness_work_stats(&connection, 4)
            .expect("waiting stats")
            .retries_waiting,
        1
    );
    assert_eq!(
        readiness_work_stats(&connection, 4)
            .expect("waiting deadline")
            .earliest_retry_at,
        Some(5)
    );

    let second = claim_readiness_target(&mut connection, &target, 5, 100)
        .expect("claim second")
        .expect("second available");
    assert_eq!(second.attempt, 2);
    assert_eq!(
        fail_readiness_work(
            &mut connection,
            &second,
            ReadinessFailureClassification::Retryable,
            "database busy",
            5,
            policy,
        )
        .expect("fail second"),
        ReadinessFailureOutcome::RetryScheduled { retry_at: 15 }
    );
    let third = claim_readiness_target(&mut connection, &target, 15, 100)
        .expect("claim third")
        .expect("third available");
    assert_eq!(third.attempt, 3);
    assert_eq!(
        fail_readiness_work(
            &mut connection,
            &third,
            ReadinessFailureClassification::Retryable,
            "still busy",
            15,
            policy,
        )
        .expect("exhaust retries"),
        ReadinessFailureOutcome::AttemptsExhausted
    );
    assert_eq!(
        claim_readiness_target(&mut connection, &target, 1_000, 100).expect("terminal claim"),
        None
    );
    let stats = readiness_work_stats(&connection, 1_000).expect("terminal stats");
    assert_eq!(stats.permanent_failures, 1);
    assert_eq!(stats.retries_due, 0);
}

#[test]
fn explicit_failure_classifications_are_terminal_and_observable() {
    let (_root, mut connection) = open_fixture();
    let permanent = file_target("permanent", ReadinessStage::AnalysisFeatures, 1);
    let unsupported = file_target("unsupported", ReadinessStage::EmbeddingAspects, 1);
    replace(
        &mut connection,
        1,
        &[permanent.clone(), unsupported.clone()],
    );
    persist_target(&mut connection, &permanent, 0);
    persist_target(&mut connection, &unsupported, 0);
    let policy = ReadinessRetryPolicy::new(1, 8, 3).expect("valid retry policy");

    for (target, classification, expected) in [
        (
            &permanent,
            ReadinessFailureClassification::Permanent,
            ReadinessFailureOutcome::Permanent,
        ),
        (
            &unsupported,
            ReadinessFailureClassification::Unsupported,
            ReadinessFailureOutcome::Unsupported,
        ),
    ] {
        let claim = claim_readiness_target(&mut connection, target, 0, 100)
            .expect("claim classified target")
            .expect("classified target available");
        assert_eq!(
            fail_readiness_work(
                &mut connection,
                &claim,
                classification,
                "terminal",
                0,
                policy,
            )
            .expect("record classified failure"),
            expected
        );
    }
    let stats = readiness_work_stats(&connection, 1).expect("classification stats");
    assert_eq!(stats.permanent_failures, 1);
    assert_eq!(stats.unsupported, 1);
}

#[test]
fn release_and_cancel_return_claims_to_pending_without_deletion() {
    let (_root, mut connection) = open_fixture();
    let target = file_target("cancel", ReadinessStage::AnalysisFeatures, 1);
    replace(&mut connection, 1, std::slice::from_ref(&target));
    persist_target(&mut connection, &target, 0);

    let first = claim_readiness_target(&mut connection, &target, 0, 100)
        .expect("claim first")
        .expect("first available");
    assert_eq!(
        release_readiness_work(&mut connection, &first, 1).expect("release claim"),
        ReadinessWorkMutationOutcome::Recorded
    );
    let second = claim_readiness_target(&mut connection, &target, 1, 100)
        .expect("claim second")
        .expect("second available");
    assert_eq!(second.attempt, 2);
    assert_eq!(
        cancel_readiness_work(&mut connection, &second, "shutdown", 2).expect("cancel claim"),
        ReadinessWorkMutationOutcome::Recorded
    );
    let cancelled = readiness_work_stats(&connection, 2).expect("cancel stats");
    assert_eq!(cancelled.total, 1);
    assert_eq!(cancelled.pending, 1);
    assert_eq!(cancelled.cancelled, 1);

    let third = claim_readiness_target(&mut connection, &target, 2, 100)
        .expect("claim cancelled work")
        .expect("cancelled work available");
    assert_eq!(third.attempt, 3);
    assert_eq!(
        readiness_work_stats(&connection, 2)
            .expect("reclaimed stats")
            .cancelled,
        0
    );
}

#[test]
fn lease_renewal_extends_only_the_current_unexpired_claim() {
    let (_root, mut connection) = open_fixture();
    let target = file_target("renew", ReadinessStage::PlaybackSummary, 1);
    replace(&mut connection, 1, std::slice::from_ref(&target));
    persist_target(&mut connection, &target, 0);
    let claim = claim_readiness_target(&mut connection, &target, 0, 10)
        .expect("claim target")
        .expect("target available");

    assert_eq!(
        renew_readiness_lease(&mut connection, &claim, 5, 20).expect("renew lease"),
        ReadinessLeaseRenewalOutcome::Renewed {
            lease_expires_at: 25
        }
    );
    assert_eq!(
        claim_readiness_target(&mut connection, &target, 20, 10).expect("still leased"),
        None
    );
    assert_eq!(
        complete_readiness_work(&mut connection, &claim, 24).expect("complete renewed claim"),
        ArtifactPublishOutcome::Recorded
    );
    assert_eq!(
        renew_readiness_lease(&mut connection, &claim, 24, 20).expect("renew completed claim"),
        ReadinessLeaseRenewalOutcome::RejectedStale
    );
}
