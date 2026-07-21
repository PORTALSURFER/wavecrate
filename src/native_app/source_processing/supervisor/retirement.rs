use super::{
    Arc, ControlState, Duration, SOURCE_RETIREMENT_RETRY_SECONDS, SampleSource, Shared,
    SourceRetirementOutcome, now_epoch_seconds, source_storage_identity_matches,
};
#[cfg(test)]
use super::{Ordering, thread};

pub(super) fn run_retirement_worker(shared: Arc<Shared>) {
    loop {
        let control = shared.control();
        let (control, _) = shared
            .retirement_wake
            .wait_timeout_while(control, Duration::from_secs(1), |control| {
                !control.shutdown && !source_retirement_is_ready(control, now_epoch_seconds())
            })
            .unwrap_or_else(|poison| poison.into_inner());
        if control.shutdown {
            return;
        }
        if !source_retirement_is_ready(&control, now_epoch_seconds()) {
            continue;
        }
        drop(control);
        process_ready_source_retirements(shared.as_ref());
        let control = shared.control();
        drop(
            shared
                .retirement_wake
                .wait_timeout(control, Duration::from_millis(250))
                .unwrap_or_else(|poison| poison.into_inner()),
        );
    }
}

pub(super) fn source_retirement_is_ready(control: &ControlState, now: i64) -> bool {
    control.pending_retirements.values().any(|retirement| {
        (!retirement.terminal_offline && retirement.retry_at <= now)
            || control
                .sources
                .values()
                .any(|active| source_storage_identity_matches(active, &retirement.source))
    })
}

