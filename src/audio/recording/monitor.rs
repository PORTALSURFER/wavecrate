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
    /// Stop the monitor worker.
    Stop,
}

/// Optional live monitor that replays captured samples.
pub struct InputMonitor {
    sender: Sender<MonitorCommand>,
    join: Option<JoinHandle<()>>,
}

impl InputMonitor {
    /// Start a monitoring worker that forwards samples into a sink.
    pub fn start(sink: MonitorSink, channels: u16, sample_rate: u32) -> Self {
        let (sender, receiver) = std::sync::mpsc::channel();
        let join = thread::spawn(move || monitor_loop(sink, channels, sample_rate, receiver));
        Self {
            sender,
            join: Some(join),
        }
    }

    /// Return a sender for pushing monitor commands.
    pub(crate) fn sender(&self) -> Sender<MonitorCommand> {
        self.sender.clone()
    }

    /// Stop the monitor worker and wait for the thread to exit.
    pub fn stop(mut self) {
        let _ = self.sender.send(MonitorCommand::Stop);
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
            MonitorCommand::Stop => break,
        }
    }
    sink.stop();
}
