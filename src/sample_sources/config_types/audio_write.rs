use serde::{Deserialize, Serialize};

/// Persistent audio-write policy for Wavecrate-created WAV files.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct AudioWriteFormatConfig {
    /// Sample-rate policy used for new or rewritten audio files.
    #[serde(default)]
    pub sample_rate: AudioWriteSampleRate,
    /// Sample representation used for WAV writes.
    #[serde(default)]
    pub sample_format: AudioWriteSampleFormat,
    /// Channel policy used for ordinary mono/stereo writes.
    #[serde(default)]
    pub channel_behavior: AudioWriteChannelBehavior,
    /// Dither policy used when reducing to integer PCM formats.
    #[serde(default)]
    pub dither: AudioWriteDither,
}

impl AudioWriteFormatConfig {
    /// Build a WAV spec for interleaved samples already rendered at the source rate.
    pub fn wav_spec_for_source(&self, channels: u16, source_sample_rate: u32) -> hound::WavSpec {
        let (bits_per_sample, sample_format) = self.sample_format.wav_parts();
        hound::WavSpec {
            channels: self.channel_behavior.output_channels(channels),
            sample_rate: self.sample_rate.output_sample_rate(source_sample_rate),
            bits_per_sample,
            sample_format,
        }
    }

    /// Concise settings label for options panels and warnings.
    pub fn summary_label(&self) -> String {
        format!(
            "{}, {}, {}, {}",
            self.sample_rate.label(),
            self.sample_format.label(),
            self.channel_behavior.label(),
            self.dither.label()
        )
    }
}

impl Default for AudioWriteFormatConfig {
    fn default() -> Self {
        Self {
            sample_rate: AudioWriteSampleRate::Source,
            sample_format: AudioWriteSampleFormat::Float32,
            channel_behavior: AudioWriteChannelBehavior::PreserveMonoStereo,
            dither: AudioWriteDither::None,
        }
    }
}

/// Sample-rate policy used when Wavecrate writes audio.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum AudioWriteSampleRate {
    /// Preserve the loaded/rendered source rate.
    Source,
    /// Write at a fixed sample rate. Resampling support is staged separately.
    Hz(u32),
}

impl AudioWriteSampleRate {
    fn output_sample_rate(&self, source_sample_rate: u32) -> u32 {
        match self {
            Self::Source => source_sample_rate.max(1),
            Self::Hz(sample_rate) => (*sample_rate).max(1),
        }
    }

    fn label(&self) -> String {
        match self {
            Self::Source => String::from("Source rate"),
            Self::Hz(sample_rate) => format_sample_rate_label(*sample_rate),
        }
    }
}

impl Default for AudioWriteSampleRate {
    fn default() -> Self {
        Self::Source
    }
}

/// WAV sample encoding used when Wavecrate creates files.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum AudioWriteSampleFormat {
    /// 16-bit signed integer PCM.
    Pcm16,
    /// 24-bit signed integer PCM.
    Pcm24,
    /// 32-bit IEEE float.
    Float32,
}

impl AudioWriteSampleFormat {
    pub(crate) fn wav_parts(&self) -> (u16, hound::SampleFormat) {
        match self {
            Self::Pcm16 => (16, hound::SampleFormat::Int),
            Self::Pcm24 => (24, hound::SampleFormat::Int),
            Self::Float32 => (32, hound::SampleFormat::Float),
        }
    }

    fn label(&self) -> &'static str {
        match self {
            Self::Pcm16 => "16-bit PCM",
            Self::Pcm24 => "24-bit PCM",
            Self::Float32 => "32-bit float",
        }
    }
}

impl Default for AudioWriteSampleFormat {
    fn default() -> Self {
        Self::Float32
    }
}

/// Channel policy for normal Wavecrate-created audio.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum AudioWriteChannelBehavior {
    /// Preserve mono/stereo source layout for ordinary writes.
    PreserveMonoStereo,
}

impl AudioWriteChannelBehavior {
    fn output_channels(&self, channels: u16) -> u16 {
        match self {
            Self::PreserveMonoStereo => channels.clamp(1, 2),
        }
    }

    fn label(&self) -> &'static str {
        match self {
            Self::PreserveMonoStereo => "Preserve mono/stereo",
        }
    }
}

impl Default for AudioWriteChannelBehavior {
    fn default() -> Self {
        Self::PreserveMonoStereo
    }
}

/// Dither policy for integer PCM writes.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum AudioWriteDither {
    /// Do not add dither during integer PCM quantization.
    None,
}

impl AudioWriteDither {
    fn label(&self) -> &'static str {
        match self {
            Self::None => "No dither",
        }
    }
}

impl Default for AudioWriteDither {
    fn default() -> Self {
        Self::None
    }
}

fn format_sample_rate_label(sample_rate: u32) -> String {
    if sample_rate >= 1000 && sample_rate.is_multiple_of(1000) {
        format!("{} kHz", sample_rate / 1000)
    } else if sample_rate >= 1000 {
        format!("{:.1} kHz", sample_rate as f32 / 1000.0)
    } else {
        format!("{sample_rate} Hz")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_write_format_matches_existing_wavecrate_wav_writes() {
        let config = AudioWriteFormatConfig::default();
        let spec = config.wav_spec_for_source(2, 48_000);

        assert_eq!(spec.channels, 2);
        assert_eq!(spec.sample_rate, 48_000);
        assert_eq!(spec.bits_per_sample, 32);
        assert_eq!(spec.sample_format, hound::SampleFormat::Float);
        assert_eq!(
            config.summary_label(),
            "Source rate, 32-bit float, Preserve mono/stereo, No dither"
        );
    }
}
