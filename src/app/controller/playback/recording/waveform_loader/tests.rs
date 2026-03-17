use super::*;
use crate::waveform::WaveformRenderer;
use hound::{SampleFormat, WavSpec, WavWriter};
use std::io::Write;
use std::sync::{Arc, Mutex};
use std::thread;
use tempfile::NamedTempFile;

static RECORDING_STATE_LOCK: Mutex<()> = Mutex::new(());

fn build_minimal_wav(sample: f32) -> Vec<u8> {
    let mut bytes = Vec::new();
    let data_bytes = sample.to_le_bytes();
    let chunk_size = 4u32 + 8u32 + data_bytes.len() as u32;
    bytes.extend_from_slice(b"RIFF");
    bytes.extend_from_slice(&chunk_size.to_le_bytes());
    bytes.extend_from_slice(b"WAVE");
    bytes.extend_from_slice(b"data");
    bytes.extend_from_slice(&(data_bytes.len() as u32).to_le_bytes());
    bytes.extend_from_slice(&data_bytes);
    bytes
}

fn build_wav_bytes(data_bytes: &[u8]) -> Vec<u8> {
    let mut bytes = Vec::new();
    let chunk_size = 4u32 + 8u32 + data_bytes.len() as u32;
    bytes.extend_from_slice(b"RIFF");
    bytes.extend_from_slice(&chunk_size.to_le_bytes());
    bytes.extend_from_slice(b"WAVE");
    bytes.extend_from_slice(b"data");
    bytes.extend_from_slice(&(data_bytes.len() as u32).to_le_bytes());
    bytes.extend_from_slice(data_bytes);
    bytes
}

fn build_wav_samples(samples: &[f32]) -> Vec<u8> {
    let mut data = Vec::with_capacity(samples.len() * 4);
    for sample in samples {
        data.extend_from_slice(&sample.to_le_bytes());
    }
    build_wav_bytes(&data)
}

fn generated_sample(frame: usize, channel: usize) -> f32 {
    let lane_scale = if channel == 0 { 0.82 } else { -0.67 };
    let base = (((frame * (channel + 3)) % 257) as f32 / 96.0) - 1.2;
    let accent = if frame % 4096 == (channel * 137) {
        1.25
    } else if frame % 4096 == (2048 + channel * 29) {
        -1.25
    } else {
        0.0
    };
    (base * lane_scale) + accent
}

fn build_long_recording_fixture(frame_count: usize, channels: u16, sample_rate: u32) -> Vec<u8> {
    let spec = WavSpec {
        channels,
        sample_rate,
        bits_per_sample: 32,
        sample_format: SampleFormat::Float,
    };
    let temp = NamedTempFile::new().expect("tempfile");
    {
        let file = temp.reopen().expect("reopen wav");
        let mut writer = WavWriter::new(file, spec).expect("wav writer");
        for frame in 0..frame_count {
            for channel in 0..channels as usize {
                writer
                    .write_sample(generated_sample(frame, channel))
                    .expect("write sample");
            }
        }
        writer.finalize().expect("finalize wav");
    }
    std::fs::read(temp.path()).expect("read wav bytes")
}

fn recording_waveform_from_wav_bytes(
    wav_bytes: &[u8],
    sample_rate: u32,
    channels: u16,
) -> DecodedWaveform {
    let data_offset = find_wav_data_chunk(wav_bytes).expect("data chunk");
    let total_frames = total_frames_for_data(
        wav_bytes.len().saturating_sub(data_offset) as u64,
        channels,
    );
    let mut state = RecordingWaveformState::new(sample_rate, channels, data_offset);
    state.prepare_for_total_frames(total_frames);
    let consumed = state.consume_data_bytes(&wav_bytes[data_offset..]);
    assert_eq!(consumed, total_frames);
    if matches!(state.mode, RecordingWaveformMode::Full { .. })
        && state.total_frames > RECORDING_MAX_FULL_FRAMES
    {
        state.convert_full_to_peaks();
    }
    state.to_decoded()
}

fn decode_reference_waveform(wav_bytes: &[u8]) -> DecodedWaveform {
    WaveformRenderer::new(8, 8)
        .decode_from_bytes(wav_bytes)
        .expect("reference decode")
}

