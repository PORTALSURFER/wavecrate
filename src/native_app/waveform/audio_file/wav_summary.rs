use std::{
    collections::hash_map::DefaultHasher,
    hash::{Hash, Hasher},
    path::{Path, PathBuf},
    sync::Arc,
    time::{Duration, Instant, SystemTime},
};

use crate::native_app::waveform::audio_file::diagnostics::log_audio_load_timing;
use wavecrate::audio::{Source, decoder::SymphoniaDecoder};

use super::{
    WaveformFile, report_phase_progress_throttled,
    wav_format::{integer_sample_max_i32, integer_sample_max_i64},
    wav_summary_builder::{
        MAX_STREAMING_WAV_SUMMARY_BUCKETS, STREAMING_WAV_SUMMARY_BUILD_END,
        STREAMING_WAV_SUMMARY_READ_END, StreamingWavSummaryBuilder,
        streaming_summary_bucket_frames,
    },
    wav_summary_hound::{
        read_float_frame_peak, read_i16_frame_peak, read_i32_frame_peak,
        read_wav_summary_with_progress,
    },
};

pub(in crate::native_app) fn load_wav_detail_summary(
    key: crate::native_app::waveform::WaveformDetailKey,
) -> crate::native_app::waveform::WaveformDetailResult {
    let started_at = Instant::now();
    let summary = (|| {
        let mut reader = hound::WavReader::open(&key.path)
            .map_err(|error| format!("failed to open detail WAV: {error}"))?;
        let spec = reader.spec();
        let channels = usize::from(spec.channels).max(1);
        let source_frames = reader.duration() as usize;
        let end = key.end_frame.min(source_frames);
        let start = key.start_frame.min(end);
        let visible = end.saturating_sub(start);
        if visible == 0 || start > u32::MAX as usize {
            return Err(String::from("waveform detail range is unavailable"));
        }
        let metadata_before = std::fs::metadata(&key.path)
            .map_err(|error| format!("failed to inspect detail WAV: {error}"))?;
        let revision_before = content_revision_for_path_metadata(
            &key.path,
            &metadata_before,
            spec.sample_rate,
            channels,
            source_frames,
        );
        if revision_before != key.content_revision {
            return Err(String::from("stale waveform detail request"));
        }
        reader
            .seek(start as u32)
            .map_err(|error| format!("failed to seek detail WAV: {error}"))?;
        let bucket_frames = visible.div_ceil(MAX_STREAMING_WAV_SUMMARY_BUCKETS).max(1);
        let mut builder = StreamingWavSummaryBuilder::new(spec.sample_rate, bucket_frames);
        match spec.sample_format {
            hound::SampleFormat::Float => {
                let mut samples = reader.samples::<f32>();
                for _ in 0..visible {
                    builder.push_peak(read_float_frame_peak(&mut samples, channels)?);
                }
            }
            hound::SampleFormat::Int if spec.bits_per_sample <= 16 => {
                let max = integer_sample_max_i32(spec.bits_per_sample);
                let mut samples = reader.samples::<i16>();
                for _ in 0..visible {
                    builder.push_peak(read_i16_frame_peak(&mut samples, channels, max)?);
                }
            }
            hound::SampleFormat::Int => {
                let max = integer_sample_max_i64(spec.bits_per_sample);
                let mut samples = reader.samples::<i32>();
                for _ in 0..visible {
                    builder.push_peak(read_i32_frame_peak(&mut samples, channels, max)?);
                }
            }
        }
        let summary = builder.finish(0.0, 1.0, &|_| {}, &|| false)?;
        let metadata_after = std::fs::metadata(&key.path)
            .map_err(|error| format!("failed to revalidate detail WAV: {error}"))?;
        let revision_after = content_revision_for_path_metadata(
            &key.path,
            &metadata_after,
            spec.sample_rate,
            channels,
            source_frames,
        );
        if revision_after != key.content_revision {
            return Err(String::from("stale waveform detail result"));
        }
        Ok(Arc::new(summary))
    })();
    tracing::debug!(
        target: "wavecrate::waveform_detail",
        event = "waveform.detail.refinement",
        path = %key.path.display(),
        start_frame = key.start_frame,
        end_frame = key.end_frame,
        elapsed_ms = started_at.elapsed().as_secs_f64() * 1000.0,
        outcome = if summary.is_ok() { "ready" } else { "rejected" },
        "Waveform viewport detail refinement finished"
    );
    crate::native_app::waveform::WaveformDetailResult { key, summary }
}

