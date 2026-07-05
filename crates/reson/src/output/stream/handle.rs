use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicU32, AtomicU64, AtomicUsize, Ordering};
use std::sync::mpsc::{Receiver, SyncSender, TrySendError};
use std::time::Instant;

use crate::telemetry;
use tracing::warn;

use crate::output::callback::{StreamCommand, sanitize_gain, store_volume};

/// Custom container for cpal output stream.
pub struct CpalAudioStream {
    _stream: cpal::Stream,
    command_sender: SyncSender<StreamCommand>,
    active_sources: Arc<AtomicUsize>,
    volume_bits: Arc<AtomicU32>,
    error_receiver: Receiver<String>,
    clear_pending: Arc<AtomicBool>,
    command_generation: Arc<AtomicU64>,
}

impl CpalAudioStream {
    /// Wrap a cpal stream with shared playback state.
    pub(super) fn new(
        stream: cpal::Stream,
        command_sender: SyncSender<StreamCommand>,
        active_sources: Arc<AtomicUsize>,
        volume_bits: Arc<AtomicU32>,
        error_receiver: Receiver<String>,
        clear_pending: Arc<AtomicBool>,
        command_generation: Arc<AtomicU64>,
    ) -> Self {
        Self {
            _stream: stream,
            command_sender,
            active_sources,
            volume_bits,
            error_receiver,
            clear_pending,
            command_generation,
        }
    }

    /// Append a new source into the playback queue.
    ///
    /// Returns an error when the bounded command queue is full to avoid blocking
    /// the calling thread.
    pub fn append_source<S: crate::Source + Send + 'static>(
        &self,
        source: S,
        volume: f32,
    ) -> Result<(), String> {
        let started_at = telemetry::playback_telemetry_enabled().then(Instant::now);
        let generation = self.command_generation.load(Ordering::Acquire);
        let result = self
            .command_sender
            .try_send(StreamCommand::Append {
                generation,
                source: Box::new(source),
                volume,
            })
            .map_err(|err| match err {
                TrySendError::Full(_) => "Audio command queue full; dropping source".to_string(),
                TrySendError::Disconnected(_) => "Audio output stream is unavailable".to_string(),
            });
        log_stream_command_timing("append", started_at, result.is_ok());
        result
    }

    /// Replace queued sources with a new source using one callback command.
    ///
    /// This is cheaper than sending a clear command followed by an append when
    /// callers need immediate replacement, such as rapid preview audition.
    pub fn append_source_replacing_previous<S: crate::Source + Send + 'static>(
        &self,
        source: S,
        volume: f32,
    ) -> Result<(), String> {
        let started_at = telemetry::playback_telemetry_enabled().then(Instant::now);
        let generation = self.command_generation.fetch_add(1, Ordering::AcqRel) + 1;
        let result = self
            .command_sender
            .try_send(StreamCommand::Append {
                generation,
                source: Box::new(source),
                volume,
            })
            .map_err(|err| match err {
                TrySendError::Full(_) => {
                    self.clear_pending.store(true, Ordering::Release);
                    "Audio command queue full; dropping replacement source with clear pending"
                        .to_string()
                }
                TrySendError::Disconnected(_) => "Audio output stream is unavailable".to_string(),
            });
        log_stream_command_timing("replace_append", started_at, result.is_ok());
        result
    }

    /// Clear all queued sources on the audio thread.
    ///
    /// If the command queue is full, a pending clear is still recorded so the
    /// callback can apply it without blocking.
    pub fn clear_sources(&self) -> Result<(), String> {
        let started_at = telemetry::playback_telemetry_enabled().then(Instant::now);
        let result = request_clear(
            &self.command_sender,
            &self.clear_pending,
            &self.command_generation,
        )
        .map_err(|err| match err {
            TrySendError::Full(_) => "Audio command queue full; clear pending".to_string(),
            TrySendError::Disconnected(_) => "Audio output stream is unavailable".to_string(),
        });
        log_stream_command_timing("clear", started_at, result.is_ok());
        result
    }

    /// Update the master output volume used by the audio callback.
    pub fn set_volume(&self, volume: f32) {
        store_volume(&self.volume_bits, sanitize_gain(volume));
    }

    /// Return the last known count of active sources.
    pub fn active_source_count(&self) -> usize {
        self.active_sources.load(Ordering::Relaxed)
    }

    /// Return and clear the most recent audio error, if any.
    pub fn take_error(&self) -> Option<String> {
        self.error_receiver.try_iter().last()
    }

    /// Create a monitor sink that sends sources into this stream.
    pub fn monitor_sink(&self, volume: f32) -> MonitorSink {
        MonitorSink {
            command_sender: self.command_sender.clone(),
            clear_pending: self.clear_pending.clone(),
            command_generation: self.command_generation.clone(),
            volume,
        }
    }
}

/// A bridge for input monitoring that mimics a Sink-like interface.
pub struct MonitorSink {
    command_sender: SyncSender<StreamCommand>,
    clear_pending: Arc<AtomicBool>,
    command_generation: Arc<AtomicU64>,
    /// Gain applied to appended sources.
    pub volume: f32,
}

impl MonitorSink {
    /// Append a new source into the monitored stream.
    ///
    /// Dropped commands are logged when the bounded queue is full.
    pub fn append<S: crate::Source + Send + 'static>(&self, source: S) {
        let generation = self.command_generation.load(Ordering::Acquire);
        if let Err(err) = self.command_sender.try_send(StreamCommand::Append {
            generation,
            source: Box::new(source),
            volume: self.volume,
        }) {
            match err {
                TrySendError::Full(_) => {
                    warn!("Failed to append monitor source: command queue full");
                }
                TrySendError::Disconnected(_) => {
                    warn!("Failed to append monitor source: output stream unavailable");
                }
            }
        }
    }

    /// Begin playback (no-op for the monitor sink).
    pub fn play(&self) {}

    /// Stop playback by clearing queued sources.
    pub fn stop(&self) {
        if let Err(err) = request_clear(
            &self.command_sender,
            &self.clear_pending,
            &self.command_generation,
        ) {
            match err {
                TrySendError::Full(_) => {
                    warn!("Failed to stop monitor sink: command queue full");
                }
                TrySendError::Disconnected(_) => {
                    warn!("Failed to stop monitor sink: output stream unavailable");
                }
            }
        }
    }
}

fn request_clear(
    command_sender: &SyncSender<StreamCommand>,
    clear_pending: &AtomicBool,
    command_generation: &AtomicU64,
) -> Result<(), TrySendError<StreamCommand>> {
    let generation = command_generation.fetch_add(1, Ordering::AcqRel) + 1;
    clear_pending.store(true, Ordering::Release);
    command_sender.try_send(StreamCommand::Clear { generation })
}

fn log_stream_command_timing(command: &'static str, started_at: Option<Instant>, submitted: bool) {
    let Some(started_at) = started_at else {
        return;
    };
    tracing::info!(
        target: "perf::audio_start",
        module = "reson_stream",
        stage = "stream_command_try_send",
        command,
        submitted,
        elapsed_ms = telemetry::elapsed_ms(started_at.elapsed()),
        "Audio stream command stage"
    );
}

#[cfg(test)]
pub(crate) fn monitor_sink_for_tests(
    command_sender: SyncSender<StreamCommand>,
    clear_pending: Arc<AtomicBool>,
    command_generation: Arc<AtomicU64>,
) -> MonitorSink {
    MonitorSink {
        command_sender,
        clear_pending,
        command_generation,
        volume: 1.0,
    }
}
