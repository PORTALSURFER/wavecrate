use crate::selection::SelectionRange;
use hound::SampleFormat;
use std::fs;
use std::io::Cursor;
use std::path::Path;

/// Decoded WAV samples plus metadata derived from the file header.
#[derive(Clone, Debug)]
pub(crate) struct DecodedSamples {
    pub(crate) samples: Vec<f32>,
    pub(crate) channels: u16,
    pub(crate) sample_rate: u32,
}

/// Decode WAV bytes into interleaved f32 samples plus basic metadata.
pub(crate) fn decode_samples_from_bytes(bytes: &[u8]) -> Result<DecodedSamples, String> {
    let mut reader =
        hound::WavReader::new(Cursor::new(bytes)).map_err(|err| format!("Invalid wav: {err}"))?;
    let spec = reader.spec();
    let channels = spec.channels.max(1);
    let samples = decode_samples(&mut reader, spec.sample_format, spec.bits_per_sample)?;
    Ok(DecodedSamples {
        samples,
        channels,
        sample_rate: spec.sample_rate.max(1),
    })
}

/// Convert generic decoder failures into browser preview/load wording.
pub(crate) fn preview_load_decode_error(relative_path: &Path, error: impl ToString) -> String {
    let error = error.to_string();
    if is_wav_path(relative_path) {
        return error;
    }
    let file_label = relative_path.display();
    if decoder_error_mentions_unsupported_codec(&error) {
        return format!("Unsupported audio codec for {file_label}");
    }
    format!("Unable to load audio file {file_label}: unsupported or unreadable audio format")
}

/// Crop interleaved samples to the provided normalized bounds.
pub(crate) fn crop_samples(
    samples: &[f32],
    channels: u16,
    bounds: SelectionRange,
) -> Result<Vec<f32>, String> {
    let channels = channels.max(1) as usize;
    let total_frames = samples.len() / channels;
    if total_frames == 0 {
        return Err("No audio data to export".into());
    }
    let (start_frame, end_frame) = frame_bounds(total_frames, bounds);
    Ok(slice_frames(samples, channels, start_frame, end_frame))
}

/// Write interleaved f32 samples to a WAV file.
pub(crate) fn write_wav(
    target: &Path,
    samples: &[f32],
    sample_rate: u32,
    channels: u16,
) -> Result<(), String> {
    let spec = hound::WavSpec {
        channels: channels.max(1),
        sample_rate: sample_rate.max(1),
        bits_per_sample: 32,
        sample_format: SampleFormat::Float,
    };
    write_wav_with_spec(target, samples, spec)
}

/// Write interleaved f32 samples to a WAV file using a configured WAV spec.
pub(crate) fn write_wav_with_spec(
    target: &Path,
    samples: &[f32],
    spec: hound::WavSpec,
) -> Result<(), String> {
    if let Some(parent) = target.parent()
        && !parent.exists()
    {
        fs::create_dir_all(parent)
            .map_err(|err| format!("Failed to create folder {}: {err}", parent.display()))?;
    }
    let mut writer = hound::WavWriter::create(target, spec)
        .map_err(|err| format!("Failed to create clip: {err}"))?;
    match spec.sample_format {
        SampleFormat::Float => {
            for sample in samples {
                writer
                    .write_sample(*sample)
                    .map_err(|err| format!("Failed to write clip: {err}"))?;
            }
        }
        SampleFormat::Int if spec.bits_per_sample <= 16 => {
            for sample in samples {
                let scaled = (sample.clamp(-1.0, 1.0) * i16::MAX as f32).round() as i16;
                writer
                    .write_sample(scaled)
                    .map_err(|err| format!("Failed to write clip: {err}"))?;
            }
        }
        SampleFormat::Int => {
            let max = ((1_i64 << (spec.bits_per_sample.saturating_sub(1) as u32)) - 1) as f32;
            for sample in samples {
                let scaled = (sample.clamp(-1.0, 1.0) * max).round() as i32;
                writer
                    .write_sample(scaled)
                    .map_err(|err| format!("Failed to write clip: {err}"))?;
            }
        }
    }
    writer
        .finalize()
        .map_err(|err| format!("Failed to finalize clip: {err}"))
}

