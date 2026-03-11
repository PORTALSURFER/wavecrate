//! Incremental-update orchestration for recording waveform refresh jobs.

use super::*;

pub(super) fn load_recording_waveform(job: RecordingWaveformJob) -> RecordingWaveformLoadResult {
    let metadata = match read_recording_metadata(&job) {
        Ok(metadata) => metadata,
        Err(result) => return result,
    };
    let file_len = metadata.len();
    if file_len == job.last_file_len {
        return load_result_no_change(&job, file_len);
    }
    if file_len == 0 {
        return load_result_updated(
            &job,
            empty_recording_waveform(job.sample_rate, job.channels),
            None,
            file_len,
        );
    }

    let key = recording_state_key(&job);
    let state = take_recording_state(&key, file_len, job.last_file_len);
    if !job.loaded_once {
        return load_full_recording_waveform(job, key, file_len);
    }
    load_incremental_recording_waveform(job, key, state, file_len)
}

fn load_full_recording_waveform(
    job: RecordingWaveformJob,
    key: RecordingWaveformKey,
    file_len: u64,
) -> RecordingWaveformLoadResult {
    let bytes = match read_recording_bytes(&job) {
        Ok(bytes) => bytes,
        Err(result) => return result,
    };
    let data_offset = match find_wav_data_chunk(&bytes) {
        Some(offset) => offset,
        None => return load_result_error(&job, RecordingWaveformError::DecodeFailed),
    };
    let data_len = bytes.len().saturating_sub(data_offset) as u64;
    let total_frames = total_frames_for_data(data_len, job.channels);
    let mut next_state = RecordingWaveformState::new(job.sample_rate, job.channels, data_offset);
    next_state.prepare_for_total_frames(total_frames);
    next_state.consume_data_bytes(&bytes[data_offset..]);
    convert_state_to_peaks_if_needed(&mut next_state);
    let decoded = next_state.to_decoded();
    restore_recording_state(key, next_state);
    load_result_updated(&job, decoded, Some(bytes), file_len)
}

fn load_incremental_recording_waveform(
    job: RecordingWaveformJob,
    key: RecordingWaveformKey,
    mut state: Option<RecordingWaveformState>,
    file_len: u64,
) -> RecordingWaveformLoadResult {
    let mut file = match open_recording_file(&job) {
        Ok(file) => file,
        Err(result) => return result,
    };
    let data_offset = match resolve_data_offset(&mut file, file_len, state.as_ref()) {
        Ok(offset) => offset,
        Err(error) => return load_result_error(&job, error),
    };
    let data_len = file_len.saturating_sub(data_offset as u64);
    let total_frames = total_frames_for_data(data_len, job.channels);
    if total_frames == 0 {
        if let Some(state) = state {
            restore_recording_state(key, state);
        }
        return load_result_no_change(&job, file_len);
    }

    if state_requires_reset(state.as_ref(), &job, data_len, total_frames) {
        state = None;
    }
    let mut next_state =
        match prepare_next_state(&mut file, data_offset, data_len, total_frames, &job, state) {
            Ok(state) => state,
            Err(error) => return load_result_error(&job, error),
        };
    if let Err(error) = append_remaining_bytes(&mut file, &mut next_state, data_offset, data_len) {
        return load_result_error(&job, error);
    }
    convert_state_to_peaks_if_needed(&mut next_state);
    let decoded = next_state.to_decoded();
    restore_recording_state(key, next_state);
    load_result_updated(&job, decoded, None, file_len)
}

fn resolve_data_offset(
    file: &mut File,
    file_len: u64,
    state: Option<&RecordingWaveformState>,
) -> Result<usize, RecordingWaveformError> {
    match state.map(|cached| cached.data_offset) {
        Some(offset) => Ok(offset),
        None => read_wav_data_offset_from_file(file, file_len)
            .ok_or(RecordingWaveformError::DecodeFailed),
    }
}

fn state_requires_reset(
    state: Option<&RecordingWaveformState>,
    job: &RecordingWaveformJob,
    data_len: u64,
    total_frames: usize,
) -> bool {
    match state {
        Some(existing) => {
            existing.sample_rate != job.sample_rate
                || existing.channels != job.channels
                || existing.bytes_read > data_len
                || existing.total_frames > total_frames
        }
        None => false,
    }
}

fn prepare_next_state(
    file: &mut File,
    data_offset: usize,
    data_len: u64,
    total_frames: usize,
    job: &RecordingWaveformJob,
    state: Option<RecordingWaveformState>,
) -> Result<RecordingWaveformState, RecordingWaveformError> {
    match state {
        Some(state) if !state.requires_rebuild(total_frames) => Ok(state),
        _ => rebuild_state_from_file(
            file,
            data_offset,
            data_len,
            job.sample_rate,
            job.channels,
            total_frames,
        ),
    }
}

fn convert_state_to_peaks_if_needed(state: &mut RecordingWaveformState) {
    if matches!(state.mode, RecordingWaveformMode::Full { .. })
        && state.total_frames > RECORDING_MAX_FULL_FRAMES
    {
        state.convert_full_to_peaks();
    }
}
