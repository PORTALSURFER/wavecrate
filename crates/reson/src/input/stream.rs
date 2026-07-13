use cpal;
use cpal::traits::DeviceTrait;
use tracing::warn;

use super::AudioInputError;

pub(crate) fn build_input_stream(
    device: &cpal::Device,
    config: &cpal::StreamConfig,
    sample_format: cpal::SampleFormat,
    selection: StreamChannelSelection,
    mut on_samples: impl FnMut(&[f32]) + Send + 'static,
) -> Result<cpal::Stream, AudioInputError> {
    let err_fn = move |err| {
        warn!("Audio input stream error: {err}");
    };
    let scratch_capacity = capture_scratch_capacity(config, &selection);
    let selection = std::sync::Arc::new(selection);
    match sample_format {
        cpal::SampleFormat::F32 => {
            let mut samples = Vec::with_capacity(scratch_capacity);
            device
                .build_input_stream(
                    config,
                    move |data: &[f32], _| {
                        extract_selected_samples(data, &selection, &mut samples, |sample| *sample);
                        on_samples(&samples);
                    },
                    err_fn,
                    None,
                )
                .map_err(|source| AudioInputError::OpenStream { source })
        }
        cpal::SampleFormat::I16 => {
            let mut samples = Vec::with_capacity(scratch_capacity);
            device
                .build_input_stream(
                    config,
                    move |data: &[i16], _| {
                        extract_selected_samples(data, &selection, &mut samples, |sample| {
                            *sample as f32 / i16::MAX as f32
                        });
                        on_samples(&samples);
                    },
                    err_fn,
                    None,
                )
                .map_err(|source| AudioInputError::OpenStream { source })
        }
        cpal::SampleFormat::U16 => {
            let mut samples = Vec::with_capacity(scratch_capacity);
            device
                .build_input_stream(
                    config,
                    move |data: &[u16], _| {
                        extract_selected_samples(data, &selection, &mut samples, |sample| {
                            (*sample as f32 - 32_768.0) / 32_768.0
                        });
                        on_samples(&samples);
                    },
                    err_fn,
                    None,
                )
                .map_err(|source| AudioInputError::OpenStream { source })
        }
        cpal::SampleFormat::I32 => {
            let mut samples = Vec::with_capacity(scratch_capacity);
            device
                .build_input_stream(
                    config,
                    move |data: &[i32], _| {
                        extract_selected_samples(data, &selection, &mut samples, |sample| {
                            *sample as f32 / i32::MAX as f32
                        });
                        on_samples(&samples);
                    },
                    err_fn,
                    None,
                )
                .map_err(|source| AudioInputError::OpenStream { source })
        }
        cpal::SampleFormat::U32 => {
            let mut samples = Vec::with_capacity(scratch_capacity);
            device
                .build_input_stream(
                    config,
                    move |data: &[u32], _| {
                        extract_selected_samples(data, &selection, &mut samples, |sample| {
                            (*sample as f32 - 2_147_483_648.0) / 2_147_483_648.0
                        });
                        on_samples(&samples);
                    },
                    err_fn,
                    None,
                )
                .map_err(|source| AudioInputError::OpenStream { source })
        }
        cpal::SampleFormat::F64 => {
            let mut samples = Vec::with_capacity(scratch_capacity);
            device
                .build_input_stream(
                    config,
                    move |data: &[f64], _| {
                        extract_selected_samples(data, &selection, &mut samples, |sample| {
                            *sample as f32
                        });
                        on_samples(&samples);
                    },
                    err_fn,
                    None,
                )
                .map_err(|source| AudioInputError::OpenStream { source })
        }
        format => Err(AudioInputError::RecordingFailed {
            detail: format!("Unsupported input sample format {format:?}"),
        }),
    }
}

fn capture_scratch_capacity(
    config: &cpal::StreamConfig,
    selection: &StreamChannelSelection,
) -> usize {
    let frames = match config.buffer_size {
        cpal::BufferSize::Fixed(frames) => frames as usize,
        cpal::BufferSize::Default => (config.sample_rate as usize / 10).max(4_096),
    };
    frames
        .saturating_mul(selection.selected_channels.len())
        .max(1)
}

#[derive(Clone)]
pub(crate) struct StreamChannelSelection {
    stream_channels: usize,
    selected_channels: Vec<usize>,
}

impl StreamChannelSelection {
    pub(crate) fn new(stream_channels: u16, selected_channels: &[u16]) -> Self {
        let stream_channels = stream_channels.max(1) as usize;
        let mut selected_channels: Vec<usize> = selected_channels
            .iter()
            .copied()
            .filter(|channel| *channel >= 1)
            .map(|channel| (channel - 1) as usize)
            .collect();
        if selected_channels.is_empty() && stream_channels > 0 {
            selected_channels.push(0);
        }
        Self {
            stream_channels,
            selected_channels,
        }
    }
}

fn extract_selected_samples<T>(
    data: &[T],
    selection: &StreamChannelSelection,
    samples: &mut Vec<f32>,
    mut convert: impl FnMut(&T) -> f32,
) {
    samples.clear();
    for frame in data.chunks(selection.stream_channels.max(1)) {
        for &channel_idx in &selection.selected_channels {
            if let Some(sample) = frame.get(channel_idx) {
                samples.push(convert(sample));
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn selected_sample_extraction_reuses_preallocated_storage() {
        let selection = StreamChannelSelection::new(2, &[1]);
        let mut samples = Vec::with_capacity(4);
        let allocation = samples.as_ptr();

        extract_selected_samples(&[0.1_f32, 0.2, 0.3, 0.4], &selection, &mut samples, |v| *v);
        assert_eq!(samples, vec![0.1, 0.3]);
        assert_eq!(samples.as_ptr(), allocation);

        extract_selected_samples(&[0.5_f32, 0.6], &selection, &mut samples, |v| *v);
        assert_eq!(samples, vec![0.5]);
        assert_eq!(samples.as_ptr(), allocation);
    }
}
