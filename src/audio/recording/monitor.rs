//! Live input-monitor worker plumbing for recording sessions.

use std::sync::{Arc, Mutex, mpsc::Receiver, mpsc::Sender};
use std::thread::{self, JoinHandle};

use crate::audio::SamplesBuffer;
use crate::audio::output::MonitorSink;

/// Shared slot that keeps the currently attached live monitor sender, if any.
pub(super) type MonitorSenderSlot = Arc<Mutex<Option<Sender<MonitorCommand>>>>;

/// Commands sent to the input monitor worker.
pub(crate) enum MonitorCommand {
    /// Forward live samples to the monitor sink.
    Samples(Vec<f32>),
}

/// Optional live monitor that replays captured samples.
pub struct InputMonitor {
    sender: Option<Sender<MonitorCommand>>,
    join: Option<JoinHandle<()>>,
}

impl InputMonitor {
    /// Start a monitoring worker that forwards samples into a sink.
    pub fn start(sink: MonitorSink, channels: u16, sample_rate: u32) -> Self {
        let (sender, receiver) = std::sync::mpsc::channel();
        let join = thread::spawn(move || monitor_loop(sink, channels, sample_rate, receiver));
        Self {
            sender: Some(sender),
            join: Some(join),
        }
    }

    /// Return a sender for pushing monitor commands.
    pub(crate) fn sender(&self) -> Sender<MonitorCommand> {
        self.sender
            .as_ref()
            .cloned()
            .expect("input monitor sender should exist while monitor is active")
    }

    /// Stop the monitor worker and wait for the thread to exit.
    pub fn stop(mut self) {
        let _ = self.sender.take();
        if let Some(join) = self.join.take() {
            let _ = join.join();
        }
    }
}

/// Allocate one empty monitor-sender slot for a recording session.
pub(super) fn new_monitor_sender_slot() -> MonitorSenderSlot {
    Arc::new(Mutex::new(None))
}

/// Replace the active monitor sender for one recording session.
pub(super) fn set_monitor_sender(slot: &MonitorSenderSlot, sender: Option<Sender<MonitorCommand>>) {
    if let Ok(mut guard) = slot.lock() {
        *guard = sender;
    }
}

/// Forward one captured sample block to the active monitor, if present.
pub(super) fn forward_monitor_samples(slot: &MonitorSenderSlot, samples: &[f32]) {
    if let Ok(guard) = slot.lock()
        && let Some(monitor) = guard.as_ref()
    {
        let _ = monitor.send(MonitorCommand::Samples(samples.to_vec()));
    }
}

fn monitor_loop(
    sink: MonitorSink,
    channels: u16,
    sample_rate: u32,
    receiver: Receiver<MonitorCommand>,
) {
    let channels = channels.max(1);
    let sample_rate = sample_rate.max(1);
    sink.play();
    while let Ok(command) = receiver.recv() {
        match command {
            MonitorCommand::Samples(samples) => {
                if samples.is_empty() {
                    continue;
                }
                let source = SamplesBuffer::new(channels, sample_rate, samples);
                sink.append(source);
            }
        }
    }
    sink.stop();
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::audio::output::{StreamCommand, monitor_sink_for_tests};
    use std::sync::Arc;
    use std::sync::atomic::{AtomicBool, AtomicU64};
    use std::sync::mpsc;
    use std::time::Duration;

    #[test]
    fn stop_drains_queued_samples_before_clearing_sink() {
        let (command_sender, command_receiver) = mpsc::sync_channel(8);
        let sink = monitor_sink_for_tests(
            command_sender,
            Arc::new(AtomicBool::new(false)),
            Arc::new(AtomicU64::new(1)),
        );
        let monitor = InputMonitor::start(sink, 1, 48_000);
        let sender = monitor.sender();
        sender.send(MonitorCommand::Samples(vec![0.5])).unwrap();
        drop(sender);

        monitor.stop();

        match command_receiver
            .recv_timeout(Duration::from_millis(100))
            .expect("monitor should append queued samples before stopping")
        {
            StreamCommand::Append { generation, .. } => assert_eq!(generation, 1),
            _ => panic!("expected monitor append command"),
        }
        match command_receiver
            .recv_timeout(Duration::from_millis(100))
            .expect("monitor stop should clear the sink after draining")
        {
            StreamCommand::Clear { generation } => assert_eq!(generation, 2),
            _ => panic!("expected monitor clear command"),
        }
    }
}
