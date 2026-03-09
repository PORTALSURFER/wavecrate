use serde::{Deserialize, Serialize};
use thiserror::Error;

mod callback;
mod discovery;
mod stream;

pub use discovery::{available_devices, available_hosts, supported_sample_rates};
pub use stream::{CpalAudioStream, MonitorSink, OpenStreamOutcome, open_output_stream};

#[cfg(test)]
use self::callback::{CallbackState, StreamCommand, process_audio_callback};
#[cfg(test)]
use self::discovery::sample_rates_in_range;
#[cfg(test)]
use self::stream::resolved_output_from_stream_config;

/// Errors that can occur while enumerating or opening audio outputs.
#[derive(Debug, Error)]
pub enum AudioOutputError {
    /// No audio output devices are available on the host.
    #[error("No audio output devices found")]
    NoOutputDevices,
    /// Failed to enumerate output devices on the host.
    #[error("Could not list output devices: {source}")]
    ListOutputDevices {
        /// Underlying cpal error.
        source: cpal::DevicesError,
    },
    /// Failed to query supported output configs for a host.
    #[error("Failed to read supported configs for {host_id}: {source}")]
    SupportedOutputConfigs {
        /// Host identifier used for the query.
        host_id: String,
        /// Underlying cpal error.
        source: cpal::SupportedStreamConfigsError,
    },
    /// Failed to build an output stream.
    #[error("Failed to build stream: {source}")]
    BuildStream {
        /// Underlying cpal error.
        source: cpal::BuildStreamError,
    },
    /// Failed to build a default output stream.
    #[error("Failed to build default stream: {source}")]
    BuildDefaultStream {
        /// Underlying cpal error.
        source: cpal::BuildStreamError,
    },
    /// Failed to start playback on an output stream.
    #[error("Playback failed to start: {source}")]
    PlayStream {
        /// Underlying cpal error.
        source: cpal::PlayStreamError,
    },
    /// Failed to resolve the default output config for a host.
    #[error("Default config error for {host_id}: {source}")]
    DefaultConfig {
        /// Host identifier used for the query.
        host_id: String,
        /// Underlying cpal error.
        source: cpal::DefaultStreamConfigError,
    },
}

/// Persisted audio output preferences chosen by the user.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
pub struct AudioOutputConfig {
    /// Preferred host identifier (e.g., "wasapi").
    #[serde(default)]
    pub host: Option<String>,
    /// Preferred device name.
    #[serde(default)]
    pub device: Option<String>,
    /// Preferred sample rate in Hz.
    #[serde(default)]
    pub sample_rate: Option<u32>,
    /// Preferred buffer size in frames.
    #[serde(default)]
    pub buffer_size: Option<u32>,
}

/// Available audio host (backend) presented to the user.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AudioHostSummary {
    /// Host identifier used by cpal.
    pub id: String,
    /// Human-readable display label.
    pub label: String,
    /// Whether this host is the system default.
    pub is_default: bool,
}

/// Available device on a specific audio host.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AudioDeviceSummary {
    /// Host identifier that owns the device.
    pub host_id: String,
    /// Human-readable device name.
    pub name: String,
    /// Whether this device is the host default.
    pub is_default: bool,
}

/// Actual output parameters in use after opening an audio stream.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ResolvedOutput {
    /// Host identifier used to open the stream.
    pub host_id: String,
    /// Human-readable device name.
    pub device_name: String,
    /// Sample rate in Hz.
    pub sample_rate: u32,
    /// Buffer size in frames, if configurable.
    pub buffer_size_frames: Option<u32>,
    /// Total channel count provided by the device.
    pub channel_count: u16,
    /// Whether a fallback device/config was chosen.
    pub used_fallback: bool,
}

impl Default for ResolvedOutput {
    fn default() -> Self {
        Self {
            host_id: "default".into(),
            device_name: "default".into(),
            sample_rate: 44_100,
            buffer_size_frames: None,
            channel_count: 2,
            used_fallback: false,
        }
    }
}

/// Unit tests for audio output callback and stream config behavior.
#[cfg(test)]
#[path = "../../../tests/unit/audio_output_tests.rs"]
mod tests;
