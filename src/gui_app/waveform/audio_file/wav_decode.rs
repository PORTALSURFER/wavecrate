use std::{io::Cursor, path::PathBuf, sync::Arc};

use super::{WaveformFile, downmix_to_mono_with_progress, report_phase_progress_throttled};

pub(in crate::gui_app::waveform) fn load_wav_waveform_file_with_progress(
    path: PathBuf,
    bytes: Arc<[u8]>,
    progress: &impl Fn(f32),
) -> Result<WaveformFile, String> {
    let cursor = Cursor::new(bytes.as_ref());
    let mut reader =
        hound::WavReader::new(cursor).map_err(|err| format!("failed to open WAV: {err}"))?;
    let spec = reader.spec();
    let channels = usize::from(spec.channels).max(1);
    let samples = read_wav_samples_with_progress(&mut reader, spec, channels, progress)?;
    if samples.is_empty() {
        return Err(String::from("WAV contains no samples"));
    }

    let frames = samples.len() / channels;
    let mono_samples =
        downmix_to_mono_with_progress(&samples, channels, frames, 0.46, 0.62, progress);
    if mono_samples.is_empty() {
        return Err(String::from("WAV contains no complete frames"));
    }
    let mut file = super::waveform_file_from_mono_samples_with_progress(
        path,
        bytes,
        spec.sample_rate,
        channels,
        mono_samples,
        progress,
    );
    file.playback_samples = Some(Arc::from(samples));
    Ok(file)
}

pub(super) fn read_wav_playback_samples(bytes: &Arc<[u8]>) -> Result<Vec<f32>, String> {
    let cursor = Cursor::new(bytes.as_ref());
    let mut reader =
        hound::WavReader::new(cursor).map_err(|err| format!("failed to open WAV: {err}"))?;
    let spec = reader.spec();
    let channels = usize::from(spec.channels).max(1);
    read_wav_samples_with_progress(&mut reader, spec, channels, &|_| {})
}

fn read_wav_samples_with_progress<R: std::io::Read>(
    reader: &mut hound::WavReader<R>,
    spec: hound::WavSpec,
    channels: usize,
    progress: &impl Fn(f32),
) -> Result<Vec<f32>, String> {
    let total_samples = reader.duration() as usize * channels;
    match spec.sample_format {
        hound::SampleFormat::Float => read_float_samples(reader, total_samples, progress),
        hound::SampleFormat::Int if spec.bits_per_sample <= 16 => {
            read_i16_samples(reader, spec.bits_per_sample, total_samples, progress)
        }
        hound::SampleFormat::Int => {
            read_i32_samples(reader, spec.bits_per_sample, total_samples, progress)
        }
    }
}

fn read_float_samples<R: std::io::Read>(
    reader: &mut hound::WavReader<R>,
    total_samples: usize,
    progress: &impl Fn(f32),
) -> Result<Vec<f32>, String> {
    let mut samples = Vec::with_capacity(total_samples);
    for (index, sample) in reader.samples::<f32>().enumerate() {
        let sample = sample
            .map(|value| value.clamp(-1.0, 1.0))
            .map_err(|err| format!("failed to read float sample: {err}"))?;
        samples.push(sample);
        report_phase_progress_throttled(0.08, 0.46, index + 1, total_samples, progress);
    }
    progress(0.46);
    Ok(samples)
}

fn read_i16_samples<R: std::io::Read>(
    reader: &mut hound::WavReader<R>,
    bits_per_sample: u16,
    total_samples: usize,
    progress: &impl Fn(f32),
) -> Result<Vec<f32>, String> {
    let max = integer_sample_max_i32(bits_per_sample);
    let mut samples = Vec::with_capacity(total_samples);
    for (index, sample) in reader.samples::<i16>().enumerate() {
        let sample = sample
            .map(|value| (f32::from(value) / max).clamp(-1.0, 1.0))
            .map_err(|err| format!("failed to read integer sample: {err}"))?;
        samples.push(sample);
        report_phase_progress_throttled(0.08, 0.46, index + 1, total_samples, progress);
    }
    progress(0.46);
    Ok(samples)
}

fn read_i32_samples<R: std::io::Read>(
    reader: &mut hound::WavReader<R>,
    bits_per_sample: u16,
    total_samples: usize,
    progress: &impl Fn(f32),
) -> Result<Vec<f32>, String> {
    let max = integer_sample_max_i64(bits_per_sample);
    let mut samples = Vec::with_capacity(total_samples);
    for (index, sample) in reader.samples::<i32>().enumerate() {
        let sample = sample
            .map(|value| ((value as f32) / max).clamp(-1.0, 1.0))
            .map_err(|err| format!("failed to read integer sample: {err}"))?;
        samples.push(sample);
        report_phase_progress_throttled(0.08, 0.46, index + 1, total_samples, progress);
    }
    progress(0.46);
    Ok(samples)
}

fn integer_sample_max_i32(bits_per_sample: u16) -> f32 {
    ((1_i32 << (u32::from(bits_per_sample).saturating_sub(1))) - 1).max(1) as f32
}

fn integer_sample_max_i64(bits_per_sample: u16) -> f32 {
    ((1_i64 << (u32::from(bits_per_sample).saturating_sub(1))) - 1).max(1) as f32
}
