//! Background worker for recording waveform refresh tasks.

use super::{RECORDING_MAX_FULL_FRAMES, RECORDING_MAX_PEAK_BUCKETS};
use crate::sample_sources::SourceId;
use crate::waveform::{DecodedWaveform, WaveformPeaks};
use std::collections::HashMap;
use std::fs::File;
use std::io::{Read, Seek, SeekFrom};
use std::path::PathBuf;
use std::sync::{Arc, Condvar, Mutex, OnceLock, mpsc::Receiver};
use std::{fs, thread};
use tracing::warn;

/// Waveform aggregation state, cache map, and peak-analysis helpers.
mod aggregation;
/// WAV decode and rebuild helpers for full/incremental refresh paths.
mod decode;
/// Worker queue and sender/handle lifecycle helpers.
mod queue;
/// Public result/update/error payload types.
mod result;

use self::aggregation::*;
#[cfg(test)]
use self::decode::decode_recording_waveform;
use self::decode::*;
#[cfg(test)]
use self::queue::RecordingWaveformJobQueue;
pub(crate) use self::queue::{
    RecordingWaveformJobSender, RecordingWaveformWorkerHandle, spawn_recording_waveform_loader,
};
pub(crate) use self::result::{
    RecordingWaveformError, RecordingWaveformLoadResult, RecordingWaveformUpdate,
};

/// Request data needed to refresh a recording waveform off the UI thread.
#[derive(Clone, Debug)]
pub(crate) struct RecordingWaveformJob {
    pub(crate) request_id: u64,
    pub(crate) source_id: SourceId,
    pub(crate) relative_path: PathBuf,
    pub(crate) absolute_path: PathBuf,
    pub(crate) last_file_len: u64,
    pub(crate) loaded_once: bool,
    pub(crate) sample_rate: u32,
    pub(crate) channels: u16,
}

