use super::*;

const MIN_MAX_ANALYSIS_DURATION_SECONDS: f32 = 1.0;
const MAX_MAX_ANALYSIS_DURATION_SECONDS: f32 = 60.0 * 60.0;
const MIN_LONG_SAMPLE_THRESHOLD_SECONDS: f32 = 1.0;
const MAX_LONG_SAMPLE_THRESHOLD_SECONDS: f32 = 60.0 * 60.0;

pub(crate) fn clamp_max_analysis_duration_seconds(seconds: f32) -> f32 {
    seconds.clamp(
        MIN_MAX_ANALYSIS_DURATION_SECONDS,
        MAX_MAX_ANALYSIS_DURATION_SECONDS,
    )
}

pub(crate) fn clamp_long_sample_threshold_seconds(seconds: f32) -> f32 {
    seconds.clamp(
        MIN_LONG_SAMPLE_THRESHOLD_SECONDS,
        MAX_LONG_SAMPLE_THRESHOLD_SECONDS,
    )
}

impl AppController {
    /// Return the maximum analysis duration in seconds.
    pub fn max_analysis_duration_seconds(&self) -> f32 {
        self.settings.analysis.max_analysis_duration_seconds
    }

    /// Return the threshold for marking long samples in the browser.
    pub fn long_sample_threshold_seconds(&self) -> f32 {
        self.settings.analysis.long_sample_threshold_seconds
    }

    /// Set the maximum analysis duration in seconds.
    pub fn set_max_analysis_duration_seconds(&mut self, seconds: f32) {
        let clamped = clamp_max_analysis_duration_seconds(seconds);
        if (self.settings.analysis.max_analysis_duration_seconds - clamped).abs() < f32::EPSILON {
            return;
        }
        self.settings.analysis.max_analysis_duration_seconds = clamped;
        if let Err(err) = self.persist_config("Failed to save options") {
            self.set_status(err, StatusTone::Warning);
        }
    }

    /// Set the threshold for marking long samples in the browser.
    pub fn set_long_sample_threshold_seconds(&mut self, seconds: f32) {
        let clamped = clamp_long_sample_threshold_seconds(seconds);
        if (self.settings.analysis.long_sample_threshold_seconds - clamped).abs() < f32::EPSILON {
            return;
        }
        self.settings.analysis.long_sample_threshold_seconds = clamped;
        if let Err(err) = self.persist_config("Failed to save options") {
            self.set_status(err, StatusTone::Warning);
        }
    }
}
