use std::{
    path::{Path, PathBuf},
    sync::Arc,
    time::{Duration, SystemTime},
};

use super::wav_format::{integer_sample_max_i32, integer_sample_max_i64};

#[derive(Clone, Debug, PartialEq)]
pub(in crate::native_app) struct PreviewAuditionClip {
    pub(in crate::native_app) path: PathBuf,
    pub(in crate::native_app) source_len: u64,
    pub(in crate::native_app) source_modified: Option<SystemTime>,
    pub(in crate::native_app) samples: Arc<[f32]>,
    pub(in crate::native_app) sample_rate: u32,
    pub(in crate::native_app) channels: usize,
    pub(in crate::native_app) frames: usize,
}

impl PreviewAuditionClip {
    pub(in crate::native_app) fn byte_len(&self) -> usize {
        self.samples
            .len()
            .saturating_mul(std::mem::size_of::<f32>())
    }

    pub(in crate::native_app) fn duration_seconds(&self) -> f32 {
        self.frames as f32 / self.sample_rate.max(1) as f32
    }

    #[cfg(test)]
    pub(in crate::native_app) fn matches_file(&self, path: &Path) -> bool {
        source_identity(path)
            .is_some_and(|identity| identity == (self.source_len, self.source_modified))
    }
}

pub(in crate::native_app) fn decode_wav_preview_clip(
    path: PathBuf,
    max_duration: Duration,
) -> Result<PreviewAuditionClip, String> {
    let (source_len, source_modified) = source_identity(&path).ok_or_else(|| {
        format!(
            "Could not read audio file metadata for preview: {}",
            path.display()
        )
    })?;
    let mut reader = hound::WavReader::open(&path)
        .map_err(|err| format!("failed to open WAV preview {}: {err}", path.display()))?;
    let spec = reader.spec();
    let sample_rate = spec.sample_rate.max(1);
    let channels = usize::from(spec.channels).max(1);
    let available_frames = reader.duration() as usize;
    if available_frames == 0 {
        return Err(String::from("WAV contains no previewable samples"));
    }
    let requested_frames =
        ((max_duration.as_secs_f64() * f64::from(sample_rate)).ceil() as usize).max(1);
    let frames = requested_frames.min(available_frames);
    let sample_count = frames.saturating_mul(channels);
    let samples = match spec.sample_format {
        hound::SampleFormat::Float => read_preview_float_samples(&mut reader, sample_count)?,
        hound::SampleFormat::Int if spec.bits_per_sample <= 16 => {
            read_preview_i16_samples(&mut reader, spec.bits_per_sample, sample_count)?
        }
        hound::SampleFormat::Int => {
            read_preview_i32_samples(&mut reader, spec.bits_per_sample, sample_count)?
        }
    };
    if samples.is_empty() {
        return Err(String::from("WAV contains no previewable samples"));
    }
    let frames = samples.len() / channels;
    Ok(PreviewAuditionClip {
        path,
        source_len,
        source_modified,
        samples: Arc::from(samples),
        sample_rate,
        channels,
        frames,
    })
}

fn source_identity(path: &Path) -> Option<(u64, Option<SystemTime>)> {
    let metadata = path.metadata().ok()?;
    Some((metadata.len(), metadata.modified().ok()))
}

fn read_preview_float_samples<R: std::io::Read>(
    reader: &mut hound::WavReader<R>,
    sample_count: usize,
) -> Result<Vec<f32>, String> {
    let mut samples = Vec::with_capacity(sample_count);
    for sample in reader.samples::<f32>().take(sample_count) {
        let sample = sample
            .map(|value| value.clamp(-1.0, 1.0))
            .map_err(|err| format!("failed to read float preview sample: {err}"))?;
        samples.push(sample);
    }
    Ok(samples)
}

fn read_preview_i16_samples<R: std::io::Read>(
    reader: &mut hound::WavReader<R>,
    bits_per_sample: u16,
    sample_count: usize,
) -> Result<Vec<f32>, String> {
    let max = integer_sample_max_i32(bits_per_sample);
    let mut samples = Vec::with_capacity(sample_count);
    for sample in reader.samples::<i16>().take(sample_count) {
        let sample = sample
            .map(|value| (f32::from(value) / max).clamp(-1.0, 1.0))
            .map_err(|err| format!("failed to read integer preview sample: {err}"))?;
        samples.push(sample);
    }
    Ok(samples)
}

fn read_preview_i32_samples<R: std::io::Read>(
    reader: &mut hound::WavReader<R>,
    bits_per_sample: u16,
    sample_count: usize,
) -> Result<Vec<f32>, String> {
    let max = integer_sample_max_i64(bits_per_sample);
    let mut samples = Vec::with_capacity(sample_count);
    for sample in reader.samples::<i32>().take(sample_count) {
        let sample = sample
            .map(|value| ((value as f32) / max).clamp(-1.0, 1.0))
            .map_err(|err| format!("failed to read integer preview sample: {err}"))?;
        samples.push(sample);
    }
    Ok(samples)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn wav_preview_clip_reads_only_requested_head_frames() {
        let dir = tempfile::tempdir().expect("tempdir");
        let path = dir.path().join("preview.wav");
        let spec = hound::WavSpec {
            channels: 2,
            sample_rate: 48_000,
            bits_per_sample: 16,
            sample_format: hound::SampleFormat::Int,
        };
        {
            let mut writer = hound::WavWriter::create(&path, spec).expect("writer");
            for sample in 0..48_000_i32 {
                let sample = (sample % 24_000) as i16;
                writer.write_sample(sample).expect("write left");
                writer.write_sample(-sample).expect("write right");
            }
            writer.finalize().expect("finalize");
        }

        let clip =
            decode_wav_preview_clip(path.clone(), Duration::from_millis(10)).expect("preview clip");

        assert_eq!(clip.path, path);
        assert_eq!(clip.sample_rate, 48_000);
        assert_eq!(clip.channels, 2);
        assert_eq!(clip.frames, 480);
        assert_eq!(clip.samples.len(), 960);
        assert!(clip.matches_file(&clip.path));
    }
}