pub(super) fn process_ready_source_retirements(shared: &Shared) {
    let now = now_epoch_seconds();
    let candidates = {
        let control = shared.control();
        control
            .pending_retirements
            .iter()
            .filter(|(_, retirement)| {
                (!retirement.terminal_offline && retirement.retry_at <= now)
                    || control
                        .sources
                        .values()
                        .any(|active| source_storage_identity_matches(active, &retirement.source))
            })
            .map(|(retirement_id, retirement)| (*retirement_id, retirement.clone()))
            .collect::<Vec<_>>()
    };
    let scheduled = candidates.len();
    let mut started = 0_usize;
    let mut retired = 0_usize;
    let mut offline = 0_usize;
    let mut cancelled = 0_usize;
    let mut retrying = 0_usize;

    for (retirement_id, retirement) in candidates {
        // Fence only the admission snapshot and final publication. The potentially blocking
        // source I/O runs in a killable child without owning `source_replacement`, so a fast
        // re-add can cancel and supersede it instead of freezing source configuration.
        {
            let _replacement = shared
                .source_replacement
                .lock()
                .unwrap_or_else(|poison| poison.into_inner());
            let control = shared.control();
            let Some(current) = control.pending_retirements.get(&retirement_id) else {
                continue;
            };
            if current.lifecycle_generation != retirement.lifecycle_generation {
                continue;
            }
            drop(control);
            if shared.source_has_external_activity(
                retirement.source.id.as_str(),
                retirement.lifecycle_generation,
            ) || shared.source_has_in_flight_work(
                retirement.source.id.as_str(),
                retirement.lifecycle_generation,
            ) {
                continue;
            }
            let mut control = shared.control();
            let Some(current) = control.pending_retirements.get(&retirement_id) else {
                continue;
            };
            if current.lifecycle_generation != retirement.lifecycle_generation {
                continue;
            }
            if let Some(source_id) = reactivated_source_id(&control, &retirement.source) {
                control.pending_retirements.remove(&retirement_id);
                control.dirty_sources.insert(source_id);
                control.notify("source_storage_handoff_completed");
                drop(control);
                shared.wake.notify_all();
                shared.budget_wake.notify_all();
                continue;
            }
        }
        started = started.saturating_add(1);
        tracing::debug!(
            target: "wavecrate::source_processing",
            event = "source_processing.retirement.started",
            source_id = retirement.source.id.as_str(),
            source_root = %retirement.source.root.display(),
            lifecycle_generation = retirement.lifecycle_generation,
            "Retiring removed source state"
        );
        #[cfg(test)]
        let result = if shared.retirement_cleanup_blocked.load(Ordering::Acquire) {
            shared
                .retirement_cleanup_started
                .store(true, Ordering::Release);
            while !shared.cancel.load(Ordering::Acquire)
                && !retirement.cancel.load(Ordering::Acquire)
            {
                thread::sleep(Duration::from_millis(5));
            }
            Ok(None)
        } else {
            super::super::worker::run_source_retirement(
                &retirement.source,
                retirement.cancel.as_ref(),
            )
        };
        #[cfg(not(test))]
        let result = super::super::worker::run_source_retirement(
            &retirement.source,
            retirement.cancel.as_ref(),
        );

        let _replacement = shared
            .source_replacement
            .lock()
            .unwrap_or_else(|poison| poison.into_inner());
        let mut control = shared.control();
        let Some(current) = control.pending_retirements.get(&retirement_id) else {
            continue;
        };
        if current.lifecycle_generation != retirement.lifecycle_generation {
            continue;
        }
        if let Some(source_id) = reactivated_source_id(&control, &retirement.source) {
            control.pending_retirements.remove(&retirement_id);
            control.dirty_sources.insert(source_id);
            control.notify("source_storage_handoff_completed");
            drop(control);
            shared.wake.notify_all();
            shared.budget_wake.notify_all();
            continue;
        }
        if control.shutdown {
            return;
        }
        match result {
            Ok(Some(SourceRetirementOutcome::Retired { retired_cache_refs })) => {
                control.pending_retirements.remove(&retirement_id);
                retired = retired.saturating_add(1);
                tracing::debug!(
                    target: "wavecrate::source_processing",
                    event = "source_processing.retirement.completed",
                    source_id = retirement.source.id.as_str(),
                    retired_cache_refs,
                    "Retired removed source runtime and path-derived cache ownership"
                );
            }
            Ok(Some(SourceRetirementOutcome::TerminalOffline)) => {
                if let Some(pending) = control.pending_retirements.get_mut(&retirement_id) {
                    pending.terminal_offline = true;
                    pending.retry_at = i64::MAX;
                }
                offline = offline.saturating_add(1);
                tracing::debug!(
                    target: "wavecrate::source_processing",
                    event = "source_processing.retirement.offline",
                    source_id = retirement.source.id.as_str(),
                    lifecycle_generation = retirement.lifecycle_generation,
                    "Removed source storage is offline; retirement is parked until re-add"
                );
            }
            Ok(None) => {
                control.pending_retirements.remove(&retirement_id);
                cancelled = cancelled.saturating_add(1);
                tracing::debug!(
                    target: "wavecrate::source_processing",
                    event = "source_processing.retirement.cancelled",
                    source_id = retirement.source.id.as_str(),
                    lifecycle_generation = retirement.lifecycle_generation,
                    "Cancelled superseded removed-source retirement"
                );
            }
            Err(error) => {
                if let Some(pending) = control.pending_retirements.get_mut(&retirement_id) {
                    pending.attempts = pending.attempts.saturating_add(1);
                    let delay = SOURCE_RETIREMENT_RETRY_SECONDS
                        .saturating_mul(1_i64 << pending.attempts.min(6));
                    pending.retry_at = now.saturating_add(delay);
                }
                retrying = retrying.saturating_add(1);
                tracing::warn!(
                    target: "wavecrate::source_processing",
                    event = "source_processing.retirement.retry",
                    source_id = retirement.source.id.as_str(),
                    attempt = control
                        .pending_retirements
                        .get(&retirement_id)
                        .map_or(0, |pending| pending.attempts),
                    retry_at = control
                        .pending_retirements
                        .get(&retirement_id)
                        .map_or(0, |pending| pending.retry_at),
                    error,
                    "Removed source retirement will retry without reactivating the source"
                );
            }
        }
    }
    if started > 0 {
        tracing::info!(
            target: "wavecrate::source_processing",
            event = "source_processing.retirement.sweep",
            scheduled,
            started,
            retired,
            offline,
            cancelled,
            retrying,
            "Removed-source retirement pass complete"
        );
    }
}

pub(super) fn reactivated_source_id(
    control: &ControlState,
    retired_source: &SampleSource,
) -> Option<String> {
    control.sources.values().find_map(|active| {
        source_storage_identity_matches(active, retired_source)
            .then(|| active.id.as_str().to_string())
    })
}