fn load_recording_waveform(job: RecordingWaveformJob) -> RecordingWaveformLoadResult {
    let metadata = match fs::metadata(&job.absolute_path) {
        Ok(metadata) => metadata,
        Err(err) => {
            let missing = err.kind() == std::io::ErrorKind::NotFound;
            let message = if missing {
                RecordingWaveformError::Missing
            } else {
                RecordingWaveformError::Failed
            };
            return RecordingWaveformLoadResult {
                request_id: job.request_id,
                source_id: job.source_id,
                relative_path: job.relative_path,
                result: Err(message),
            };
        }
    };
    let file_len = metadata.len();
    if file_len == job.last_file_len {
        return RecordingWaveformLoadResult {
            request_id: job.request_id,
            source_id: job.source_id,
            relative_path: job.relative_path,
            result: Ok(RecordingWaveformUpdate::NoChange { file_len }),
        };
    }
    if file_len == 0 {
        return RecordingWaveformLoadResult {
            request_id: job.request_id,
            source_id: job.source_id,
            relative_path: job.relative_path,
            result: Ok(RecordingWaveformUpdate::Updated {
                decoded: empty_recording_waveform(job.sample_rate, job.channels),
                bytes: None,
                file_len,
            }),
        };
    }

    let key = RecordingWaveformKey {
        source_id: job.source_id.clone(),
        relative_path: job.relative_path.clone(),
    };
    let mut state = {
        let mut guard = recording_state_map()
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        guard.remove(&key)
    };
    if file_len < job.last_file_len {
        state = None;
    }

    if !job.loaded_once {
        let bytes = match fs::read(&job.absolute_path) {
            Ok(bytes) => bytes,
            Err(err) => {
                let missing = err.kind() == std::io::ErrorKind::NotFound;
                let message = if missing {
                    RecordingWaveformError::Missing
                } else {
                    RecordingWaveformError::Failed
                };
                return RecordingWaveformLoadResult {
                    request_id: job.request_id,
                    source_id: job.source_id,
                    relative_path: job.relative_path,
                    result: Err(message),
                };
            }
        };
        let data_offset = match find_wav_data_chunk(&bytes) {
            Some(offset) => offset,
            None => {
                return RecordingWaveformLoadResult {
                    request_id: job.request_id,
                    source_id: job.source_id,
                    relative_path: job.relative_path,
                    result: Err(RecordingWaveformError::DecodeFailed),
                };
            }
        };
        let data_len = bytes.len().saturating_sub(data_offset) as u64;
        let total_frames = total_frames_for_data(data_len, job.channels);
        let mut next_state =
            RecordingWaveformState::new(job.sample_rate, job.channels, data_offset);
        next_state.prepare_for_total_frames(total_frames);
        next_state.consume_data_bytes(&bytes[data_offset..]);
        if matches!(next_state.mode, RecordingWaveformMode::Full { .. })
            && next_state.total_frames > RECORDING_MAX_FULL_FRAMES
        {
            next_state.convert_full_to_peaks();
        }
        let decoded = next_state.to_decoded();
        let mut guard = recording_state_map()
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        guard.insert(key, next_state);
        return RecordingWaveformLoadResult {
            request_id: job.request_id,
            source_id: job.source_id,
            relative_path: job.relative_path,
            result: Ok(RecordingWaveformUpdate::Updated {
                decoded,
                bytes: Some(bytes),
                file_len,
            }),
        };
    }

    let mut file = match File::open(&job.absolute_path) {
        Ok(file) => file,
        Err(err) => {
            let missing = err.kind() == std::io::ErrorKind::NotFound;
            let message = if missing {
                RecordingWaveformError::Missing
            } else {
                RecordingWaveformError::Failed
            };
            return RecordingWaveformLoadResult {
                request_id: job.request_id,
                source_id: job.source_id,
                relative_path: job.relative_path,
                result: Err(message),
            };
        }
    };

    let data_offset = match state.as_ref().map(|s| s.data_offset) {
        Some(offset) => offset,
        None => match read_wav_data_offset_from_file(&mut file, file_len) {
            Some(offset) => offset,
            None => {
                return RecordingWaveformLoadResult {
                    request_id: job.request_id,
                    source_id: job.source_id,
                    relative_path: job.relative_path,
                    result: Err(RecordingWaveformError::DecodeFailed),
                };
            }
        },
    };
    let data_len = file_len.saturating_sub(data_offset as u64);
    let total_frames = total_frames_for_data(data_len, job.channels);
    if total_frames == 0 {
        if let Some(state) = state {
            let mut guard = recording_state_map()
                .lock()
                .unwrap_or_else(|poisoned| poisoned.into_inner());
            guard.insert(key, state);
        }
        return RecordingWaveformLoadResult {
            request_id: job.request_id,
            source_id: job.source_id,
            relative_path: job.relative_path,
            result: Ok(RecordingWaveformUpdate::NoChange { file_len }),
        };
    }

    if let Some(existing) = &state
        && (existing.sample_rate != job.sample_rate
            || existing.channels != job.channels
            || existing.bytes_read > data_len
            || existing.total_frames > total_frames)
    {
        state = None;
    }

    let mut next_state = match state {
        Some(state) if !state.requires_rebuild(total_frames) => state,
        _ => match rebuild_state_from_file(
            &mut file,
            data_offset,
            data_len,
            job.sample_rate,
            job.channels,
            total_frames,
        ) {
            Ok(state) => state,
            Err(err) => {
                return RecordingWaveformLoadResult {
                    request_id: job.request_id,
                    source_id: job.source_id,
                    relative_path: job.relative_path,
                    result: Err(err),
                };
            }
        },
    };

    if next_state.bytes_read < data_len {
        let start = data_offset as u64 + next_state.bytes_read;
        if file.seek(SeekFrom::Start(start)).is_err() {
            return RecordingWaveformLoadResult {
                request_id: job.request_id,
                source_id: job.source_id,
                relative_path: job.relative_path,
                result: Err(RecordingWaveformError::Failed),
            };
        }
        let mut remaining = data_len.saturating_sub(next_state.bytes_read);
        let mut buf = vec![0u8; 64 * 1024];
        while remaining > 0 {
            let to_read = remaining.min(buf.len() as u64) as usize;
            let read = file.read(&mut buf[..to_read]).unwrap_or(0);
            if read == 0 {
                break;
            }
            next_state.consume_data_bytes(&buf[..read]);
            remaining = remaining.saturating_sub(read as u64);
        }
    }
    if matches!(next_state.mode, RecordingWaveformMode::Full { .. })
        && next_state.total_frames > RECORDING_MAX_FULL_FRAMES
    {
        next_state.convert_full_to_peaks();
    }
    let decoded = next_state.to_decoded();
    let mut guard = recording_state_map()
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    guard.insert(key, next_state);
    RecordingWaveformLoadResult {
        request_id: job.request_id,
        source_id: job.source_id,
        relative_path: job.relative_path,
        result: Ok(RecordingWaveformUpdate::Updated {
            decoded,
            bytes: None,
            file_len,
        }),
    }
}

#[cfg(test)]
mod tests {
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

    fn clear_recording_state() {
        let mut guard = recording_state_map()
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        guard.clear();
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
}