pub(in crate::native_app::waveform) fn load_wav_waveform_summary_from_path_with_progress(
    path: PathBuf,
    progress: &impl Fn(f32),
    cancelled: &impl Fn() -> bool,
) -> Result<WaveformFile, String> {
    match load_wav_waveform_summary_with_hound(path.clone(), progress, cancelled) {
        Ok(file) => Ok(file),
        Err(error) if error == "cancelled" || cancelled() => Err(error),
        Err(hound_error) => {
            tracing::warn!(
                target: "wavecrate::audio_file",
                path = %path.display(),
                error = %hound_error,
                "Falling back to Symphonia for file-backed WAV summary"
            );
            load_wav_waveform_summary_with_symphonia(path, progress, cancelled).map_err(
                |fallback_error| {
                    format!(
                        "failed to summarize WAV with hound: {hound_error}; fallback decoder failed: {fallback_error}"
                    )
                },
            )
        }
    }
}

fn load_wav_waveform_summary_with_hound(
    path: PathBuf,
    progress: &impl Fn(f32),
    cancelled: &impl Fn() -> bool,
) -> Result<WaveformFile, String> {
    let mut reader =
        hound::WavReader::open(&path).map_err(|err| format!("failed to open WAV: {err}"))?;
    let spec = reader.spec();
    let channels = usize::from(spec.channels).max(1);
    let total_frames = reader.duration() as usize;
    let bucket_frames = streaming_summary_bucket_frames(total_frames);
    let mut builder = StreamingWavSummaryBuilder::new(spec.sample_rate, bucket_frames);
    let read_started_at = Instant::now();
    read_wav_summary_with_progress(
        &mut reader,
        spec,
        channels,
        &mut builder,
        progress,
        cancelled,
    )?;
    log_audio_load_timing(
        "browser.audio_file.wav.stream_summary_read",
        &path,
        read_started_at.elapsed(),
    );
    finish_streaming_summary_file(
        path,
        spec.sample_rate,
        channels,
        builder,
        "browser.audio_file.wav.stream_summary_build",
        progress,
        cancelled,
    )
}

fn load_wav_waveform_summary_with_symphonia(
    path: PathBuf,
    progress: &impl Fn(f32),
    cancelled: &impl Fn() -> bool,
) -> Result<WaveformFile, String> {
    let mut decoder = SymphoniaDecoder::from_path(&path)?;
    let sample_rate = decoder.sample_rate().max(1);
    let channels = usize::from(decoder.channels()).max(1);
    let total_frames_estimate =
        estimate_decoder_total_frames(&path, decoder.total_duration(), sample_rate, channels);
    let bucket_frames = streaming_summary_bucket_frames(total_frames_estimate);
    let mut builder = StreamingWavSummaryBuilder::new(sample_rate, bucket_frames);
    let read_started_at = Instant::now();
    read_decoder_summary_with_progress(
        &mut decoder,
        channels,
        total_frames_estimate,
        &mut builder,
        progress,
        cancelled,
    )?;
    log_audio_load_timing(
        "browser.audio_file.wav.stream_summary_fallback_read",
        &path,
        read_started_at.elapsed(),
    );
    finish_streaming_summary_file(
        path,
        sample_rate,
        channels,
        builder,
        "browser.audio_file.wav.stream_summary_fallback_build",
        progress,
        cancelled,
    )
}

