use super::*;

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
