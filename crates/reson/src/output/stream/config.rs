use cpal::traits::DeviceTrait;

use crate::output::{AudioOutputConfig, AudioOutputError, ResolvedOutput};

pub(super) struct ResolvedOutputStreamConfig {
    pub(super) stream_config: cpal::StreamConfig,
    pub(super) sample_format: cpal::SampleFormat,
    pub(super) used_fallback: bool,
}

pub(in crate::output) fn resolved_output_from_stream_config(
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

pub(super) fn resolve_output_stream_config(
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
