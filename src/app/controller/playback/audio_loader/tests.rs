use super::{AudioLoadJob, drain_to_latest_job, is_stale_request};
use crate::app::controller::playback::audio_cache::FileMetadata;
use crate::waveform::DecodedWaveform;
use crate::waveform::WaveformRenderer;
use hound::{SampleFormat, WavSpec, WavWriter};
use std::io::Cursor;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};
use tempfile::NamedTempFile;

fn render_spec()
-> crate::app::controller::library::wavs::waveform_rendering::InitialWaveformRenderSpec {
    crate::app::controller::library::wavs::waveform_rendering::InitialWaveformRenderSpec {
        size: [16, 16],
        channel_view: crate::waveform::WaveformChannelView::Mono,
    }
}

fn test_job(request_id: u64, relative_path: &str) -> AudioLoadJob {
    AudioLoadJob {
        request_id,
        source_id: crate::sample_sources::SourceId::from_string("source"),
        root: PathBuf::from("/tmp"),
        relative_path: PathBuf::from(relative_path),
        stretch_ratio: None,
        render_spec: render_spec(),
        prepared: None,
    }
}

fn test_job_with_root(
    request_id: u64,
    root: &Path,
    relative_path: &Path,
    stretch_ratio: Option<f64>,
) -> AudioLoadJob {
    AudioLoadJob {
        request_id,
        source_id: crate::sample_sources::SourceId::from_string("source"),
        root: root.to_path_buf(),
        relative_path: relative_path.to_path_buf(),
        stretch_ratio,
        render_spec: render_spec(),
        prepared: None,
    }
}

fn generated_audio_sample(frame: usize) -> f32 {
    let base = (((frame * 17) % 193) as f32 / 96.0) - 1.0;
    let accent = if frame % 1024 == 0 {
        1.0
    } else if frame % 1024 == 128 {
        -1.0
    } else {
        0.0
    };
    (base * 0.42) + accent
}

fn build_float_wav(samples: &[f32], channels: u16, sample_rate: u32) -> Vec<u8> {
    let spec = WavSpec {
        channels,
        sample_rate,
        bits_per_sample: 32,
        sample_format: SampleFormat::Float,
    };
    let mut cursor = Cursor::new(Vec::new());
    {
        let mut writer = WavWriter::new(&mut cursor, spec).expect("create wav writer");
        for &sample in samples {
            writer.write_sample(sample).expect("write sample");
        }
        writer.finalize().expect("finalize wav");
    }
    cursor.into_inner()
}

fn write_test_wav(bytes: &[u8]) -> NamedTempFile {
    let mut temp = NamedTempFile::new().expect("tempfile");
    temp.write_all(bytes).expect("write wav");
    temp
}

fn decode_test_waveform(renderer: &WaveformRenderer, bytes: &[u8]) -> Arc<DecodedWaveform> {
    Arc::new(
        renderer
            .decode_from_bytes(bytes)
            .expect("decode reference waveform"),
    )
}

fn test_metadata(byte_len: usize) -> FileMetadata {
    FileMetadata {
        file_size: byte_len as u64,
        modified_ns: 123,
    }
}

#[test]
fn ensure_safe_relative_path_rejects_parent_dir() {
    let err = super::stages::ensure_safe_relative_path(Path::new("../escape.wav")).unwrap_err();
    assert!(matches!(err, super::AudioLoadError::Failed(_)));
}

#[test]
fn ensure_safe_relative_path_rejects_rooted_path() {
    let err = super::stages::ensure_safe_relative_path(Path::new("/escape.wav")).unwrap_err();
    assert!(matches!(err, super::AudioLoadError::Failed(_)));
}

#[cfg(windows)]
#[test]
fn ensure_safe_relative_path_rejects_windows_drive_prefix() {
    let err = super::stages::ensure_safe_relative_path(Path::new(r"C:\escape.wav")).unwrap_err();
    assert!(matches!(err, super::AudioLoadError::Failed(_)));
}

#[cfg(windows)]
#[test]
fn ensure_safe_relative_path_rejects_windows_rooted_path() {
    let err = super::stages::ensure_safe_relative_path(Path::new(r"\escape.wav")).unwrap_err();
    assert!(matches!(err, super::AudioLoadError::Failed(_)));
}

#[test]
fn ensure_safe_relative_path_accepts_normal_relative_paths() {
    super::stages::ensure_safe_relative_path(Path::new("folder/./file.wav")).unwrap();
}

#[test]
fn drain_to_latest_job_keeps_most_recent_request() {
    let (tx, rx) = std::sync::mpsc::channel::<AudioLoadJob>();
    tx.send(test_job(2, "two.wav")).unwrap();
    tx.send(test_job(3, "three.wav")).unwrap();
    let drained = drain_to_latest_job(test_job(1, "one.wav"), &rx);
    assert_eq!(drained.request_id, 3);
    assert_eq!(drained.relative_path, Path::new("three.wav"));
}

#[test]
fn stale_request_detection_ignores_zero_and_matches_latest_only() {
    let latest = AtomicU64::new(0);
    assert!(!is_stale_request(1, &latest));
    latest.store(5, std::sync::atomic::Ordering::Relaxed);
    assert!(is_stale_request(4, &latest));
    assert!(!is_stale_request(5, &latest));
}

#[test]
/// Stale request checks should short-circuit before filesystem work in the IO stage.
fn load_audio_inner_drops_stale_pre_io() {
    let renderer = WaveformRenderer::new(16, 16);
    let latest = AtomicU64::new(99);
    let result = super::stages::load_audio_inner(&renderer, &test_job(1, "still.wav"), &latest)
        .expect("stale pre-io path should not error");
    assert!(result.is_none());
}

