use std::sync::{
    Arc, Condvar, Mutex,
    atomic::{AtomicU64, AtomicUsize, Ordering},
};
use std::time::Instant;

#[derive(Clone, Copy, Debug)]
pub(in crate::native_app::source_processing) enum DatabasePhase {
    Claim,
    Lease,
    Publish,
    ScanOpen,
    ScanManifest,
    ScanDeferredHash,
    SerialCompatibility,
}

#[derive(Default)]
struct PhaseCounters {
    count: AtomicU64,
    wait_ns: AtomicU64,
    held_ns: AtomicU64,
}

#[derive(Default)]
struct DatabaseWriterGateInner {
    locked: Mutex<bool>,
    wake: Condvar,
    #[cfg(test)]
    waiting: AtomicUsize,
    claim: PhaseCounters,
    lease: PhaseCounters,
    publish: PhaseCounters,
    scan_open: PhaseCounters,
    scan_manifest: PhaseCounters,
    scan_deferred_hash: PhaseCounters,
    serial: PhaseCounters,
    active: AtomicUsize,
    peak_active: AtomicUsize,
}

#[derive(Clone, Default)]
pub(in crate::native_app) struct DatabaseWriterGate {
    inner: Arc<DatabaseWriterGateInner>,
}

impl DatabaseWriterGate {
    pub(in crate::native_app::source_processing) fn lock(
        &self,
        phase: DatabasePhase,
    ) -> DatabaseWriterGuard {
        let waited_at = Instant::now();
        let mut locked = self
            .inner
            .locked
            .lock()
            .unwrap_or_else(|poison| poison.into_inner());
        #[cfg(test)]
        let registered_waiter = *locked;
        #[cfg(test)]
        if registered_waiter {
            self.inner.waiting.fetch_add(1, Ordering::AcqRel);
        }
        while *locked {
            locked = self
                .inner
                .wake
                .wait(locked)
                .unwrap_or_else(|poison| poison.into_inner());
        }
        #[cfg(test)]
        if registered_waiter {
            self.inner.waiting.fetch_sub(1, Ordering::AcqRel);
        }
        *locked = true;
        drop(locked);
        let counters = counters(&self.inner, phase);
        counters.count.fetch_add(1, Ordering::Relaxed);
        counters
            .wait_ns
            .fetch_add(duration_ns(waited_at), Ordering::Relaxed);
        let active = self
            .inner
            .active
            .fetch_add(1, Ordering::AcqRel)
            .saturating_add(1);
        self.inner.peak_active.fetch_max(active, Ordering::AcqRel);
        DatabaseWriterGuard {
            inner: Arc::clone(&self.inner),
            phase,
            acquired_at: Instant::now(),
        }
    }

    pub(super) fn snapshot(&self) -> DatabaseWriterSnapshot {
        DatabaseWriterSnapshot {
            claim: phase_snapshot(&self.inner.claim),
            lease: phase_snapshot(&self.inner.lease),
            publish: phase_snapshot(&self.inner.publish),
            scan_open: phase_snapshot(&self.inner.scan_open),
            scan_manifest: phase_snapshot(&self.inner.scan_manifest),
            scan_deferred_hash: phase_snapshot(&self.inner.scan_deferred_hash),
            serial: phase_snapshot(&self.inner.serial),
            active: self.inner.active.load(Ordering::Acquire),
            peak_active: self.inner.peak_active.load(Ordering::Acquire),
        }
    }

    #[cfg(test)]
    pub(in crate::native_app::source_processing) fn waiting_count(&self) -> usize {
        self.inner.waiting.load(Ordering::Acquire)
    }
}

pub(in crate::native_app) struct DatabaseWriterGuard {
    inner: Arc<DatabaseWriterGateInner>,
    phase: DatabasePhase,
    acquired_at: Instant,
}

impl Drop for DatabaseWriterGuard {
    fn drop(&mut self) {
        counters(&self.inner, self.phase)
            .held_ns
            .fetch_add(duration_ns(self.acquired_at), Ordering::Relaxed);
        self.inner.active.fetch_sub(1, Ordering::AcqRel);
        let mut locked = self
            .inner
            .locked
            .lock()
            .unwrap_or_else(|poison| poison.into_inner());
        *locked = false;
        drop(locked);
        self.inner.wake.notify_one();
    }
}

