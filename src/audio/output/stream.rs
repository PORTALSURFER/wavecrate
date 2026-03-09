use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicU32, AtomicUsize, Ordering};
use std::sync::mpsc::{self, Receiver, SyncSender, TrySendError};
use tracing::{info, warn};

use super::callback::{
    CallbackState, StreamCommand, process_audio_callback, sanitize_gain, store_volume,
};
use super::discovery::{resolve_device, resolve_host};
use super::{AudioOutputConfig, AudioOutputError, ResolvedOutput};
use crate::audio::device::device_label;

/// Custom container for cpal output stream.
pub struct CpalAudioStream {
    _stream: cpal::Stream,
    command_sender: SyncSender<StreamCommand>,
    active_sources: Arc<AtomicUsize>,
    volume_bits: Arc<AtomicU32>,
    error_receiver: Receiver<String>,
    clear_pending: Arc<AtomicBool>,
}

impl CpalAudioStream {
    /// Wrap a cpal stream with shared playback state.
    fn new(
        stream: cpal::Stream,
        command_sender: SyncSender<StreamCommand>,
        active_sources: Arc<AtomicUsize>,
        volume_bits: Arc<AtomicU32>,
        error_receiver: Receiver<String>,
        clear_pending: Arc<AtomicBool>,
    ) -> Self {
        Self {
            _stream: stream,
            command_sender,
            active_sources,
            volume_bits,
            error_receiver,
            clear_pending,
        }
    }

    /// Append a new source into the playback queue.
    ///
    /// Returns an error when the bounded command queue is full to avoid blocking
    /// the calling thread.
    pub fn append_source<S: crate::audio::Source + Send + 'static>(
        &self,
        source: S,
        volume: f32,
    ) -> Result<(), String> {
        self.command_sender
            .try_send(StreamCommand::Append {
                source: Box::new(source),
                volume,
            })
            .map_err(|err| match err {
                TrySendError::Full(_) => "Audio command queue full; dropping source".to_string(),
                TrySendError::Disconnected(_) => "Audio output stream is unavailable".to_string(),
            })
    }

    /// Clear all queued sources on the audio thread.
    ///
    /// If the command queue is full, a pending clear is still recorded so the
    /// callback can apply it without blocking.
    pub fn clear_sources(&self) -> Result<(), String> {
        self.clear_pending.store(true, Ordering::Relaxed);
        self.command_sender
            .try_send(StreamCommand::Clear)
            .map_err(|err| match err {
                TrySendError::Full(_) => "Audio command queue full; clear pending".to_string(),
                TrySendError::Disconnected(_) => "Audio output stream is unavailable".to_string(),
            })
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
            volume,
        }
    }
}

/// A bridge for input monitoring that mimics a Sink-like interface.
pub struct MonitorSink {
    command_sender: SyncSender<StreamCommand>,
    clear_pending: Arc<AtomicBool>,
    /// Gain applied to appended sources.
    pub volume: f32,
}

