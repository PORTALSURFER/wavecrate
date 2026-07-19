use crate::app::controller::library::analysis_jobs::ReadinessStageError;
use std::path::Path;

pub(crate) fn decode_for_readiness(
    absolute_path: &Path,
) -> Result<wavecrate_analysis::AnalysisAudio, ReadinessStageError> {
    wavecrate_analysis::decode_for_analysis_with_rate_limit_typed(
        absolute_path,
        wavecrate_analysis::ANALYSIS_SAMPLE_RATE,
        None,
    )
    .map_err(ReadinessStageError::Decode)
}