#[derive(Clone, Copy, Debug)]
pub(super) struct DatabasePhaseSnapshot {
    pub(super) count: u64,
    pub(super) wait_ms: f64,
    pub(super) held_ms: f64,
}

#[derive(Clone, Copy, Debug)]
pub(super) struct DatabaseWriterSnapshot {
    pub(super) claim: DatabasePhaseSnapshot,
    pub(super) lease: DatabasePhaseSnapshot,
    pub(super) publish: DatabasePhaseSnapshot,
    pub(super) scan_open: DatabasePhaseSnapshot,
    pub(super) scan_manifest: DatabasePhaseSnapshot,
    pub(super) scan_deferred_hash: DatabasePhaseSnapshot,
    pub(super) serial: DatabasePhaseSnapshot,
    pub(super) active: usize,
    pub(super) peak_active: usize,
}

fn counters(inner: &DatabaseWriterGateInner, phase: DatabasePhase) -> &PhaseCounters {
    match phase {
        DatabasePhase::Claim => &inner.claim,
        DatabasePhase::Lease => &inner.lease,
        DatabasePhase::Publish => &inner.publish,
        DatabasePhase::ScanOpen => &inner.scan_open,
        DatabasePhase::ScanManifest => &inner.scan_manifest,
        DatabasePhase::ScanDeferredHash => &inner.scan_deferred_hash,
        DatabasePhase::SerialCompatibility => &inner.serial,
    }
}

impl wavecrate_scan::sample_sources::scanner::ScanWriter for DatabaseWriterGate {
    type Guard = DatabaseWriterGuard;

    fn lock(&self, phase: wavecrate_scan::sample_sources::scanner::ScanWritePhase) -> Self::Guard {
        let phase = match phase {
            wavecrate_scan::sample_sources::scanner::ScanWritePhase::Open => {
                DatabasePhase::ScanOpen
            }
            wavecrate_scan::sample_sources::scanner::ScanWritePhase::Manifest => {
                DatabasePhase::ScanManifest
            }
            wavecrate_scan::sample_sources::scanner::ScanWritePhase::DeferredHash => {
                DatabasePhase::ScanDeferredHash
            }
        };
        DatabaseWriterGate::lock(self, phase)
    }
}

fn phase_snapshot(counters: &PhaseCounters) -> DatabasePhaseSnapshot {
    DatabasePhaseSnapshot {
        count: counters.count.load(Ordering::Acquire),
        wait_ms: counters.wait_ns.load(Ordering::Acquire) as f64 / 1_000_000.0,
        held_ms: counters.held_ns.load(Ordering::Acquire) as f64 / 1_000_000.0,
    }
}

fn duration_ns(started: Instant) -> u64 {
    started.elapsed().as_nanos().min(u64::MAX as u128) as u64
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn writer_gate_serializes_phases_and_records_wait_and_hold_time() {
        let gate = DatabaseWriterGate::default();
        let first = gate.clone();
        let second = gate.clone();
        let first_worker = std::thread::spawn(move || {
            let _guard = first.lock(DatabasePhase::Publish);
            std::thread::sleep(std::time::Duration::from_millis(20));
        });
        std::thread::sleep(std::time::Duration::from_millis(2));
        let second_worker = std::thread::spawn(move || {
            let _guard = second.lock(DatabasePhase::Claim);
        });
        first_worker.join().expect("first writer joins");
        second_worker.join().expect("second writer joins");

        let snapshot = gate.snapshot();
        assert_eq!(snapshot.peak_active, 1);
        assert_eq!(snapshot.active, 0);
        assert_eq!(snapshot.publish.count, 1);
        assert_eq!(snapshot.claim.count, 1);
        assert!(snapshot.publish.held_ms >= 10.0);
        assert!(snapshot.claim.wait_ms >= 5.0);
    }
}