fn assert_slice_approx_eq(actual: &[f32], expected: &[f32]) {
    assert_eq!(actual.len(), expected.len());
    for (index, (actual, expected)) in actual.iter().zip(expected.iter()).enumerate() {
        assert!(
            (actual - expected).abs() < 1e-6,
            "sample mismatch at {index}: {actual} vs {expected}"
        );
    }
}

fn assert_peak_pairs_approx_eq(actual: &[(f32, f32)], expected: &[(f32, f32)]) {
    assert_eq!(actual.len(), expected.len());
    for (index, (actual, expected)) in actual.iter().zip(expected.iter()).enumerate() {
        assert!(
            (actual.0 - expected.0).abs() < 1e-6,
            "min peak mismatch at {index}: {} vs {}",
            actual.0,
            expected.0
        );
        assert!(
            (actual.1 - expected.1).abs() < 1e-6,
            "max peak mismatch at {index}: {} vs {}",
            actual.1,
            expected.1
        );
    }
}

fn assert_decoded_peak_parity(actual: &DecodedWaveform, expected: &DecodedWaveform) {
    assert_eq!(actual.frame_count(), expected.frame_count());
    assert_eq!(actual.sample_rate, expected.sample_rate);
    assert_eq!(actual.channels, expected.channels);
    assert_eq!(actual.analysis_stride, expected.analysis_stride);
    assert_eq!(actual.analysis_sample_rate, expected.analysis_sample_rate);
    assert!(
        (actual.duration_seconds - expected.duration_seconds).abs() < 1e-6,
        "duration mismatch: {} vs {}",
        actual.duration_seconds,
        expected.duration_seconds
    );
    assert_slice_approx_eq(&actual.samples, &expected.samples);
    assert_slice_approx_eq(&actual.analysis_samples, &expected.analysis_samples);

    let actual_peaks = actual.peaks.as_deref().expect("recording peaks");
    let expected_peaks = expected.peaks.as_deref().expect("reference peaks");
    assert_eq!(actual_peaks.total_frames, expected_peaks.total_frames);
    assert_eq!(actual_peaks.channels, expected_peaks.channels);
    assert_eq!(actual_peaks.bucket_size_frames, expected_peaks.bucket_size_frames);
    assert_peak_pairs_approx_eq(&actual_peaks.mono, &expected_peaks.mono);
    match (&actual_peaks.left, &expected_peaks.left) {
        (Some(actual), Some(expected)) => assert_peak_pairs_approx_eq(actual, expected),
        (None, None) => {}
        _ => panic!("left-channel peak parity mismatch"),
    }
    match (&actual_peaks.right, &expected_peaks.right) {
        (Some(actual), Some(expected)) => assert_peak_pairs_approx_eq(actual, expected),
        (None, None) => {}
        _ => panic!("right-channel peak parity mismatch"),
    }
}

#[test]
fn recording_waveform_queue_replaces_pending() {
    let queue = RecordingWaveformJobQueue::new();
    let job = RecordingWaveformJob {
        request_id: 1,
        source_id: SourceId::from_string("source"),
        relative_path: PathBuf::from("one.wav"),
        absolute_path: PathBuf::from("/tmp/one.wav"),
        last_file_len: 0,
        loaded_once: false,
        sample_rate: 48_000,
        channels: 1,
    };
    let newer = RecordingWaveformJob {
        request_id: 2,
        source_id: SourceId::from_string("source"),
        relative_path: PathBuf::from("two.wav"),
        absolute_path: PathBuf::from("/tmp/two.wav"),
        last_file_len: 0,
        loaded_once: false,
        sample_rate: 48_000,
        channels: 1,
    };
    queue.send(job);
    queue.send(newer.clone());
    let pending = queue.try_take().expect("expected pending job");
    assert_eq!(pending.request_id, newer.request_id);
    assert_eq!(pending.relative_path, newer.relative_path);
}

#[test]
fn recording_waveform_queue_shutdown_unblocks() {
    let queue = Arc::new(RecordingWaveformJobQueue::new());
    let (tx, rx) = std::sync::mpsc::channel();
    let queue_worker = Arc::clone(&queue);
    let handle = thread::spawn(move || {
        let result = queue_worker.take_blocking();
        tx.send(result.is_none()).expect("send result");
    });
    queue.shutdown();
    let shutdown = rx
        .recv_timeout(std::time::Duration::from_secs(1))
        .expect("shutdown signal");
    assert!(shutdown);
    handle.join().expect("worker thread panicked");
}