/// Encode interleaved f32 samples into WAV bytes for in-memory playback.
pub(crate) fn wav_bytes_from_samples(
    samples: &[f32],
    sample_rate: u32,
    channels: u16,
) -> Result<Vec<u8>, String> {
    let spec = hound::WavSpec {
        channels: channels.max(1),
        sample_rate: sample_rate.max(1),
        bits_per_sample: 32,
        sample_format: SampleFormat::Float,
    };
    let mut cursor = Cursor::new(Vec::new());
    {
        let mut writer = hound::WavWriter::new(&mut cursor, spec)
            .map_err(|err| format!("Failed to create wav buffer: {err}"))?;
        for sample in samples {
            writer
                .write_sample(*sample)
                .map_err(|err| format!("Failed to write wav buffer: {err}"))?;
        }
        writer
            .finalize()
            .map_err(|err| format!("Failed to finalize wav buffer: {err}"))?;
    }
    Ok(cursor.into_inner())
}

fn is_wav_path(path: &Path) -> bool {
    path.extension()
        .and_then(|ext| ext.to_str())
        .is_some_and(|ext| ext.eq_ignore_ascii_case("wav"))
}

fn decoder_error_mentions_unsupported_codec(error: &str) -> bool {
    let error = error.to_ascii_lowercase();
    error.contains("unsupported codec")
        || (error.contains("unsupported feature") && error.contains("codec"))
}

fn decode_samples(
    reader: &mut hound::WavReader<Cursor<&[u8]>>,
    format: SampleFormat,
    bits_per_sample: u16,
) -> Result<Vec<f32>, String> {
    match format {
        SampleFormat::Float => reader
            .samples::<f32>()
            .map(|s| s.map_err(|err| format!("Sample error: {err}")))
            .collect::<Result<Vec<_>, _>>(),
        SampleFormat::Int => {
            let scale = (1i64 << bits_per_sample.saturating_sub(1)).max(1) as f32;
            reader
                .samples::<i32>()
                .map(|s| {
                    s.map(|v| v as f32 / scale)
                        .map_err(|err| format!("Sample error: {err}"))
                })
                .collect::<Result<Vec<_>, _>>()
        }
    }
}

fn frame_bounds(total_frames: usize, bounds: SelectionRange) -> (usize, usize) {
    let start_frame = ((bounds.start() * total_frames as f32).floor() as usize)
        .min(total_frames.saturating_sub(1));
    let mut end_frame = ((bounds.end() * total_frames as f32).ceil() as usize).min(total_frames);
    if end_frame <= start_frame {
        end_frame = (start_frame + 1).min(total_frames);
    }
    (start_frame, end_frame)
}

fn slice_frames(
    samples: &[f32],
    channels: usize,
    start_frame: usize,
    end_frame: usize,
) -> Vec<f32> {
    let mut cropped = Vec::with_capacity((end_frame - start_frame) * channels);
    for frame in start_frame..end_frame {
        let offset = frame * channels;
        cropped.extend_from_slice(&samples[offset..offset + channels]);
    }
    cropped
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn write_wav_with_spec_writes_configured_pcm24_header() {
        let temp = tempfile::tempdir().expect("tempdir");
        let path = temp.path().join("clip.wav");
        let spec = hound::WavSpec {
            channels: 2,
            sample_rate: 48_000,
            bits_per_sample: 24,
            sample_format: SampleFormat::Int,
        };

        write_wav_with_spec(&path, &[-1.0, 0.0, 0.5, 1.0], spec).expect("write wav");

        let reader = hound::WavReader::open(path).expect("read wav");
        let written = reader.spec();
        assert_eq!(written.channels, 2);
        assert_eq!(written.sample_rate, 48_000);
        assert_eq!(written.bits_per_sample, 24);
        assert_eq!(written.sample_format, SampleFormat::Int);
    }
}
