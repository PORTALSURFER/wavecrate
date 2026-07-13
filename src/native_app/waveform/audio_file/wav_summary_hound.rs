use super::{
    report_phase_progress_throttled,
    wav_format::{integer_sample_max_i32, integer_sample_max_i64},
    wav_summary_builder::{STREAMING_WAV_SUMMARY_READ_END, StreamingWavSummaryBuilder},
};

pub(super) fn read_wav_summary_with_progress<R: std::io::Read>(
    reader: &mut hound::WavReader<R>,
    spec: hound::WavSpec,
    channels: usize,
    builder: &mut StreamingWavSummaryBuilder,
    progress: &impl Fn(f32),
    cancelled: &impl Fn() -> bool,
) -> Result<(), String> {
    let total_frames = reader.duration() as usize;
    match spec.sample_format {
        hound::SampleFormat::Float => {
            read_float_summary(reader, channels, builder, total_frames, progress, cancelled)
        }
        hound::SampleFormat::Int if spec.bits_per_sample <= 16 => read_i16_summary(
            reader,
            channels,
            spec.bits_per_sample,
            builder,
            total_frames,
            progress,
            cancelled,
        ),
        hound::SampleFormat::Int => read_i32_summary(
            reader,
            channels,
            spec.bits_per_sample,
            builder,
            total_frames,
            progress,
            cancelled,
        ),
    }
}

fn read_float_summary<R: std::io::Read>(
    reader: &mut hound::WavReader<R>,
    channels: usize,
    builder: &mut StreamingWavSummaryBuilder,
    total_frames: usize,
    progress: &impl Fn(f32),
    cancelled: &impl Fn() -> bool,
) -> Result<(), String> {
    let mut samples = reader.samples::<f32>();
    for frame_index in 0..total_frames {
        if cancelled() {
            return Err(String::from("cancelled"));
        }
        let peak = read_float_frame_peak(&mut samples, channels)?;
        builder.push_peak(peak);
        report_phase_progress_throttled(
            0.0,
            STREAMING_WAV_SUMMARY_READ_END,
            frame_index + 1,
            total_frames,
            progress,
        );
    }
    progress(STREAMING_WAV_SUMMARY_READ_END);
    Ok(())
}

fn read_i16_summary<R: std::io::Read>(
    reader: &mut hound::WavReader<R>,
    channels: usize,
    bits_per_sample: u16,
    builder: &mut StreamingWavSummaryBuilder,
    total_frames: usize,
    progress: &impl Fn(f32),
    cancelled: &impl Fn() -> bool,
) -> Result<(), String> {
    let max = integer_sample_max_i32(bits_per_sample);
    let mut samples = reader.samples::<i16>();
    for frame_index in 0..total_frames {
        if cancelled() {
            return Err(String::from("cancelled"));
        }
        let peak = read_i16_frame_peak(&mut samples, channels, max)?;
        builder.push_peak(peak);
        report_phase_progress_throttled(
            0.0,
            STREAMING_WAV_SUMMARY_READ_END,
            frame_index + 1,
            total_frames,
            progress,
        );
    }
    progress(STREAMING_WAV_SUMMARY_READ_END);
    Ok(())
}

fn read_i32_summary<R: std::io::Read>(
    reader: &mut hound::WavReader<R>,
    channels: usize,
    bits_per_sample: u16,
    builder: &mut StreamingWavSummaryBuilder,
    total_frames: usize,
    progress: &impl Fn(f32),
    cancelled: &impl Fn() -> bool,
) -> Result<(), String> {
    let max = integer_sample_max_i64(bits_per_sample);
    let mut samples = reader.samples::<i32>();
    for frame_index in 0..total_frames {
        if cancelled() {
            return Err(String::from("cancelled"));
        }
        let peak = read_i32_frame_peak(&mut samples, channels, max)?;
        builder.push_peak(peak);
        report_phase_progress_throttled(
            0.0,
            STREAMING_WAV_SUMMARY_READ_END,
            frame_index + 1,
            total_frames,
            progress,
        );
    }
    progress(STREAMING_WAV_SUMMARY_READ_END);
    Ok(())
}

pub(super) fn read_float_frame_peak<R: std::io::Read>(
    samples: &mut hound::WavSamples<'_, R, f32>,
    channels: usize,
) -> Result<f32, String> {
    let mut peak = 0.0_f32;
    for _ in 0..channels {
        let sample = samples
            .next()
            .transpose()
            .map_err(|err| format!("failed to read float sample: {err}"))?
            .ok_or_else(|| String::from("WAV ended before declared frame count"))?
            .clamp(-1.0, 1.0);
        if sample.abs() > peak.abs() {
            peak = sample;
        }
    }
    Ok(peak.clamp(-1.0, 1.0))
}

pub(super) fn read_i16_frame_peak<R: std::io::Read>(
    samples: &mut hound::WavSamples<'_, R, i16>,
    channels: usize,
    max: f32,
) -> Result<f32, String> {
    let mut peak = 0.0_f32;
    for _ in 0..channels {
        let sample = samples
            .next()
            .transpose()
            .map_err(|err| format!("failed to read integer sample: {err}"))?
            .ok_or_else(|| String::from("WAV ended before declared frame count"))
            .map(|value| (f32::from(value) / max).clamp(-1.0, 1.0))?;
        if sample.abs() > peak.abs() {
            peak = sample;
        }
    }
    Ok(peak.clamp(-1.0, 1.0))
}

pub(super) fn read_i32_frame_peak<R: std::io::Read>(
    samples: &mut hound::WavSamples<'_, R, i32>,
    channels: usize,
    max: f32,
) -> Result<f32, String> {
    let mut peak = 0.0_f32;
    for _ in 0..channels {
        let sample = samples
            .next()
            .transpose()
            .map_err(|err| format!("failed to read integer sample: {err}"))?
            .ok_or_else(|| String::from("WAV ended before declared frame count"))
            .map(|value| ((value as f32) / max).clamp(-1.0, 1.0))?;
        if sample.abs() > peak.abs() {
            peak = sample;
        }
    }
    Ok(peak.clamp(-1.0, 1.0))
}
