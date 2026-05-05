use crate::audio::Source;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, OnceLock};

static LOOP_CYCLE_DIAGNOSTICS_ENABLED: OnceLock<bool> = OnceLock::new();

/// Diagnostic wrapper that logs when a looped source restarts
pub(crate) struct LoopDiagnostic<S> {
    inner: S,
    samples_emitted: u64,
    cycle_count: Arc<AtomicU64>,
    expected_samples_per_cycle: u64,
    channels: u16,
    diagnostics_enabled: bool,
}

impl<S> LoopDiagnostic<S> {
    pub(crate) fn new(inner: S, expected_samples_per_cycle: u64) -> Self
    where
        S: Source<Item = f32>,
    {
        let channels = inner.channels();
        Self {
            inner,
            samples_emitted: 0,
            cycle_count: Arc::new(AtomicU64::new(0)),
            expected_samples_per_cycle,
            channels,
            diagnostics_enabled: loop_cycle_diagnostics_enabled(),
        }
    }
}

impl<S> Iterator for LoopDiagnostic<S>
where
    S: Source<Item = f32>,
{
    type Item = f32;

    fn next(&mut self) -> Option<f32> {
        let sample = self.inner.next()?;
        self.samples_emitted += 1;

        // Check if we've completed a cycle
        if self.samples_emitted >= self.expected_samples_per_cycle {
            let cycle = self.cycle_count.fetch_add(1, Ordering::Relaxed);
            let frames = self.samples_emitted / self.channels as u64;
            let is_even = frames.is_multiple_of(2);

            if self.diagnostics_enabled {
                tracing::debug!(
                    "Loop cycle {} complete: emitted {} samples ({} frames), even={}, expected={}",
                    cycle,
                    self.samples_emitted,
                    frames,
                    is_even,
                    self.expected_samples_per_cycle
                );
            }

            self.samples_emitted = 0;
        }

        Some(sample)
    }
}

impl<S> Source for LoopDiagnostic<S>
where
    S: Source,
{
    fn current_frame_len(&self) -> Option<usize> {
        self.inner.current_frame_len()
    }

    fn channels(&self) -> u16 {
        self.channels
    }

    fn sample_rate(&self) -> u32 {
        self.inner.sample_rate()
    }

    fn total_duration(&self) -> Option<std::time::Duration> {
        None // Infinite loop
    }
}

fn loop_cycle_diagnostics_enabled() -> bool {
    crate::hotpath_telemetry::enabled(&LOOP_CYCLE_DIAGNOSTICS_ENABLED)
}

#[cfg(test)]
mod tests {
    use super::loop_cycle_diagnostics_enabled;

    #[test]
    fn loop_cycle_diagnostics_default_to_disabled() {
        assert!(!loop_cycle_diagnostics_enabled());
    }
}