impl MonitorSink {
    /// Append a new source into the monitored stream.
    ///
    /// Dropped commands are logged when the bounded queue is full.
    pub fn append<S: crate::audio::Source + Send + 'static>(&self, source: S) {
        if let Err(err) = self.command_sender.try_send(StreamCommand::Append {
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
        self.clear_pending.store(true, Ordering::Relaxed);
        if let Err(err) = self.command_sender.try_send(StreamCommand::Clear) {
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

/// Stream creation result that keeps both the stream handle and resolved settings.
pub struct OpenStreamOutcome {
    /// Opened cpal stream with shared state.
    pub stream: CpalAudioStream,
    /// Resolved output configuration used to open the stream.
    pub resolved: ResolvedOutput,
}

/// Raw stream construction pieces returned before wrapping in `CpalAudioStream`.
struct BuiltStreamState {
    stream: cpal::Stream,
    command_sender: SyncSender<StreamCommand>,
    error_receiver: Receiver<String>,
    clear_pending: Arc<AtomicBool>,
}

/// Open an audio stream honoring user preferences with safe fallbacks.
///
/// On test builds, set `SEMPAL_TEST_AUDIO_OUTPUT=1` to exercise real output
/// devices; otherwise the function returns `NoOutputDevices` to keep automated
/// test runs deterministic on hosts without stable audio hardware.
pub fn open_output_stream(
    config: &AudioOutputConfig,
) -> Result<OpenStreamOutcome, AudioOutputError> {
    #[cfg(test)]
    {
        if !crate::env_flags::env_var_truthy("SEMPAL_TEST_AUDIO_OUTPUT") {
            return Err(AudioOutputError::NoOutputDevices);
        }
    }
    let (host, host_id, host_fallback) = resolve_host(config.host.as_deref())?;
    let (device, device_name, device_fallback) = resolve_device(&host, config.device.as_deref())?;

    let stream_config = match device.default_output_config() {
        Ok(c) => c,
        Err(err) => {
            return Err(AudioOutputError::DefaultConfig {
                host_id,
                source: err,
            });
        }
    };

    let mut stream_config: cpal::StreamConfig = stream_config.into();
    if let Some(rate) = config.sample_rate {
        stream_config.sample_rate = rate;
    }
    if let Some(size) = config.buffer_size.filter(|size| *size > 0) {
        stream_config.buffer_size = cpal::BufferSize::Fixed(size);
    }

    let mut used_fallback = host_fallback || device_fallback;
    let mut resolved_host_id = host_id;
    let mut resolved_device_name = device_name;

    let active_sources = Arc::new(AtomicUsize::new(0));
    let volume_bits = Arc::new(AtomicU32::new(1.0_f32.to_bits()));
    let clear_pending = Arc::new(AtomicBool::new(false));

    let mut resolved_stream_config = stream_config.clone();
    let BuiltStreamState {
        stream,
        command_sender,
        error_receiver,
        clear_pending,
    } = match build_stream_with_state(
        &device,
        &stream_config,
        volume_bits.clone(),
        active_sources.clone(),
        clear_pending.clone(),
    ) {
        Ok(stream) => stream,
        Err(err) => {
            used_fallback = true;
            let default_host = cpal::default_host();
            let fallback_device = default_host.default_output_device().ok_or_else(|| {
                AudioOutputError::BuildStream {
                    source: err.clone(),
                }
            })?;
            resolved_host_id = default_host.id().name().to_string();
            resolved_device_name =
                device_label(&fallback_device).unwrap_or_else(|| "Default device".to_string());

            let fallback_config = fallback_device.default_output_config().map_err(|source| {
                AudioOutputError::DefaultConfig {
                    host_id: resolved_host_id.clone(),
                    source,
                }
            })?;

            let fallback_stream_config: cpal::StreamConfig = fallback_config.into();
            resolved_stream_config = fallback_stream_config.clone();

            build_stream_with_state(
                &fallback_device,
                &fallback_stream_config,
                volume_bits.clone(),
                active_sources.clone(),
                clear_pending.clone(),
            )
            .map_err(|source| AudioOutputError::BuildDefaultStream { source })?
        }
    };

    stream
        .play()
        .map_err(|source| AudioOutputError::PlayStream { source })?;

    let resolved = resolved_output_from_stream_config(
        resolved_host_id,
        resolved_device_name,
        &resolved_stream_config,
        used_fallback,
    );
    info!(
        "Audio output ready: host={} device=\"{}\" rate={}Hz channels={} buffer={:?} fallback={}",
        resolved.host_id,
        resolved.device_name,
        resolved.sample_rate,
        resolved.channel_count,
        resolved.buffer_size_frames,
        resolved.used_fallback
    );
    Ok(OpenStreamOutcome {
        stream: CpalAudioStream::new(
            stream,
            command_sender,
            active_sources,
            volume_bits,
            error_receiver,
            clear_pending,
        ),
        resolved,
    })
}

pub(super) fn resolved_output_from_stream_config(
    host_id: String,
    device_name: String,
    stream_config: &cpal::StreamConfig,
    used_fallback: bool,
) -> ResolvedOutput {
    let applied_buffer = match stream_config.buffer_size {
        cpal::BufferSize::Default => None,
        cpal::BufferSize::Fixed(size) => Some(size),
    };
    ResolvedOutput {
        host_id,
        device_name,
        sample_rate: stream_config.sample_rate,
        buffer_size_frames: applied_buffer,
        channel_count: stream_config.channels,
        used_fallback,
    }
}

fn build_stream_with_state(
    device: &cpal::Device,
    stream_config: &cpal::StreamConfig,
    volume_bits: Arc<AtomicU32>,
    active_sources: Arc<AtomicUsize>,
    clear_pending: Arc<AtomicBool>,
) -> Result<BuiltStreamState, cpal::BuildStreamError> {
    const COMMAND_QUEUE_CAPACITY: usize = 512;
    let (command_sender, command_receiver) = mpsc::sync_channel(COMMAND_QUEUE_CAPACITY);
    let (error_sender, error_receiver) = mpsc::channel();
    let mut callback_state = CallbackState::new(
        command_receiver,
        error_sender,
        volume_bits,
        active_sources,
        clear_pending.clone(),
    );
    let callback = move |data: &mut [f32], _: &cpal::OutputCallbackInfo| {
        process_audio_callback(&mut callback_state, data);
    };
    let stream = device.build_output_stream(
        stream_config,
        callback,
        |err| tracing::error!("Stream error: {}", err),
        None,
    )?;
    Ok(BuiltStreamState {
        stream,
        command_sender,
        error_receiver,
        clear_pending,
    })
}