#[test]
fn decode_recording_waveform_ignores_partial_frames() {
    let mut data = Vec::new();
    for sample in [0.1_f32, -0.2_f32, 0.3_f32] {
        data.extend_from_slice(&sample.to_le_bytes());
    }
    let bytes = build_wav_bytes(&data);
    let decoded = decode_recording_waveform(&bytes, 48_000, 2).expect("expected waveform");
    assert_eq!(decoded.samples.len(), 2);
    assert!((decoded.samples[0] - 0.1).abs() < 1e-6);
    assert!((decoded.samples[1] + 0.2).abs() < 1e-6);
}

#[test]
fn load_recording_waveform_decodes_updated_file() {
    let _guard = RECORDING_STATE_LOCK.lock().unwrap();
    clear_recording_state();
    let bytes = build_minimal_wav(0.5);
    let mut temp = NamedTempFile::new().expect("tempfile");
    temp.write_all(&bytes).expect("write wav");
    let path = temp.path().to_path_buf();
    let job = RecordingWaveformJob {
        request_id: 10,
        source_id: SourceId::from_string("source"),
        relative_path: PathBuf::from("recording.wav"),
        absolute_path: path,
        last_file_len: 0,
        loaded_once: false,
        sample_rate: 48_000,
        channels: 1,
    };
    let result = load_recording_waveform(job);
    let update = result.result.expect("expected update");
    match update {
        RecordingWaveformUpdate::Updated {
            decoded,
            bytes,
            file_len,
        } => {
            assert!(decoded.duration_seconds > 0.0);
            assert!(bytes.is_some());
            assert!(file_len > 0);
        }
        RecordingWaveformUpdate::NoChange { .. } => {
            panic!("expected updated waveform");
        }
    }
}

#[test]
fn load_recording_waveform_handles_truncation() {
    let _guard = RECORDING_STATE_LOCK.lock().unwrap();
    clear_recording_state();
    let bytes = build_minimal_wav(0.25);
    let mut temp = NamedTempFile::new().expect("tempfile");
    temp.write_all(&bytes).expect("write wav");
    let file_len = bytes.len() as u64;
    let path = temp.path().to_path_buf();
    let job = RecordingWaveformJob {
        request_id: 20,
        source_id: SourceId::from_string("source"),
        relative_path: PathBuf::from("recording.wav"),
        absolute_path: path.clone(),
        last_file_len: 0,
        loaded_once: true,
        sample_rate: 48_000,
        channels: 1,
    };
    let result = load_recording_waveform(job);
    let update = result.result.expect("expected update");
    match update {
        RecordingWaveformUpdate::Updated { decoded, .. } => {
            assert_eq!(decoded.frame_count(), 1);
        }
        _ => panic!("expected updated waveform"),
    }

    temp.as_file_mut().set_len(0).expect("truncate wav");
    let truncated_job = RecordingWaveformJob {
        request_id: 21,
        source_id: SourceId::from_string("source"),
        relative_path: PathBuf::from("recording.wav"),
        absolute_path: path,
        last_file_len: file_len,
        loaded_once: true,
        sample_rate: 48_000,
        channels: 1,
    };
    let truncated_result = load_recording_waveform(truncated_job);
    let update = truncated_result.result.expect("expected update");
    match update {
        RecordingWaveformUpdate::Updated {
            decoded, file_len, ..
        } => {
            assert_eq!(file_len, 0);
            assert_eq!(decoded.frame_count(), 0);
        }
        _ => panic!("expected updated waveform"),
    }
}

