use std::{
    path::Path,
    sync::atomic::{AtomicU64, Ordering},
    time::{Duration, Instant, SystemTime, UNIX_EPOCH},
};

const SLOW_CACHE_PHASE_THRESHOLD: Duration = Duration::from_millis(4);
const SLOW_CACHE_PHASE_LOG_INTERVAL_MS: u64 = 250;

static LAST_SLOW_CACHE_PHASE_LOG_MS: AtomicU64 = AtomicU64::new(0);

pub(super) fn log_slow_cache_phase(event: &'static str, path: &Path, started_at: Instant) {
    let elapsed = started_at.elapsed();
    if elapsed < SLOW_CACHE_PHASE_THRESHOLD || !claim_slow_cache_phase_log(now_ms()) {
        return;
    }
    tracing::warn!(
        target: "wavecrate::debug::sample_cache",
        event,
        elapsed_ms = elapsed.as_secs_f64() * 1000.0,
        path = %path.display(),
        "Slow sample cache phase"
    );
}

fn claim_slow_cache_phase_log(now_ms: u64) -> bool {
    let mut previous = LAST_SLOW_CACHE_PHASE_LOG_MS.load(Ordering::Relaxed);
    loop {
        if now_ms.saturating_sub(previous) < SLOW_CACHE_PHASE_LOG_INTERVAL_MS {
            return false;
        }
        match LAST_SLOW_CACHE_PHASE_LOG_MS.compare_exchange_weak(
            previous,
            now_ms,
            Ordering::Relaxed,
            Ordering::Relaxed,
        ) {
            Ok(_) => return true,
            Err(current) => previous = current,
        }
    }
}

fn now_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis()
        .try_into()
        .unwrap_or(u64::MAX)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn slow_cache_phase_log_claim_is_rate_limited() {
        LAST_SLOW_CACHE_PHASE_LOG_MS.store(1_000, Ordering::Relaxed);

        assert!(!claim_slow_cache_phase_log(1_100));
        assert!(claim_slow_cache_phase_log(1_250));
        assert!(!claim_slow_cache_phase_log(1_300));
    }
}