#[test]
/// Chunked reader should stop and return `None` once stale checks flip true.
fn chunked_read_aborts_when_stale_mid_stream() {
    let payload = vec![1u8; super::stages::AUDIO_LOADER_READ_CHUNK_BYTES * 2];
    let mut stale_checks = 0u32;
    let result =
        super::stages::read_bytes_chunked_with_stale_check(Cursor::new(payload), 0, || {
            stale_checks = stale_checks.saturating_add(1);
            stale_checks >= 2
        });
    assert!(result.is_ok());
    let Some(result) = result.ok() else {
        return;
    };
    assert!(result.is_none());
}

#[test]
/// Chunked reader should return complete payload when stale checks stay false.
fn chunked_read_returns_payload_when_not_stale() {
    let payload = vec![7u8; super::stages::AUDIO_LOADER_READ_CHUNK_BYTES + 9];
    let result =
        super::stages::read_bytes_chunked_with_stale_check(Cursor::new(payload.clone()), 0, || {
            false
        });
    assert!(result.is_ok());
    let Some(result) = result.ok() else {
        return;
    };
    assert_eq!(result, Some(payload));
}

#[test]
fn load_audio_inner_applies_stretch_ratio_and_returns_stretched_payload() {
    let renderer = WaveformRenderer::new(16, 16);
    let latest = AtomicU64::new(1);
    let samples: Vec<f32> = (0..16_384).map(generated_audio_sample).collect();
    let wav_bytes = build_float_wav(&samples, 1, 48_000);
    let temp = write_test_wav(&wav_bytes);
    let relative_path = PathBuf::from(temp.path().file_name().expect("temp filename"));
    let job = test_job_with_root(
        1,
        temp.path().parent().expect("temp parent"),
        &relative_path,
        Some(1.5),
    );

    let outcome = super::stages::load_audio_inner(&renderer, &job, &latest)
        .expect("stretch load should succeed")
        .expect("stretch load should produce output");

    assert!(outcome.stretched);
    assert_eq!(outcome.metadata.file_size, wav_bytes.len() as u64);
    assert!(!outcome.decoded.samples.is_empty());
    assert_ne!(outcome.bytes.as_ref(), wav_bytes.as_slice());
}

#[test]
fn run_stretch_stage_drops_result_when_request_turns_stale_after_stretch() {
    let renderer = WaveformRenderer::new(16, 16);
    let latest = AtomicU64::new(7);
    let samples: Vec<f32> = (0..8_192).map(generated_audio_sample).collect();
    let wav_bytes = build_float_wav(&samples, 1, 48_000);
    let decoded = decode_test_waveform(&renderer, &wav_bytes);
    let job = AudioLoadJob {
        request_id: 7,
        source_id: crate::sample_sources::SourceId::from_string("source"),
        root: PathBuf::from("/tmp"),
        relative_path: PathBuf::from("stretch.wav"),
        stretch_ratio: Some(1.25),
        render_spec: render_spec(),
        prepared: None,
    };

    let result = super::stages::run_stretch_stage_for_test(
        &renderer,
        &job,
        &latest,
        decoded,
        Arc::<[u8]>::from(wav_bytes),
        || latest.store(99, Ordering::Relaxed),
    )
    .expect("stretch stage should not error");

    assert!(result.is_none());
}

#[test]
fn build_transient_result_propagates_metadata_and_stretch_state() {
    let renderer = WaveformRenderer::new(16, 16);
    let latest = AtomicU64::new(11);
    let samples: Vec<f32> = (0..8_192).map(generated_audio_sample).collect();
    let wav_bytes = build_float_wav(&samples, 1, 48_000);
    let decoded = decode_test_waveform(&renderer, &wav_bytes);
    let cache_token = decoded.cache_token;
    let pending = super::PendingTransientCompute {
        request_id: 11,
        source_id: crate::sample_sources::SourceId::from_string("source"),
        relative_path: PathBuf::from("transients.wav"),
        metadata: test_metadata(wav_bytes.len()),
        cache_token,
        decoded,
        stretched: true,
    };

    let result = super::stages::build_transient_result_for_test(pending, &latest, || {})
        .expect("transient result should be produced");

    assert_eq!(result.request_id, 11);
    assert_eq!(result.relative_path, Path::new("transients.wav"));
    assert_eq!(result.metadata.file_size, wav_bytes.len() as u64);
    assert_eq!(result.cache_token, cache_token);
    assert!(result.stretched);
}

#[test]
fn build_transient_result_drops_result_when_request_turns_stale_after_transients() {
    let renderer = WaveformRenderer::new(16, 16);
    let latest = AtomicU64::new(12);
    let samples: Vec<f32> = (0..8_192).map(generated_audio_sample).collect();
    let wav_bytes = build_float_wav(&samples, 1, 48_000);
    let decoded = decode_test_waveform(&renderer, &wav_bytes);
    let pending = super::PendingTransientCompute {
        request_id: 12,
        source_id: crate::sample_sources::SourceId::from_string("source"),
        relative_path: PathBuf::from("transients.wav"),
        metadata: test_metadata(wav_bytes.len()),
        cache_token: decoded.cache_token,
        decoded,
        stretched: false,
    };

    let result = super::stages::build_transient_result_for_test(pending, &latest, || {
        latest.store(99, Ordering::Relaxed);
    });

    assert!(result.is_none());
}
