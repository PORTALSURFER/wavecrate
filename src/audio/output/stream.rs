use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use cpal::{FromSample, SizedSample};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicU32, AtomicU64, AtomicUsize, Ordering};
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
    command_generation: Arc<AtomicU64>,
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
    pub fn append_source<S: crate::audio::Source + Send + 'static>(
        &self,
        source: S,
        volume: f32,
    ) -> Result<(), String> {
        let generation = self.command_generation.load(Ordering::Acquire);
        self.command_sender
            .try_send(StreamCommand::Append {
                generation,
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
        request_clear(
            &self.command_sender,
            &self.clear_pending,
            &self.command_generation,
        )
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
    pub fn append<S: crate::audio::Source + Send + 'static>(&self, source: S) {
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
    command_generation: Arc<AtomicU64>,
}

struct ResolvedOutputStreamConfig {
    stream_config: cpal::StreamConfig,
    sample_format: cpal::SampleFormat,
    used_fallback: bool,
}

/// Open an audio stream honoring user preferences with safe fallbacks.
///
/// On test builds, set `WAVECRATE_TEST_AUDIO_OUTPUT=1` to exercise real output
/// devices; otherwise the function returns `NoOutputDevices` to keep automated
/// test runs deterministic on hosts without stable audio hardware.
pub fn open_output_stream(
    config: &AudioOutputConfig,
) -> Result<OpenStreamOutcome, AudioOutputError> {
    #[cfg(test)]
    {
        if !crate::env_flags::env_var_truthy("WAVECRATE_TEST_AUDIO_OUTPUT") {
            return Err(AudioOutputError::NoOutputDevices);
        }
    }
    let (host, host_id, host_fallback) = resolve_host(config.host.as_deref())?;
    let (device, device_name, device_fallback) = resolve_device(&host, config.device.as_deref())?;

    let mut used_fallback = host_fallback || device_fallback;
    let resolved_config = resolve_output_stream_config(&device, &host_id, config)?;
    used_fallback |= resolved_config.used_fallback;
    let mut resolved_host_id = host_id;
    let mut resolved_device_name = device_name;

    let active_sources = Arc::new(AtomicUsize::new(0));
    let volume_bits = Arc::new(AtomicU32::new(1.0_f32.to_bits()));
    let clear_pending = Arc::new(AtomicBool::new(false));
    let command_generation = Arc::new(AtomicU64::new(1));

    let mut resolved_stream_config = resolved_config.stream_config.clone();
    let BuiltStreamState {
        stream,
        command_sender,
        error_receiver,
        clear_pending,
        command_generation,
    } = match build_stream_with_state(
        &device,
        &resolved_config.stream_config,
        resolved_config.sample_format,
        volume_bits.clone(),
        active_sources.clone(),
        clear_pending.clone(),
        command_generation.clone(),
    ) {
        Ok(stream) => stream,
        Err(err) => {
            if config.host.is_some() {
                return Err(AudioOutputError::BuildStream { source: err });
            }
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

            let fallback_config = resolve_output_stream_config(
                &fallback_device,
                &resolved_host_id,
                &AudioOutputConfig::default(),
            )?;
            resolved_stream_config = fallback_config.stream_config.clone();

            build_stream_with_state(
                &fallback_device,
                &fallback_config.stream_config,
                fallback_config.sample_format,
                volume_bits.clone(),
                active_sources.clone(),
                clear_pending.clone(),
                command_generation.clone(),
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
            command_generation,
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

fn resolve_output_stream_config(
    device: &cpal::Device,
    host_id: &str,
    config: &AudioOutputConfig,
) -> Result<ResolvedOutputStreamConfig, AudioOutputError> {
    let default_config =
        device
            .default_output_config()
            .map_err(|source| AudioOutputError::DefaultConfig {
                host_id: host_id.to_string(),
                source,
            })?;
    let supported: Vec<_> = device
        .supported_output_configs()
        .map_err(|source| AudioOutputError::SupportedOutputConfigs {
            host_id: host_id.to_string(),
            source,
        })?
        .collect();
    let mut used_fallback = false;
    let (stream_config, sample_format) =
        pick_output_stream_config(&default_config, &supported, config, &mut used_fallback);
    Ok(ResolvedOutputStreamConfig {
        stream_config,
        sample_format,
        used_fallback,
    })
}

fn pick_output_stream_config(
    default_config: &cpal::SupportedStreamConfig,
    supported: &[cpal::SupportedStreamConfigRange],
    config: &AudioOutputConfig,
    used_fallback: &mut bool,
) -> (cpal::StreamConfig, cpal::SampleFormat) {
    let default_rate = default_config.sample_rate();
    let requested_rate = config.sample_rate;
    let default_channels = default_config.channels();
    let default_format = default_config.sample_format();
    let matching_channels: Vec<&cpal::SupportedStreamConfigRange> = supported
        .iter()
        .filter(|range| range.channels() == default_channels)
        .collect();
    let channel_ranges = if matching_channels.is_empty() {
        if !supported.is_empty() {
            *used_fallback = true;
        }
        supported.iter().collect()
    } else {
        matching_channels
    };
    let matching_format: Vec<&cpal::SupportedStreamConfigRange> = channel_ranges
        .iter()
        .copied()
        .filter(|range| range.sample_format() == default_format)
        .collect();
    let ranges = if matching_format.is_empty() {
        if !channel_ranges.is_empty() {
            *used_fallback = true;
        }
        channel_ranges
    } else {
        matching_format
    };

    let (range, sample_rate) = if ranges.is_empty() {
        let mut stream_config = default_config.config();
        apply_output_buffer_size(&mut stream_config, None, config.buffer_size, used_fallback);
        return (stream_config, default_format);
    } else {
        choose_output_range_and_rate(&ranges, requested_rate, default_rate, used_fallback)
    };

    let mut stream_config = range.with_sample_rate(sample_rate).config();
    apply_output_buffer_size(
        &mut stream_config,
        Some(range.buffer_size()),
        config.buffer_size,
        used_fallback,
    );
    (stream_config, range.sample_format())
}

fn choose_output_range_and_rate<'a>(
    ranges: &'a [&'a cpal::SupportedStreamConfigRange],
    requested_rate: Option<u32>,
    default_rate: u32,
    used_fallback: &mut bool,
) -> (&'a cpal::SupportedStreamConfigRange, u32) {
    if let Some(rate) = requested_rate {
        if let Some(range) = ranges
            .iter()
            .find(|range| output_rate_in_range(rate, range))
        {
            return (*range, rate);
        }
        *used_fallback = true;
    }
    if let Some(range) = ranges
        .iter()
        .find(|range| output_rate_in_range(default_rate, range))
    {
        return (*range, default_rate);
    }
    *used_fallback = true;
    let range = ranges[0];
    (range, range.max_sample_rate())
}

fn output_rate_in_range(rate: u32, range: &cpal::SupportedStreamConfigRange) -> bool {
    rate >= range.min_sample_rate() && rate <= range.max_sample_rate()
}

fn apply_output_buffer_size(
    stream_config: &mut cpal::StreamConfig,
    supported: Option<&cpal::SupportedBufferSize>,
    requested_size: Option<u32>,
    used_fallback: &mut bool,
) {
    let Some(size) = requested_size.filter(|size| *size > 0) else {
        return;
    };
    if supported.is_some_and(|supported| output_buffer_size_supported(supported, size)) {
        stream_config.buffer_size = cpal::BufferSize::Fixed(size);
    } else {
        *used_fallback = true;
        stream_config.buffer_size = cpal::BufferSize::Default;
    }
}

fn output_buffer_size_supported(supported: &cpal::SupportedBufferSize, size: u32) -> bool {
    match supported {
        cpal::SupportedBufferSize::Range { min, max } => size >= *min && size <= *max,
        cpal::SupportedBufferSize::Unknown => false,
    }
}

fn build_stream_with_state(
    device: &cpal::Device,
    stream_config: &cpal::StreamConfig,
    sample_format: cpal::SampleFormat,
    volume_bits: Arc<AtomicU32>,
    active_sources: Arc<AtomicUsize>,
    clear_pending: Arc<AtomicBool>,
    command_generation: Arc<AtomicU64>,
) -> Result<BuiltStreamState, cpal::BuildStreamError> {
    const COMMAND_QUEUE_CAPACITY: usize = 512;
    let (command_sender, command_receiver) = mpsc::sync_channel(COMMAND_QUEUE_CAPACITY);
    let (error_sender, error_receiver) = mpsc::channel();
    let callback_state = CallbackState::new(
        command_receiver,
        error_sender,
        volume_bits,
        active_sources,
        clear_pending.clone(),
        command_generation.clone(),
    );
    let stream = match sample_format {
        cpal::SampleFormat::F32 => {
            build_typed_output_stream::<f32>(device, stream_config, callback_state)?
        }
        cpal::SampleFormat::F64 => {
            build_typed_output_stream::<f64>(device, stream_config, callback_state)?
        }
        cpal::SampleFormat::I8 => {
            build_typed_output_stream::<i8>(device, stream_config, callback_state)?
        }
        cpal::SampleFormat::I16 => {
            build_typed_output_stream::<i16>(device, stream_config, callback_state)?
        }
        cpal::SampleFormat::I24 => {
            build_typed_output_stream::<cpal::I24>(device, stream_config, callback_state)?
        }
        cpal::SampleFormat::I32 => {
            build_typed_output_stream::<i32>(device, stream_config, callback_state)?
        }
        cpal::SampleFormat::I64 => {
            build_typed_output_stream::<i64>(device, stream_config, callback_state)?
        }
        cpal::SampleFormat::U8 => {
            build_typed_output_stream::<u8>(device, stream_config, callback_state)?
        }
        cpal::SampleFormat::U16 => {
            build_typed_output_stream::<u16>(device, stream_config, callback_state)?
        }
        cpal::SampleFormat::U24 => {
            build_typed_output_stream::<cpal::U24>(device, stream_config, callback_state)?
        }
        cpal::SampleFormat::U32 => {
            build_typed_output_stream::<u32>(device, stream_config, callback_state)?
        }
        cpal::SampleFormat::U64 => {
            build_typed_output_stream::<u64>(device, stream_config, callback_state)?
        }
        format => {
            warn!("Unsupported output sample format {format:?}; trying f32 stream");
            build_typed_output_stream::<f32>(device, stream_config, callback_state)?
        }
    };
    Ok(BuiltStreamState {
        stream,
        command_sender,
        error_receiver,
        clear_pending,
        command_generation,
    })
}

fn build_typed_output_stream<T>(
    device: &cpal::Device,
    stream_config: &cpal::StreamConfig,
    mut callback_state: CallbackState,
) -> Result<cpal::Stream, cpal::BuildStreamError>
where
    T: SizedSample + FromSample<f32>,
{
    let mut scratch = Vec::new();
    device.build_output_stream(
        stream_config,
        move |data: &mut [T], _: &cpal::OutputCallbackInfo| {
            scratch.resize(data.len(), 0.0);
            process_audio_callback(&mut callback_state, &mut scratch);
            for (sample_out, sample_in) in data.iter_mut().zip(scratch.iter().copied()) {
                *sample_out = T::from_sample(sample_in.clamp(-1.0, 1.0));
            }
        },
        |err| tracing::error!("Stream error: {}", err),
        None,
    )
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

#[cfg(test)]
mod stream_config_tests {
    use super::*;
    use cpal::{SampleFormat, SupportedBufferSize, SupportedStreamConfigRange};

    fn range(
        channels: u16,
        min_rate: u32,
        max_rate: u32,
        buffer: SupportedBufferSize,
        format: SampleFormat,
    ) -> SupportedStreamConfigRange {
        SupportedStreamConfigRange::new(channels, min_rate, max_rate, buffer, format)
    }

    #[test]
    fn requested_output_rate_falls_back_to_supported_default_rate() {
        let default = range(
            2,
            44_100,
            48_000,
            SupportedBufferSize::Range {
                min: 128,
                max: 1024,
            },
            SampleFormat::F32,
        )
        .with_sample_rate(48_000);
        let supported = vec![range(
            2,
            44_100,
            48_000,
            SupportedBufferSize::Range {
                min: 128,
                max: 1024,
            },
            SampleFormat::F32,
        )];
        let mut used_fallback = false;
        let (config, format) = pick_output_stream_config(
            &default,
            &supported,
            &AudioOutputConfig {
                sample_rate: Some(96_000),
                ..AudioOutputConfig::default()
            },
            &mut used_fallback,
        );

        assert_eq!(config.sample_rate, 48_000);
        assert_eq!(format, SampleFormat::F32);
        assert!(used_fallback);
    }

    #[test]
    fn unsupported_output_buffer_uses_driver_default() {
        let default = range(
            2,
            44_100,
            48_000,
            SupportedBufferSize::Range { min: 128, max: 256 },
            SampleFormat::F32,
        )
        .with_sample_rate(48_000);
        let supported = vec![range(
            2,
            44_100,
            48_000,
            SupportedBufferSize::Range { min: 128, max: 256 },
            SampleFormat::F32,
        )];
        let mut used_fallback = false;
        let (config, _format) = pick_output_stream_config(
            &default,
            &supported,
            &AudioOutputConfig {
                buffer_size: Some(512),
                ..AudioOutputConfig::default()
            },
            &mut used_fallback,
        );

        assert_eq!(config.buffer_size, cpal::BufferSize::Default);
        assert!(used_fallback);
    }

    #[test]
    fn output_config_uses_supported_non_f32_sample_format() {
        let default = range(
            2,
            48_000,
            48_000,
            SupportedBufferSize::Range { min: 64, max: 512 },
            SampleFormat::I32,
        )
        .with_sample_rate(48_000);
        let supported = vec![range(
            2,
            48_000,
            48_000,
            SupportedBufferSize::Range { min: 64, max: 512 },
            SampleFormat::I32,
        )];
        let mut used_fallback = false;
        let (_config, format) = pick_output_stream_config(
            &default,
            &supported,
            &AudioOutputConfig::default(),
            &mut used_fallback,
        );

        assert_eq!(format, SampleFormat::I32);
        assert!(!used_fallback);
    }
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
