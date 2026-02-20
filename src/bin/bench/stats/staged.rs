use super::{LatencySummary, summarize};
use serde::Serialize;
use std::time::Instant;

/// Stage-level latency summaries for one benchmark action.
#[derive(Clone, Debug, Serialize)]
pub(crate) struct StageLatencyBreakdown {
    /// Latency summary for input/step selection work.
    pub(crate) input_stage: LatencySummary,
    /// Latency summary for action-apply mutation work.
    pub(crate) apply_stage: LatencySummary,
    /// Latency summary for model-pull preparation work.
    pub(crate) pull_stage: LatencySummary,
    /// Latency summary for projection work.
    pub(crate) projection_stage: LatencySummary,
}

/// Combined total + stage-attributed latency summaries.
#[derive(Clone, Debug, Serialize)]
pub(crate) struct StagedLatencySummary {
    /// Aggregate latency across all stages in each measured iteration.
    pub(crate) total: LatencySummary,
    /// Stage-level attribution for the same measured iterations.
    pub(crate) stages: StageLatencyBreakdown,
}

/// Per-iteration stage timer used by staged benchmark actions.
#[derive(Clone, Debug)]
pub(crate) struct StageTimer {
    started: Instant,
    stage_anchor: Instant,
    input_us: u64,
    apply_us: u64,
    pull_us: u64,
}

#[derive(Clone, Copy, Debug)]
struct StageLatencySample {
    total_us: u64,
    input_us: u64,
    apply_us: u64,
    pull_us: u64,
    projection_us: u64,
}

impl StageTimer {
    /// Start a new staged benchmark timer.
    pub(crate) fn start() -> Self {
        let now = Instant::now();
        Self {
            started: now,
            stage_anchor: now,
            input_us: 0,
            apply_us: 0,
            pull_us: 0,
        }
    }

    /// Mark completion of input/step-selection work.
    pub(crate) fn mark_input_done(&mut self) {
        self.input_us = self.elapsed_since_anchor_us();
    }

    /// Mark completion of action-apply mutation work.
    pub(crate) fn mark_apply_done(&mut self) {
        self.apply_us = self.elapsed_since_anchor_us();
    }

    /// Mark completion of model-pull preparation work.
    pub(crate) fn mark_pull_done(&mut self) {
        self.pull_us = self.elapsed_since_anchor_us();
    }

    fn elapsed_since_anchor_us(&mut self) -> u64 {
        let now = Instant::now();
        let elapsed_us = now.saturating_duration_since(self.stage_anchor).as_micros() as u64;
        self.stage_anchor = now;
        elapsed_us
    }

    fn finish(mut self) -> StageLatencySample {
        let projection_us = self.elapsed_since_anchor_us();
        let total_us = self.started.elapsed().as_micros() as u64;
        StageLatencySample {
            total_us,
            input_us: self.input_us,
            apply_us: self.apply_us,
            pull_us: self.pull_us,
            projection_us,
        }
    }
}

/// Measure benchmark actions and return total + stage-attributed summaries.
pub(crate) fn bench_staged_action_with_iters(
    warmup_iters: usize,
    measure_iters: usize,
    mut f: impl FnMut(&mut StageTimer) -> Result<(), String>,
) -> Result<StagedLatencySummary, String> {
    run_warmup_staged_action(warmup_iters, &mut f)?;
    let samples = run_measure_staged_action(measure_iters, &mut f)?;
    Ok(summarize_staged(warmup_iters, measure_iters, samples))
}

fn run_warmup_staged_action(
    warmup_iters: usize,
    f: &mut impl FnMut(&mut StageTimer) -> Result<(), String>,
) -> Result<(), String> {
    for _ in 0..warmup_iters.max(1) {
        let mut timer = StageTimer::start();
        f(&mut timer).map_err(|err| format!("Warmup action failed: {err}"))?;
    }
    Ok(())
}

fn run_measure_staged_action(
    measure_iters: usize,
    f: &mut impl FnMut(&mut StageTimer) -> Result<(), String>,
) -> Result<Vec<StageLatencySample>, String> {
    let mut samples = Vec::with_capacity(measure_iters.max(1));
    for _ in 0..measure_iters.max(1) {
        let mut timer = StageTimer::start();
        f(&mut timer).map_err(|err| format!("Measured action failed: {err}"))?;
        samples.push(timer.finish());
    }
    Ok(samples)
}

fn summarize_staged(
    warmup_iters: usize,
    measure_iters: usize,
    samples: Vec<StageLatencySample>,
) -> StagedLatencySummary {
    let total_us = samples.iter().map(|sample| sample.total_us).collect();
    let input_us = samples.iter().map(|sample| sample.input_us).collect();
    let apply_us = samples.iter().map(|sample| sample.apply_us).collect();
    let pull_us = samples.iter().map(|sample| sample.pull_us).collect();
    let projection_us = samples.iter().map(|sample| sample.projection_us).collect();
    StagedLatencySummary {
        total: summarize(warmup_iters, measure_iters, total_us),
        stages: StageLatencyBreakdown {
            input_stage: summarize(warmup_iters, measure_iters, input_us),
            apply_stage: summarize(warmup_iters, measure_iters, apply_us),
            pull_stage: summarize(warmup_iters, measure_iters, pull_us),
            projection_stage: summarize(warmup_iters, measure_iters, projection_us),
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Ensure staged benchmarks produce total and per-stage summaries.
    #[test]
    fn bench_staged_action_with_iters_reports_stage_summaries() {
        let staged = bench_staged_action_with_iters(1, 2, |timer| {
            timer.mark_input_done();
            timer.mark_apply_done();
            timer.mark_pull_done();
            Ok(())
        })
        .expect("staged bench action should succeed");
        assert_eq!(staged.total.measure_iters, 2);
        assert_eq!(staged.stages.input_stage.measure_iters, 2);
        assert_eq!(staged.stages.apply_stage.measure_iters, 2);
        assert_eq!(staged.stages.pull_stage.measure_iters, 2);
        assert_eq!(staged.stages.projection_stage.measure_iters, 2);
    }
}
