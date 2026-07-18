use serde::{Deserialize, Serialize};

use super::super::config_defaults::{
    default_analysis_worker_count, default_long_sample_threshold_seconds,
    default_max_analysis_duration_seconds,
};

/// Global preferences for analysis and feature extraction.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnalysisSettings {
    /// Skip analysis for files longer than this many seconds.
    #[serde(default = "default_max_analysis_duration_seconds")]
    pub max_analysis_duration_seconds: f32,
    /// Threshold in seconds above which samples are marked as long in the browser.
    #[serde(default = "default_long_sample_threshold_seconds")]
    pub long_sample_threshold_seconds: f32,
    /// Analysis worker count override (0 = auto).
    #[serde(default = "default_analysis_worker_count")]
    pub analysis_worker_count: u32,
}

impl Default for AnalysisSettings {
    fn default() -> Self {
        Self {
            max_analysis_duration_seconds: default_max_analysis_duration_seconds(),
            long_sample_threshold_seconds: default_long_sample_threshold_seconds(),
            analysis_worker_count: default_analysis_worker_count(),
        }
    }
}
