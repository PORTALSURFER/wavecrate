use std::{io::Cursor, path::PathBuf, sync::Arc, time::Instant};

use super::{
    WaveformFile, WaveformPlaybackReady, downmix_to_mono_with_progress_and_cancel,
    report_phase_progress_throttled,
};
use crate::native_app::waveform::audio_file::diagnostics::log_audio_load_timing;

pub(in crate::native_app::waveform) fn load_wav_waveform_file_with_progress(
    path: PathBuf,
    bytes: Arc<[u8]>,
    progress: &impl Fn(f32),
    cancelled: &impl Fn() -> bool,
    playback_ready: &impl Fn(WaveformPlaybackReady),
) -> Result<WaveformFile, String> {
    let cursor = Cursor::new(bytes.as_ref());
    let mut reader =
        hound::WavReader::new(cursor).map_err(|err| format!("failed to open WAV: {err}"))?;
    let spec = reader.spec();
    let channels = usize::from(spec.channels).max(1);
    let sample_started_at = Instant::now();
    let samples = read_wav_samples_with_progress(&mut reader, spec, channels, progress, cancelled)?;
    log_audio_load_timing(
        "browser.audio_file.wav.read_samples",
        &path,
        sample_started_at.elapsed(),
    );
    if samples.is_empty() {
        return Err(String::from("WAV contains no samples"));
    }

    let frames = samples.len() / channels;
    let playback_samples = Arc::from(samples);
    if cancelled() {
        return Err(String::from("cancelled"));
    }
    playback_ready(WaveformPlaybackReady {
        path: path.clone(),
        audio_bytes: Arc::clone(&bytes),
        playback_samples: Arc::clone(&playback_samples),
        sample_rate: spec.sample_rate,
        channels,
        frames,
    });
    if cancelled() {
        return Err(String::from("cancelled"));
    }
    let downmix_started_at = Instant::now();
    let mono_samples = downmix_to_mono_with_progress_and_cancel(
        &playback_samples,
        channels,
        frames,
        0.46,
        0.62,
        progress,
        cancelled,
    )?;
    log_audio_load_timing(
        "browser.audio_file.wav.downmix",
        &path,
        downmix_started_at.elapsed(),
    );
    if mono_samples.is_empty() {
        return Err(String::from("WAV contains no complete frames"));
    }
    if cancelled() {
        return Err(String::from("cancelled"));
    }
    let waveform_started_at = Instant::now();
    let mut file = super::waveform_file_from_mono_samples_with_progress_and_cancel(
        path,
        bytes,
        spec.sample_rate,
        channels,
        mono_samples,
        progress,
        cancelled,
    )?;
    log_audio_load_timing(
        "browser.audio_file.wav.waveform_summary",
        &file.path,
        waveform_started_at.elapsed(),
    );
    file.playback_samples = Some(playback_samples);
    Ok(file)
}

pub(super) fn read_wav_playback_samples(bytes: &Arc<[u8]>) -> Result<Vec<f32>, String> {
    let cursor = Cursor::new(bytes.as_ref());
    let mut reader =
        hound::WavReader::new(cursor).map_err(|err| format!("failed to open WAV: {err}"))?;
    let spec = reader.spec();
    let channels = usize::from(spec.channels).max(1);
    read_wav_samples_with_progress(&mut reader, spec, channels, &|_| {}, &|| false)
}

fn read_wav_samples_with_progress<R: std::io::Read>(
    reader: &mut hound::WavReader<R>,
    spec: hound::WavSpec,
    channels: usize,
    progress: &impl Fn(f32),
    cancelled: &impl Fn() -> bool,
) -> Result<Vec<f32>, String> {
    let total_samples = reader.duration() as usize * channels;
    match spec.sample_format {
        hound::SampleFormat::Float => {
            read_float_samples(reader, total_samples, progress, cancelled)
        }
        hound::SampleFormat::Int if spec.bits_per_sample <= 16 => read_i16_samples(
            reader,
            spec.bits_per_sample,
            total_samples,
            progress,
            cancelled,
        ),
        hound::SampleFormat::Int => read_i32_samples(
            reader,
            spec.bits_per_sample,
            total_samples,
            progress,
            cancelled,
        ),
    }
}

fn read_float_samples<R: std::io::Read>(
    reader: &mut hound::WavReader<R>,
    total_samples: usize,
    progress: &impl Fn(f32),
    cancelled: &impl Fn() -> bool,
) -> Result<Vec<f32>, String> {
    let mut samples = Vec::with_capacity(total_samples);
    for (index, sample) in reader.samples::<f32>().enumerate() {
        if cancelled() {
            return Err(String::from("cancelled"));
        }
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
    cancelled: &impl Fn() -> bool,
) -> Result<Vec<f32>, String> {
    let max = integer_sample_max_i32(bits_per_sample);
    let mut samples = Vec::with_capacity(total_samples);
    for (index, sample) in reader.samples::<i16>().enumerate() {
        if cancelled() {
            return Err(String::from("cancelled"));
        }
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
    cancelled: &impl Fn() -> bool,
) -> Result<Vec<f32>, String> {
    let max = integer_sample_max_i64(bits_per_sample);
    let mut samples = Vec::with_capacity(total_samples);
    for (index, sample) in reader.samples::<i32>().enumerate() {
        if cancelled() {
            return Err(String::from("cancelled"));
        }
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

#[cfg(test)]
mod tests {
    use std::{
        path::PathBuf,
        sync::{
            Arc,
            atomic::{AtomicBool, Ordering},
        },
    };

    use super::*;

    fn wav_bytes_i16(channels: u16, samples: &[i16]) -> Arc<[u8]> {
        let spec = hound::WavSpec {
            channels,
            sample_rate: 48_000,
            bits_per_sample: 16,
            sample_format: hound::SampleFormat::Int,
        };
        let mut cursor = Cursor::new(Vec::new());
        {
            let mut writer = hound::WavWriter::new(&mut cursor, spec)
                .expect("test wav writer should be created");
            for &sample in samples {
                writer
                    .write_sample(sample)
                    .expect("test wav sample should be written");
            }
            writer.finalize().expect("test wav should be finalized");
        }
        Arc::from(cursor.into_inner())
    }

    #[test]
    fn cancellation_after_playback_ready_stops_waveform_summary() {
        let bytes = wav_bytes_i16(2, &[0, 0, 1000, -1000, 2000, -2000, 0, 0]);
        let cancelled = AtomicBool::new(false);
        let playback_ready_called = AtomicBool::new(false);

        let result = load_wav_waveform_file_with_progress(
            PathBuf::from("cancelled-after-playback-ready.wav"),
            bytes,
            &|_| {},
            &|| cancelled.load(Ordering::Relaxed),
            &|_| {
                playback_ready_called.store(true, Ordering::Relaxed);
                cancelled.store(true, Ordering::Relaxed);
            },
        );

        assert!(matches!(result, Err(error) if error == "cancelled"));
        assert!(playback_ready_called.load(Ordering::Relaxed));
    }
}
