//! Filesystem IO and result-shaping helpers for recording waveform refreshes.

use super::*;

pub(super) fn load_result_error(
    job: &RecordingWaveformJob,
    error: RecordingWaveformError,
) -> Box<RecordingWaveformLoadResult> {
    Box::new(RecordingWaveformLoadResult {
        request_id: job.request_id,
        source_id: job.source_id.clone(),
        relative_path: job.relative_path.clone(),
        result: Err(error),
    })
}

pub(super) fn load_result_no_change(
    job: &RecordingWaveformJob,
    file_len: u64,
) -> RecordingWaveformLoadResult {
    RecordingWaveformLoadResult {
        request_id: job.request_id,
        source_id: job.source_id.clone(),
        relative_path: job.relative_path.clone(),
        result: Ok(RecordingWaveformUpdate::NoChange { file_len }),
    }
}

pub(super) fn load_result_updated(
    job: &RecordingWaveformJob,
    decoded: DecodedWaveform,
    bytes: Option<Vec<u8>>,
    file_len: u64,
) -> RecordingWaveformLoadResult {
    RecordingWaveformLoadResult {
        request_id: job.request_id,
        source_id: job.source_id.clone(),
        relative_path: job.relative_path.clone(),
        result: Ok(RecordingWaveformUpdate::Updated {
            decoded,
            bytes,
            file_len,
        }),
    }
}

pub(super) fn map_file_error(err: std::io::Error) -> RecordingWaveformError {
    if err.kind() == std::io::ErrorKind::NotFound {
        RecordingWaveformError::Missing
    } else {
        RecordingWaveformError::Failed
    }
}

pub(super) fn read_recording_metadata(
    job: &RecordingWaveformJob,
) -> Result<std::fs::Metadata, Box<RecordingWaveformLoadResult>> {
    fs::metadata(&job.absolute_path).map_err(|err| load_result_error(job, map_file_error(err)))
}

pub(super) fn read_recording_bytes(
    job: &RecordingWaveformJob,
) -> Result<Vec<u8>, Box<RecordingWaveformLoadResult>> {
    fs::read(&job.absolute_path).map_err(|err| load_result_error(job, map_file_error(err)))
}

pub(super) fn open_recording_file(
    job: &RecordingWaveformJob,
) -> Result<File, Box<RecordingWaveformLoadResult>> {
    File::open(&job.absolute_path).map_err(|err| load_result_error(job, map_file_error(err)))
}

pub(super) fn append_remaining_bytes(
    file: &mut File,
    state: &mut RecordingWaveformState,
    data_offset: usize,
    data_len: u64,
) -> Result<(), RecordingWaveformError> {
    if state.bytes_read >= data_len {
        return Ok(());
    }
    let start = data_offset as u64 + state.bytes_read;
    file.seek(SeekFrom::Start(start))
        .map_err(|_| RecordingWaveformError::Failed)?;
    let mut remaining = data_len.saturating_sub(state.bytes_read);
    let mut buf = vec![0u8; 64 * 1024];
    while remaining > 0 {
        let to_read = remaining.min(buf.len() as u64) as usize;
        let read = file.read(&mut buf[..to_read]).unwrap_or(0);
        if read == 0 {
            break;
        }
        state.consume_data_bytes(&buf[..read]);
        remaining = remaining.saturating_sub(read as u64);
    }
    Ok(())
}
