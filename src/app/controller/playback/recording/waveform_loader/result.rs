//! Recording waveform result payloads and error types.

use super::*;

/// Result of a recording waveform refresh operation.
#[derive(Clone, Debug)]
pub(crate) enum RecordingWaveformUpdate {
    /// The file length did not change since the last refresh.
    NoChange { file_len: u64 },
    /// A new waveform was decoded from the recording file.
    Updated {
        decoded: DecodedWaveform,
        bytes: Option<Vec<u8>>,
        file_len: u64,
    },
}

/// Errors encountered while refreshing a recording waveform.
#[derive(Debug)]
pub(crate) enum RecordingWaveformError {
    /// The recording file is missing.
    Missing,
    /// The recording file failed to load.
    Failed,
    /// The recording file could not be decoded.
    DecodeFailed,
}

/// Completed recording waveform refresh response.
#[derive(Debug)]
pub(crate) struct RecordingWaveformLoadResult {
    pub(crate) request_id: u64,
    pub(crate) source_id: SourceId,
    pub(crate) relative_path: PathBuf,
    pub(crate) result: Result<RecordingWaveformUpdate, RecordingWaveformError>,
}
