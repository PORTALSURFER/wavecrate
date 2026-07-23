use super::{Instant, Ordering, SourceProcessingSupervisor, Value};

impl SourceProcessingSupervisor {
    pub(in crate::native_app) fn shutdown(&mut self) -> Value {
        let started_at = Instant::now();
        self.shared.cancel.store(true, Ordering::Release);
        self.shared.cancel_external_scans(|_| true);
        {
            let mut control = self.shared.control();
            control.cancel_all_source_work();
            for retirement in control.pending_retirements.values() {
                retirement.cancel.store(true, Ordering::Release);
            }
            control.shutdown = true;
            control.notify("shutdown");
        }
        self.shared.wake.notify_all();
        self.shared.budget_wake.notify_all();
        self.shared.retirement_wake.notify_all();
        let coordinator_joined = self
            .coordinator
            .take()
            .is_none_or(|coordinator| coordinator.join().is_ok());
        let retirement_joined = self
            .retirement_worker
            .take()
            .is_none_or(|worker| worker.join().is_ok());
        let joined = coordinator_joined && retirement_joined;
        self.shared.wait_for_external_scans();
        let database = self.shared.database_writer.snapshot();
        let database_phases = serde_json::json!({
            "writer_active": database.active,
            "writer_peak_active": database.peak_active,
            "claim": { "count": database.claim.count, "wait_ms": database.claim.wait_ms, "held_ms": database.claim.held_ms },
            "lease": { "count": database.lease.count, "wait_ms": database.lease.wait_ms, "held_ms": database.lease.held_ms },
            "publish": { "count": database.publish.count, "wait_ms": database.publish.wait_ms, "held_ms": database.publish.held_ms },
            "scan_open": { "count": database.scan_open.count, "wait_ms": database.scan_open.wait_ms, "held_ms": database.scan_open.held_ms },
            "scan_manifest": { "count": database.scan_manifest.count, "wait_ms": database.scan_manifest.wait_ms, "held_ms": database.scan_manifest.held_ms },
            "scan_deferred_hash": { "count": database.scan_deferred_hash.count, "wait_ms": database.scan_deferred_hash.wait_ms, "held_ms": database.scan_deferred_hash.held_ms },
            "serial": { "count": database.serial.count, "wait_ms": database.serial.wait_ms, "held_ms": database.serial.held_ms },
        });
        let telemetry = self.shared.telemetry();
        serde_json::json!({
            "joined": joined,
            "external_scans_joined": true,
            "elapsed_ms": started_at.elapsed().as_secs_f64() * 1_000.0,
            "sweeps": telemetry.sweeps,
            "claimed": telemetry.claimed,
            "completed": telemetry.completed,
            "failed": telemetry.failed,
            "retried": telemetry.retried,
            "stale": telemetry.stale,
            "cancelled": telemetry.cancelled,
            "contention": telemetry.contention,
            "max_queue_depth": telemetry.max_queue_depth,
            "queue_depth": telemetry.queue_depth,
            "oldest_job_age_seconds": telemetry.oldest_job_age_seconds,
            "retries_due": telemetry.retries_due,
            "readiness_queue_depth": telemetry.readiness_queue_depth,
            "queue_depth_by_source": telemetry.queue_depth_by_source,
            "readiness_queue_depth_by_source": telemetry.readiness_queue_depth_by_source,
            "retries_due_by_source": telemetry.retries_due_by_source,
            "retry_at_by_source": telemetry.retry_at_by_source,
            "source_discoveries": telemetry.source_discoveries,
            "cheap_noop_sweeps": telemetry.cheap_noop_sweeps,
            "delta_reconciliations": telemetry.delta_reconciliations,
            "full_audits": telemetry.full_audits,
            "settled_wake_generation": telemetry.settled_wake_generation,
            "active_execution_workers": telemetry.active_execution_workers,
            "peak_execution_workers": telemetry.peak_execution_workers,
            "execution_count": telemetry.execution_count,
            "execution_elapsed_ms": telemetry.execution_elapsed_ms,
            "execution_queue_depth": telemetry.execution_queue_depth,
            "max_execution_queue_depth": telemetry.max_execution_queue_depth,
            "database_phases": database_phases,
        })
    }
}

impl Drop for SourceProcessingSupervisor {
    fn drop(&mut self) {
        if self.coordinator.is_some() || self.retirement_worker.is_some() {
            let _ = self.shutdown();
        }
    }
}
