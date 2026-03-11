use super::*;
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
