use super::{AudioLoadJob, drain_to_latest_job, is_stale_request};
use crate::waveform::WaveformRenderer;
use std::io::Cursor;
use std::path::{Path, PathBuf};
use std::sync::atomic::AtomicU64;

fn test_job(request_id: u64, relative_path: &str) -> AudioLoadJob {
    AudioLoadJob {
        request_id,
        source_id: crate::sample_sources::SourceId::from_string("source"),
        root: PathBuf::from("/tmp"),
        relative_path: PathBuf::from(relative_path),
        stretch_ratio: None,
    }
}

#[test]
fn ensure_safe_relative_path_rejects_parent_dir() {
    let err = super::stages::ensure_safe_relative_path(Path::new("../escape.wav")).unwrap_err();
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
