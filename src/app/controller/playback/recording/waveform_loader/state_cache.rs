//! Retained recording waveform state-cache helpers.

use super::*;

pub(super) fn recording_state_key(job: &RecordingWaveformJob) -> RecordingWaveformKey {
    RecordingWaveformKey {
        source_id: job.source_id.clone(),
        relative_path: job.relative_path.clone(),
    }
}

pub(super) fn take_recording_state(
    key: &RecordingWaveformKey,
    file_len: u64,
    last_file_len: u64,
) -> Option<RecordingWaveformState> {
    let mut state = {
        let mut guard = recording_state_map()
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        guard.remove(key)
    };
    if file_len < last_file_len {
        state = None;
    }
    state
}

pub(super) fn restore_recording_state(key: RecordingWaveformKey, state: RecordingWaveformState) {
    let mut guard = recording_state_map()
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    guard.insert(key, state);
}

#[cfg(test)]
pub(super) fn clear_recording_state() {
    let mut guard = recording_state_map()
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    guard.clear();
}
