//! Bounded live-input monitor transport for recording sessions.

use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{Receiver, SyncSender, TryRecvError, TrySendError};
use std::thread::{self, JoinHandle};
use std::time::Duration;

use ringbuf::{HeapRb, traits::*};

use crate::SamplesBuffer;
use crate::output::MonitorSink;

use super::health::RecordingHealthState;

const MONITOR_BUFFER_MILLISECONDS: usize = 500;
const MONITOR_DRAIN_FRAMES: usize = 1_024;
const MONITOR_CONTROL_CAPACITY: usize = 4;
const IDLE_POLL_INTERVAL: Duration = Duration::from_millis(1);

#[derive(Clone)]
pub(super) struct MonitorTarget {
    sink: MonitorSink,
    channels: u16,
    sample_rate: u32,
}

/// Optional live monitor target that can be attached to a recorder.
pub struct InputMonitor {
    target: MonitorTarget,
}

impl InputMonitor {
    /// Prepare a monitoring target that forwards captured samples into a sink.
    pub fn start(sink: MonitorSink, channels: u16, sample_rate: u32) -> Self {
        sink.play();
        Self {
            target: MonitorTarget {
                sink,
                channels: channels.max(1),
                sample_rate: sample_rate.max(1),
            },
        }
    }

    pub(super) fn target(&self) -> MonitorTarget {
        self.target.clone()
    }

    /// Stop the monitor sink.
    pub fn stop(self) {
        self.target.sink.stop();
    }
}

pub(super) struct MonitorCapture {
    producer: ringbuf::HeapProd<f32>,
    enabled: Arc<AtomicBool>,
    health: Arc<RecordingHealthState>,
}

impl MonitorCapture {
    pub(super) fn submit(&mut self, samples: &[f32]) {
        if !self.enabled.load(Ordering::Relaxed) {
            return;
        }
        if self.producer.vacant_len() < samples.len() {
            self.health
                .monitor_dropped_samples
                .fetch_add(samples.len() as u64, Ordering::Relaxed);
            self.health
                .monitor_overrun_events
                .fetch_add(1, Ordering::Relaxed);
            return;
        }
        let pushed = self.producer.push_slice(samples);
        debug_assert_eq!(pushed, samples.len());
    }
}

enum MonitorControlCommand {
    Attach(MonitorTarget),
    Detach,
}

pub(super) struct RecordingMonitor {
    control: SyncSender<MonitorControlCommand>,
    stop: Arc<AtomicBool>,
    enabled: Arc<AtomicBool>,
    join: Option<JoinHandle<()>>,
    health: Arc<RecordingHealthState>,
}

impl RecordingMonitor {
    pub(super) fn attach(&self, target: MonitorTarget) {
        if self.submit_control(MonitorControlCommand::Attach(target)) {
            self.enabled.store(true, Ordering::Release);
        }
    }

    pub(super) fn detach(&self) {
        if self.submit_control(MonitorControlCommand::Detach) {
            self.enabled.store(false, Ordering::Release);
        }
    }

    fn submit_control(&self, command: MonitorControlCommand) -> bool {
        match self.control.try_send(command) {
            Ok(()) => true,
            Err(TrySendError::Full(_)) => {
                self.health
                    .monitor_control_drops
                    .fetch_add(1, Ordering::Relaxed);
                false
            }
            Err(TrySendError::Disconnected(_)) => {
                self.health
                    .monitor_disconnected
                    .store(true, Ordering::Release);
                false
            }
        }
    }

    pub(super) fn stop(&mut self) {
        self.enabled.store(false, Ordering::Release);
        self.stop.store(true, Ordering::Release);
        if let Some(join) = self.join.take()
            && join.join().is_err()
        {
            self.health.monitor_failed.store(true, Ordering::Release);
        }
    }
}

pub(super) fn start_recording_monitor(
    sample_rate: u32,
    channels: u16,
    health: Arc<RecordingHealthState>,
) -> (RecordingMonitor, MonitorCapture) {
    let capacity = (sample_rate as usize)
        .saturating_mul(channels.max(1) as usize)
        .saturating_mul(MONITOR_BUFFER_MILLISECONDS)
        .checked_div(1_000)
        .unwrap_or(usize::MAX)
        .max(1);
    let ring = HeapRb::<f32>::new(capacity);
    let (producer, consumer) = ring.split();
    let (control, control_receiver) = std::sync::mpsc::sync_channel(MONITOR_CONTROL_CAPACITY);
    let stop = Arc::new(AtomicBool::new(false));
    let enabled = Arc::new(AtomicBool::new(false));
    let worker_stop = Arc::clone(&stop);
    let join = thread::spawn(move || {
        monitor_loop(consumer, control_receiver, channels.max(1), &worker_stop)
    });
    (
        RecordingMonitor {
            control,
            stop,
            enabled: Arc::clone(&enabled),
            join: Some(join),
            health: Arc::clone(&health),
        },
        MonitorCapture {
            producer,
            enabled,
            health,
        },
    )
}

fn monitor_loop(
    mut consumer: ringbuf::HeapCons<f32>,
    controls: Receiver<MonitorControlCommand>,
    capture_channels: u16,
    stop: &AtomicBool,
) {
    let mut target: Option<MonitorTarget> = None;
    let mut samples = vec![0.0; MONITOR_DRAIN_FRAMES.saturating_mul(capture_channels as usize)];
    loop {
        loop {
            match controls.try_recv() {
                Ok(MonitorControlCommand::Attach(next)) => target = Some(next),
                Ok(MonitorControlCommand::Detach) => target = None,
                Err(TryRecvError::Empty) => break,
                Err(TryRecvError::Disconnected) => break,
            }
        }
        let popped = consumer.pop_slice(&mut samples);
        if popped > 0 {
            if let Some(target) = &target {
                target.sink.append(SamplesBuffer::new(
                    target.channels,
                    target.sample_rate,
                    samples[..popped].to_vec(),
                ));
            }
            continue;
        }
        if stop.load(Ordering::Acquire) {
            break;
        }
        thread::sleep(IDLE_POLL_INTERVAL);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn stalled_monitor_consumer_has_bounded_nonblocking_submission() {
        let ring = HeapRb::<f32>::new(4);
        let (producer, _consumer) = ring.split();
        let health = Arc::new(RecordingHealthState::default());
        let mut capture = MonitorCapture {
            producer,
            enabled: Arc::new(AtomicBool::new(true)),
            health: Arc::clone(&health),
        };

        capture.submit(&[0.0, 0.1, 0.2, 0.3, 0.4, 0.5]);

        let snapshot = health.snapshot();
        assert_eq!(snapshot.monitor_dropped_samples, 6);
        assert_eq!(snapshot.monitor_overrun_events, 1);
    }

    #[test]
    fn monitor_control_reports_full_and_disconnected_queue() {
        let health = Arc::new(RecordingHealthState::default());
        let stop = Arc::new(AtomicBool::new(false));
        let (control, receiver) = std::sync::mpsc::sync_channel(1);
        let monitor = RecordingMonitor {
            control,
            stop,
            enabled: Arc::new(AtomicBool::new(false)),
            join: None,
            health: Arc::clone(&health),
        };

        monitor.detach();
        monitor.detach();
        assert_eq!(health.snapshot().monitor_control_drops, 1);

        drop(receiver);
        monitor.detach();
        assert!(health.snapshot().monitor_disconnected);
    }
}