fn finish_streaming_summary_file(
    path: PathBuf,
    sample_rate: u32,
    channels: usize,
    builder: StreamingWavSummaryBuilder,
    build_stage: &'static str,
    progress: &impl Fn(f32),
    cancelled: &impl Fn() -> bool,
) -> Result<WaveformFile, String> {
    if builder.frames() == 0 {
        return Err(String::from("WAV contains no complete frames"));
    }
    if cancelled() {
        return Err(String::from("cancelled"));
    }
    let summary_started_at = Instant::now();
    let summary = builder.finish(
        STREAMING_WAV_SUMMARY_READ_END,
        STREAMING_WAV_SUMMARY_BUILD_END,
        progress,
        cancelled,
    )?;
    log_audio_load_timing(build_stage, &path, summary_started_at.elapsed());
    let metadata = std::fs::metadata(&path)
        .map_err(|err| format!("failed to read WAV metadata {}: {err}", path.display()))?;
    let frames = summary.frames;
    Ok(WaveformFile {
        path: path.clone(),
        content_revision: content_revision_for_path_metadata(
            &path,
            &metadata,
            sample_rate,
            channels,
            frames,
        ),
        audio_bytes: Arc::from([]),
        playback_samples: None,
        playback_cache_file: None,
        sample_rate,
        channels,
        frames,
        gpu_signal_summary: Arc::new(summary),
    })
}

fn read_decoder_summary_with_progress(
    decoder: &mut SymphoniaDecoder,
    channels: usize,
    total_frames_estimate: usize,
    builder: &mut StreamingWavSummaryBuilder,
    progress: &impl Fn(f32),
    cancelled: &impl Fn() -> bool,
) -> Result<(), String> {
    let mut frames_read = 0usize;
    while let Some(peak) = read_decoder_frame_peak(decoder, channels) {
        if cancelled() {
            return Err(String::from("cancelled"));
        }
        builder.push_peak(peak);
        frames_read = frames_read.saturating_add(1);
        report_phase_progress_throttled(
            0.0,
            STREAMING_WAV_SUMMARY_READ_END,
            frames_read.min(total_frames_estimate),
            total_frames_estimate.max(1),
            progress,
        );
    }
    if let Some(error) = decoder.last_error() {
        return Err(error);
    }
    progress(STREAMING_WAV_SUMMARY_READ_END);
    Ok(())
}

fn read_decoder_frame_peak(decoder: &mut SymphoniaDecoder, channels: usize) -> Option<f32> {
    let first = decoder.next()?.clamp(-1.0, 1.0);
    let mut peak = first;
    for _ in 1..channels {
        let sample = decoder.next()?.clamp(-1.0, 1.0);
        if sample.abs() > peak.abs() {
            peak = sample;
        }
    }
    Some(peak.clamp(-1.0, 1.0))
}

fn estimate_decoder_total_frames(
    path: &Path,
    duration: Option<Duration>,
    sample_rate: u32,
    channels: usize,
) -> usize {
    duration
        .map(|duration| duration_to_frames_ceil(duration, sample_rate))
        .or_else(|| {
            path.metadata()
                .ok()
                .map(|metadata| estimate_frames_from_file_size(metadata.len(), channels))
        })
        .unwrap_or(MAX_STREAMING_WAV_SUMMARY_BUCKETS)
        .max(1)
}

fn duration_to_frames_ceil(duration: Duration, sample_rate: u32) -> usize {
    let sample_rate = u128::from(sample_rate.max(1));
    let whole = u128::from(duration.as_secs()).saturating_mul(sample_rate);
    let fractional = (u128::from(duration.subsec_nanos()) * sample_rate).div_ceil(1_000_000_000);
    usize::try_from(whole.saturating_add(fractional)).unwrap_or(usize::MAX)
}

fn estimate_frames_from_file_size(bytes: u64, channels: usize) -> usize {
    let bytes_per_frame = u64::try_from(channels.max(1)).unwrap_or(u64::MAX).max(1);
    usize::try_from(bytes / bytes_per_frame)
        .unwrap_or(usize::MAX)
        .max(1)
}

