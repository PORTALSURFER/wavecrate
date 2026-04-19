//! Background normalization DTOs for destructive waveform cleanup work.

use super::*;

/// Request to normalize one sample file in the background.
#[derive(Debug)]
pub(crate) struct NormalizationJob {
    pub(crate) source: crate::sample_sources::SampleSource,
    pub(crate) relative_path: PathBuf,
    pub(crate) absolute_path: PathBuf,
}

/// Result of one background normalization attempt.
#[derive(Debug)]
pub(crate) struct NormalizationResult {
    pub(crate) source_id: crate::sample_sources::SourceId,
    pub(crate) relative_path: PathBuf,
    pub(crate) result: Result<
        (
            u64,
            i64,
            crate::sample_sources::Rating,
            crate::app::controller::undo::OverwriteBackup,
        ),
        String,
    >,
}
