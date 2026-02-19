use super::options::BenchOptions;
use serde::Serialize;
use std::time::Instant;

#[derive(Clone, Debug, Serialize)]
pub(super) struct LatencySummary {
    /// Warmup iteration count used for each benchmark action.
    pub(super) warmup_iters: usize,
    /// Measured iteration count used for each benchmark action.
    pub(super) measure_iters: usize,
    /// Minimum sampled latency in microseconds.
    pub(super) min_us: u64,
    /// 50th percentile (median) in microseconds.
    pub(super) p50_us: u64,
    /// 95th percentile in microseconds.
    pub(super) p95_us: u64,
    /// Maximum sampled latency in microseconds.
    pub(super) max_us: u64,
    /// Mean latency in microseconds.
    pub(super) mean_us: f64,
}

/// Measure benchmark actions and return a latency summary.
pub(super) fn bench_action(
    options: &BenchOptions,
    mut f: impl FnMut() -> Result<(), String>,
) -> Result<LatencySummary, String> {
    run_warmup_action(options.warmup_iters, &mut f)?;
    let samples_us = run_measure_action(options.measure_iters, &mut f)?;
    Ok(summarize(
        options.warmup_iters,
        options.measure_iters,
        samples_us,
    ))
}

fn run_warmup_action(
    warmup_iters: usize,
    f: &mut impl FnMut() -> Result<(), String>,
) -> Result<(), String> {
    for _ in 0..warmup_iters.max(1) {
        f().map_err(|err| format!("Warmup action failed: {err}"))?;
    }
    Ok(())
}

fn run_measure_action(
    measure_iters: usize,
    f: &mut impl FnMut() -> Result<(), String>,
) -> Result<Vec<u64>, String> {
    let mut samples_us = Vec::with_capacity(measure_iters.max(1));
    for _ in 0..measure_iters.max(1) {
        let started = Instant::now();
        f().map_err(|err| format!("Measured action failed: {err}"))?;
        samples_us.push(started.elapsed().as_micros() as u64);
    }
    samples_us.sort_unstable();
    Ok(samples_us)
}

fn summarize(warmup_iters: usize, measure_iters: usize, samples_us: Vec<u64>) -> LatencySummary {
    let mut samples_us = samples_us;
    samples_us.sort_unstable();
    let min_us = *samples_us.first().unwrap_or(&0);
    let max_us = *samples_us.last().unwrap_or(&0);
    let p50_us = percentile(&samples_us, 0.50);
    let p95_us = percentile(&samples_us, 0.95);
    let mean_us = if samples_us.is_empty() {
        0.0
    } else {
        samples_us.iter().copied().map(|v| v as f64).sum::<f64>() / samples_us.len() as f64
    };
    LatencySummary {
        warmup_iters,
        measure_iters,
        min_us,
        p50_us,
        p95_us,
        max_us,
        mean_us,
    }
}

fn percentile(sorted: &[u64], p: f64) -> u64 {
    if sorted.is_empty() {
        return 0;
    }
    let idx = (sorted.len() as f64 * p.clamp(0.0, 1.0)).ceil() as isize - 1;
    let idx = (idx.clamp(0, (sorted.len() - 1) as isize) as usize).min(sorted.len() - 1);
    sorted[idx]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn run_warmup_invokes_action_exactly() {
        let mut calls = 0usize;
        let result = run_warmup_action(3, &mut || {
            calls += 1;
            Ok(())
        });
        assert!(result.is_ok());
        assert_eq!(calls, 3);
    }

    #[test]
    fn run_measure_action_collects_timings_and_sorts() {
        let mut calls = 0usize;
        let result = run_measure_action(3, &mut || {
            calls += 1;
            Ok(())
        });
        let samples = result.expect("measured samples");
        assert_eq!(calls, 3);
        assert_eq!(samples.len(), 3);
    }

    #[test]
    fn summarize_percentile_handles_empty_and_known_data() {
        let summary = summarize(0, 4, vec![40_u64, 10_u64, 30_u64, 20_u64, 50_u64]);
        assert_eq!(summary.min_us, 10);
        assert_eq!(summary.max_us, 50);
        assert_eq!(summary.p50_us, 30);
        assert_eq!(summary.p95_us, 50);
    }

    #[test]
    fn percentile_rounds_and_clamps_indices() {
        let sorted = vec![10_u64, 20, 30, 40];
        assert_eq!(percentile(&sorted, 0.5), 20);
        assert_eq!(percentile(&sorted, 1.0), 40);
        assert_eq!(percentile(&sorted, -1.0), 10);
    }

    #[test]
    fn bench_action_reports_requested_sample_counts() {
        let mut calls = 0usize;
        let options = BenchOptions {
            warmup_iters: 2,
            measure_iters: 3,
            ..BenchOptions::default()
        };
        let summary = bench_action(&options, || {
            calls += 1;
            Ok(())
        })
        .expect("bench action");
        assert_eq!(summary.warmup_iters, 2);
        assert_eq!(summary.measure_iters, 3);
        assert_eq!(calls, 5);
    }

    #[test]
    fn bench_action_wraps_measured_action_error() {
        let mut attempts = 0usize;
        let options = BenchOptions {
            warmup_iters: 1,
            measure_iters: 2,
            ..BenchOptions::default()
        };
        let result = bench_action(&options, || {
            attempts += 1;
            if attempts > 1 {
                Err(format!("attempt {attempts} failed"))
            } else {
                Ok(())
            }
        });
        let error = result.expect_err("expected failure");
        assert!(error.contains("Measured action failed: attempt 2 failed"));
    }
}
