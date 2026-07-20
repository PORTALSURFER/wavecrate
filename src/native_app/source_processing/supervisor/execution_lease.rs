use super::{
    AtomicBool, ClaimedReadinessWork, Duration, ExecutionOutcome, Instant, Mutex, Ordering,
    ReadinessExecutionOutcome, ReadinessFailureOutcome, ReadinessLeaseRenewalOutcome,
    ReadinessStore, ReadinessWorkMutationOutcome, SampleSource, SourceDatabase,
    SourceDatabaseConnectionRole, SourceProcessingFailure, invalidate_persisted_waveform_cache_ref,
    now_epoch_seconds, thread,
};

pub(super) fn cleanup_unpublished_readiness_output(
    outcome: &Result<ReadinessExecutionOutcome, SourceProcessingFailure>,
) {
    if let Ok(ReadinessExecutionOutcome::Complete(Some(artifact_ref))) = outcome {
        invalidate_persisted_waveform_cache_ref(artifact_ref);
    }
}

pub(super) fn cancel_claim(
    connection: &mut rusqlite::Connection,
    claim: &ClaimedReadinessWork,
    reason: &str,
    now: i64,
) -> Result<ExecutionOutcome, String> {
    match ReadinessStore::new(connection)
        .cancel(claim, reason, now)
        .map_err(|error| error.to_string())?
    {
        ReadinessWorkMutationOutcome::Recorded => Ok(ExecutionOutcome::Cancelled),
        ReadinessWorkMutationOutcome::RejectedStale => Ok(ExecutionOutcome::Stale),
    }
}

pub(super) fn execution_outcome_for_failure(outcome: ReadinessFailureOutcome) -> ExecutionOutcome {
    match outcome {
        ReadinessFailureOutcome::RetryScheduled { retry_at } => {
            ExecutionOutcome::Retried { retry_at }
        }
        ReadinessFailureOutcome::RejectedStale => ExecutionOutcome::Stale,
        ReadinessFailureOutcome::Permanent
        | ReadinessFailureOutcome::Unsupported
        | ReadinessFailureOutcome::AttemptsExhausted => ExecutionOutcome::Failed,
    }
}

pub(super) fn run_with_readiness_lease_heartbeat<T>(
    source: &SampleSource,
    claim: &ClaimedReadinessWork,
    external_cancel: &AtomicBool,
    lease_duration_seconds: i64,
    execute: impl FnOnce(&AtomicBool) -> T,
) -> Result<(T, bool), String> {
    let local_cancel = AtomicBool::new(external_cancel.load(Ordering::Acquire));
    let stop = AtomicBool::new(false);
    let lease_stale = AtomicBool::new(false);
    let heartbeat_error = Mutex::new(None::<String>);
    let database_root = source.database_root().map_err(|error| error.to_string())?;
    let renew_interval = Duration::from_secs((lease_duration_seconds / 3).max(1) as u64);
    let mut heartbeat_connection = SourceDatabase::open_connection_with_role_and_database_root(
        &source.root,
        &database_root,
        SourceDatabaseConnectionRole::JobWorker,
    )
    .map_err(|error| error.to_string())?;

    let result = thread::scope(|scope| {
        scope.spawn(|| {
            let mut next_renewal = Instant::now() + renew_interval;
            while !stop.load(Ordering::Acquire) {
                if external_cancel.load(Ordering::Acquire) {
                    local_cancel.store(true, Ordering::Release);
                }
                if Instant::now() >= next_renewal {
                    match ReadinessStore::new(&mut heartbeat_connection).renew_lease(
                        claim,
                        now_epoch_seconds(),
                        lease_duration_seconds,
                    ) {
                        Ok(ReadinessLeaseRenewalOutcome::Renewed { .. }) => {
                            next_renewal = Instant::now() + renew_interval;
                        }
                        Ok(ReadinessLeaseRenewalOutcome::RejectedStale) => {
                            lease_stale.store(true, Ordering::Release);
                            local_cancel.store(true, Ordering::Release);
                            return;
                        }
                        Err(error) => {
                            *heartbeat_error
                                .lock()
                                .unwrap_or_else(|poison| poison.into_inner()) =
                                Some(error.to_string());
                            local_cancel.store(true, Ordering::Release);
                            return;
                        }
                    }
                }
                thread::sleep(Duration::from_millis(25));
            }
        });
        let result = execute(&local_cancel);
        stop.store(true, Ordering::Release);
        result
    });
    if let Some(error) = heartbeat_error
        .lock()
        .unwrap_or_else(|poison| poison.into_inner())
        .take()
    {
        return Err(format!("Readiness lease heartbeat failed: {error}"));
    }
    Ok((result, lease_stale.load(Ordering::Acquire)))
}
