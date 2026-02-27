//! WAV decode and state-rebuild helpers used by incremental recording refreshes.

use super::*;

pub(super) fn decode_recording_waveform(
    bytes: &[u8],
    sample_rate: u32,
    channels: u16,
) -> Option<DecodedWaveform> {
    let data_offset = find_wav_data_chunk(bytes)?;
    let data_len = bytes.len().saturating_sub(data_offset) as u64;
    let total_frames = total_frames_for_data(data_len, channels);
    if total_frames == 0 {
        return None;
    }
    let mut state = RecordingWaveformState::new(sample_rate, channels, data_offset);
    state.prepare_for_total_frames(total_frames);
    state.consume_data_bytes(&bytes[data_offset..]);
    if matches!(state.mode, RecordingWaveformMode::Full { .. })
        && state.total_frames > RECORDING_MAX_FULL_FRAMES
    {
        state.convert_full_to_peaks();
    }
    Some(state.to_decoded())
}

pub(super) fn find_wav_data_chunk(bytes: &[u8]) -> Option<usize> {
    if bytes.len() < 12 {
        return None;
    }
    if &bytes[0..4] != b"RIFF" || &bytes[8..12] != b"WAVE" {
        return None;
    }
    let mut offset = 12usize;
    while offset + 8 <= bytes.len() {
        let id = &bytes[offset..offset + 4];
        let chunk_size = u32::from_le_bytes(bytes[offset + 4..offset + 8].try_into().ok()?);
        let data_start = offset + 8;
        if id == b"data" {
            return Some(data_start);
        }
        let mut next = data_start.saturating_add(chunk_size as usize);
        if chunk_size % 2 == 1 {
            next = next.saturating_add(1);
        }
        if next <= offset {
            break;
        }
        offset = next;
    }
    None
}

pub(super) fn read_wav_data_offset_from_file(file: &mut File, file_len: u64) -> Option<usize> {
    if file.seek(SeekFrom::Start(0)).is_err() {
        return None;
    }
    let max_read = file_len.min(64 * 1024) as usize;
    let mut header = vec![0u8; max_read];
    let read = file.read(&mut header).ok()?;
    header.truncate(read);
    find_wav_data_chunk(&header)
}

pub(super) fn total_frames_for_data(data_len: u64, channels: u16) -> usize {
    let channels = channels.max(1) as u64;
    let frame_bytes = 4u64 * channels;
    if frame_bytes == 0 {
        return 0;
    }
    (data_len / frame_bytes) as usize
}

pub(super) fn rebuild_state_from_file(
    file: &mut File,
    data_offset: usize,
    data_len: u64,
    sample_rate: u32,
    channels: u16,
    total_frames: usize,
) -> Result<RecordingWaveformState, RecordingWaveformError> {
    let mut state = RecordingWaveformState::new(sample_rate, channels, data_offset);
    state.prepare_for_total_frames(total_frames);
    if data_len == 0 {
        return Ok(state);
    }
    if file.seek(SeekFrom::Start(data_offset as u64)).is_err() {
        return Err(RecordingWaveformError::Failed);
    }
    let mut buf = vec![0u8; 64 * 1024];
    let mut remaining = data_len;
    while remaining > 0 {
        let to_read = remaining.min(buf.len() as u64) as usize;
        let read = file.read(&mut buf[..to_read]).unwrap_or(0);
        if read == 0 {
            break;
        }
        state.consume_data_bytes(&buf[..read]);
        remaining = remaining.saturating_sub(read as u64);
    }
    if matches!(state.mode, RecordingWaveformMode::Full { .. })
        && state.total_frames > RECORDING_MAX_FULL_FRAMES
    {
        state.convert_full_to_peaks();
    }
    Ok(state)
}