fn content_revision_for_path_metadata(
    path: &Path,
    metadata: &std::fs::Metadata,
    sample_rate: u32,
    channels: usize,
    frames: usize,
) -> u64 {
    let mut hasher = DefaultHasher::new();
    path.hash(&mut hasher);
    metadata.len().hash(&mut hasher);
    modified_ns(metadata).hash(&mut hasher);
    sample_rate.hash(&mut hasher);
    channels.hash(&mut hasher);
    frames.hash(&mut hasher);
    hasher.finish().max(1)
}

fn modified_ns(metadata: &std::fs::Metadata) -> u128 {
    metadata
        .modified()
        .ok()
        .and_then(|modified| modified.duration_since(SystemTime::UNIX_EPOCH).ok())
        .map(|duration| duration.as_nanos())
        .unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use std::{cell::RefCell, io::Cursor, path::Path, sync::Arc};

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

    fn write_wav_i16(path: &Path, channels: u16, samples: &[i16]) {
        std::fs::write(path, wav_bytes_i16(channels, samples)).expect("write wav");
    }

    fn corrupt_byte_rate(bytes: &mut [u8]) {
        let byte_rate_offset = 12 + 8 + 2 + 2 + 4;
        if bytes.len() >= byte_rate_offset + 4 {
            bytes[byte_rate_offset..byte_rate_offset + 4].copy_from_slice(&0u32.to_le_bytes());
        }
    }

    #[test]
    fn streaming_wav_summary_keeps_large_file_playback_file_backed() {
        let dir = tempfile::tempdir().expect("tempdir");
        let path = dir.path().join("large-summary.wav");
        write_wav_i16(
            &path,
            2,
            &[0, 0, 1_000, -4_000, 6_000, -2_000, 0, 0, -8_000, 4_000],
        );
        let progress_values = RefCell::new(Vec::new());

        let file = load_wav_waveform_summary_from_path_with_progress(
            path.clone(),
            &|progress| progress_values.borrow_mut().push(progress),
            &|| false,
        )
        .expect("streaming summary");

        assert_eq!(file.path, path);
        assert_eq!(file.sample_rate, 48_000);
        assert_eq!(file.channels, 2);
        assert_eq!(file.frames, 5);
        assert!(file.audio_bytes.is_empty());
        assert!(file.playback_samples.is_none());
        assert!(file.playback_cache_file.is_none());
        assert_eq!(file.gpu_signal_summary.frames, 5);
        assert!(!file.gpu_signal_summary.levels.is_empty());
        assert!(progress_values.borrow().iter().any(|value| *value >= 0.99));
    }

    #[test]
    fn streaming_wav_summary_falls_back_for_hound_rejected_wav() {
        let dir = tempfile::tempdir().expect("tempdir");
        let path = dir.path().join("fallback-summary.wav");
        let mut bytes = wav_bytes_i16(1, &[0, 1_000, -1_000, 0]).to_vec();
        corrupt_byte_rate(&mut bytes);
        assert!(
            hound::WavReader::new(Cursor::new(bytes.as_slice())).is_err(),
            "expected hound to reject the file"
        );
        std::fs::write(&path, bytes).expect("write wav");
        let progress_values = RefCell::new(Vec::new());

        let file = load_wav_waveform_summary_from_path_with_progress(
            path.clone(),
            &|progress| progress_values.borrow_mut().push(progress),
            &|| false,
        )
        .expect("fallback summary");

        assert_eq!(file.path, path);
        assert_eq!(file.sample_rate, 48_000);
        assert_eq!(file.channels, 1);
        assert_eq!(file.frames, 4);
        assert!(file.audio_bytes.is_empty());
        assert!(file.playback_samples.is_none());
        assert!(file.playback_cache_file.is_none());
        assert_eq!(file.gpu_signal_summary.frames, 4);
        assert!(!file.gpu_signal_summary.levels.is_empty());
        assert!(progress_values.borrow().iter().any(|value| *value >= 0.99));
    }
}