#[test]
fn load_recording_waveform_appends_incrementally() {
    let _guard = RECORDING_STATE_LOCK.lock().unwrap();
    clear_recording_state();
    let mut temp = NamedTempFile::new().expect("tempfile");
    let first = build_wav_samples(&[0.25]);
    temp.write_all(&first).expect("write wav");
    let path = temp.path().to_path_buf();
    let first_len = first.len() as u64;

    let first_job = RecordingWaveformJob {
        request_id: 30,
        source_id: SourceId::from_string("source"),
        relative_path: PathBuf::from("recording.wav"),
        absolute_path: path.clone(),
        last_file_len: 0,
        loaded_once: false,
        sample_rate: 48_000,
        channels: 1,
    };
    let first_result = load_recording_waveform(first_job);
    let first_update = first_result.result.expect("expected update");
    let RecordingWaveformUpdate::Updated { decoded, .. } = first_update else {
        panic!("expected updated waveform");
    };
    assert_eq!(decoded.frame_count(), 1);

    let second = build_wav_samples(&[0.25, -0.5]);
    temp.as_file_mut().set_len(0).expect("truncate wav");
    temp.as_file_mut()
        .seek(SeekFrom::Start(0))
        .expect("seek wav");
    temp.write_all(&second).expect("write wav");

    let second_job = RecordingWaveformJob {
        request_id: 31,
        source_id: SourceId::from_string("source"),
        relative_path: PathBuf::from("recording.wav"),
        absolute_path: path,
        last_file_len: first_len,
        loaded_once: true,
        sample_rate: 48_000,
        channels: 1,
    };
    let second_result = load_recording_waveform(second_job);
    let second_update = second_result.result.expect("expected update");
    let RecordingWaveformUpdate::Updated { decoded, .. } = second_update else {
        panic!("expected updated waveform");
    };
    assert_eq!(decoded.frame_count(), 2);
}

#[test]
fn recording_waveform_queue_recovers_after_poisoned_lock() {
    let queue = RecordingWaveformJobQueue::new();
    let job = RecordingWaveformJob {
        request_id: 42,
        source_id: SourceId::from_string("source"),
        relative_path: PathBuf::from("recover.wav"),
        absolute_path: PathBuf::from("/tmp/recover.wav"),
        last_file_len: 0,
        loaded_once: false,
        sample_rate: 48_000,
        channels: 1,
    };
    let _ = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        let _guard = queue.state.lock().expect("poison queue lock");
        panic!("poison queue lock for test");
    }));
    queue.send(job.clone());
    let pending = queue.try_take().expect("expected pending job");
    assert_eq!(pending.request_id, job.request_id);
    assert_eq!(pending.relative_path, job.relative_path);
}

#[test]
fn long_mono_recording_waveform_matches_reference_decode_peaks_and_analysis() {
    let wav_bytes = build_long_recording_fixture(RECORDING_MAX_FULL_FRAMES + 1, 1, 48_000);
    let recording = recording_waveform_from_wav_bytes(&wav_bytes, 48_000, 1);
    let reference = decode_reference_waveform(&wav_bytes);

    assert_decoded_peak_parity(&recording, &reference);
}

#[test]
fn long_stereo_recording_waveform_matches_reference_decode_peaks_and_analysis() {
    let wav_bytes = build_long_recording_fixture(RECORDING_MAX_FULL_FRAMES + 1, 2, 48_000);
    let recording = recording_waveform_from_wav_bytes(&wav_bytes, 48_000, 2);
    let reference = decode_reference_waveform(&wav_bytes);

    assert_decoded_peak_parity(&recording, &reference);
}

#[test]
fn incremental_recording_growth_matches_reference_after_full_to_peaks_transition() {
    let total_frames = RECORDING_MAX_FULL_FRAMES + 1;
    let wav_bytes = build_long_recording_fixture(total_frames, 1, 48_000);
    let data_offset = find_wav_data_chunk(&wav_bytes).expect("data chunk");
    let split_offset = data_offset + (RECORDING_MAX_FULL_FRAMES * 4);
    let mut state = RecordingWaveformState::new(48_000, 1, data_offset);
    state.prepare_for_total_frames(RECORDING_MAX_FULL_FRAMES);

    let consumed = state.consume_data_bytes(&wav_bytes[data_offset..split_offset]);
    assert_eq!(consumed, RECORDING_MAX_FULL_FRAMES);
    assert!(matches!(state.mode, RecordingWaveformMode::Full { .. }));

    let consumed = state.consume_data_bytes(&wav_bytes[split_offset..]);
    assert_eq!(consumed, 1);
    assert!(matches!(state.mode, RecordingWaveformMode::Full { .. }));

    state.convert_full_to_peaks();
    let reference = decode_reference_waveform(&wav_bytes);

    assert_decoded_peak_parity(&state.to_decoded(), &reference);
}
