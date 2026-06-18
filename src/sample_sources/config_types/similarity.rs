use serde::{Deserialize, Serialize};
use wavecrate_analysis::aspects::{ASPECT_COUNT, SimilarityAspect};

/// User controls for similarity aspect weighting and row-column presentation.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct SimilarityAspectSettings {
    /// Whether the active similarity ordering should use aspect weights.
    #[serde(default)]
    pub weighting_enabled: bool,
    /// Existing full-vector similarity control.
    #[serde(default)]
    pub overall: SimilarityAspectControl,
    /// Spectral-shape similarity control.
    #[serde(default)]
    pub spectrum: SimilarityAspectControl,
    /// Timbre-proxy similarity control.
    #[serde(default)]
    pub timbre: SimilarityAspectControl,
    /// Pitch-proxy similarity control.
    #[serde(default)]
    pub pitch: SimilarityAspectControl,
    /// Amplitude-envelope similarity control.
    #[serde(default)]
    pub amplitude: SimilarityAspectControl,
}

impl SimilarityAspectSettings {
    /// Return a copy of the control for one aspect.
    pub fn control(&self, aspect: SimilarityAspect) -> SimilarityAspectControl {
        match aspect {
            SimilarityAspect::Overall => self.overall,
            SimilarityAspect::Spectrum => self.spectrum,
            SimilarityAspect::Timbre => self.timbre,
            SimilarityAspect::Pitch => self.pitch,
            SimilarityAspect::Amplitude => self.amplitude,
        }
    }

    /// Return whether one aspect contributes to weighted similarity and normal columns.
    pub fn aspect_enabled(&self, aspect: SimilarityAspect) -> bool {
        self.control(aspect).enabled
    }

    /// Return aspect enablement in stable display order.
    pub fn aspect_enabled_flags(&self) -> [bool; ASPECT_COUNT] {
        std::array::from_fn(|index| self.aspect_enabled(SimilarityAspect::ORDER[index]))
    }

    /// Set the master weighting mode.
    pub fn set_weighting_enabled(&mut self, enabled: bool) {
        self.weighting_enabled = enabled;
    }

    /// Set whether one aspect contributes to weighted ranking.
    pub fn set_aspect_enabled(&mut self, aspect: SimilarityAspect, enabled: bool) {
        self.control_mut(aspect).enabled = enabled;
    }

    /// Set one aspect's normalized ranking weight.
    pub fn set_aspect_weight(&mut self, aspect: SimilarityAspect, weight: f32) {
        self.control_mut(aspect).weight = normalize_similarity_weight(weight);
    }

    /// Return a normalized copy suitable for runtime use after deserialization.
    pub fn normalized(mut self) -> Self {
        for aspect in SimilarityAspect::ORDER {
            let control = self.control_mut(aspect);
            control.weight = normalize_similarity_weight(control.weight);
        }
        self
    }

    /// Compute the score used for active similarity sorting.
    pub fn effective_score(
        &self,
        raw_score: Option<f32>,
        aspect_scores: &[Option<f32>; ASPECT_COUNT],
    ) -> Option<f32> {
        let raw_score = finite_clamped_score(raw_score)?;
        if !self.weighting_enabled {
            return Some(raw_score);
        }

        let mut weighted_sum = 0.0;
        let mut total_weight = 0.0;
        for aspect in SimilarityAspect::ORDER {
            let control = self.control(aspect);
            if !control.enabled || control.weight <= f32::EPSILON {
                continue;
            }
            let score = match aspect {
                SimilarityAspect::Overall => Some(raw_score),
                _ => finite_clamped_score(aspect_scores[aspect.index()]),
            };
            if let Some(score) = score {
                weighted_sum += score * control.weight;
                total_weight += control.weight;
            }
        }

        if total_weight <= f32::EPSILON {
            return Some(raw_score);
        }
        Some((weighted_sum / total_weight).clamp(-1.0, 1.0))
    }

    /// Return a stable key for invalidating caches that depend on controls.
    pub fn cache_key(&self) -> (bool, [bool; ASPECT_COUNT], [u32; ASPECT_COUNT]) {
        (
            self.weighting_enabled,
            self.aspect_enabled_flags(),
            std::array::from_fn(|index| self.control(SimilarityAspect::ORDER[index]).weight_bits()),
        )
    }

    fn control_mut(&mut self, aspect: SimilarityAspect) -> &mut SimilarityAspectControl {
        match aspect {
            SimilarityAspect::Overall => &mut self.overall,
            SimilarityAspect::Spectrum => &mut self.spectrum,
            SimilarityAspect::Timbre => &mut self.timbre,
            SimilarityAspect::Pitch => &mut self.pitch,
            SimilarityAspect::Amplitude => &mut self.amplitude,
        }
    }
}

impl Default for SimilarityAspectSettings {
    fn default() -> Self {
        Self {
            weighting_enabled: false,
            overall: SimilarityAspectControl::default(),
            spectrum: SimilarityAspectControl::default(),
            timbre: SimilarityAspectControl::default(),
            pitch: SimilarityAspectControl::default(),
            amplitude: SimilarityAspectControl::default(),
        }
    }
}

/// Per-aspect enablement and ranking weight.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct SimilarityAspectControl {
    /// Whether this aspect participates in weighted ranking and normal column display.
    #[serde(default = "default_true")]
    pub enabled: bool,
    /// Normalized ranking weight in the `0.0..=1.0` range.
    #[serde(default = "default_similarity_weight")]
    pub weight: f32,
}

impl SimilarityAspectControl {
    /// Return the normalized weight bits used for cache keys.
    pub fn weight_bits(self) -> u32 {
        normalize_similarity_weight(self.weight).to_bits()
    }
}

impl Default for SimilarityAspectControl {
    fn default() -> Self {
        Self {
            enabled: true,
            weight: default_similarity_weight(),
        }
    }
}

fn finite_clamped_score(score: Option<f32>) -> Option<f32> {
    score
        .filter(|score| score.is_finite())
        .map(|score| score.clamp(-1.0, 1.0))
}

fn normalize_similarity_weight(weight: f32) -> f32 {
    if weight.is_finite() {
        weight.clamp(0.0, 1.0)
    } else {
        default_similarity_weight()
    }
}

fn default_true() -> bool {
    true
}

fn default_similarity_weight() -> f32 {
    1.0
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn defaults_preserve_raw_similarity_score() {
        let controls = SimilarityAspectSettings::default();
        let aspect_scores = [Some(-1.0); ASPECT_COUNT];

        assert_eq!(
            controls.effective_score(Some(0.75), &aspect_scores),
            Some(0.75)
        );
    }

    #[test]
    fn enabled_weights_blend_available_aspects() {
        let mut controls = SimilarityAspectSettings::default();
        controls.set_weighting_enabled(true);
        controls.set_aspect_enabled(SimilarityAspect::Overall, false);
        controls.set_aspect_enabled(SimilarityAspect::Timbre, false);
        controls.set_aspect_enabled(SimilarityAspect::Pitch, false);
        controls.set_aspect_enabled(SimilarityAspect::Amplitude, false);
        controls.set_aspect_weight(SimilarityAspect::Spectrum, 0.5);
        let mut aspect_scores = [None; ASPECT_COUNT];
        aspect_scores[SimilarityAspect::Spectrum.index()] = Some(0.25);

        assert_eq!(
            controls.effective_score(Some(0.9), &aspect_scores),
            Some(0.25)
        );
    }

    #[test]
    fn weighted_score_falls_back_to_raw_score_without_available_aspects() {
        let mut controls = SimilarityAspectSettings::default();
        controls.set_weighting_enabled(true);
        controls.set_aspect_enabled(SimilarityAspect::Overall, false);

        assert_eq!(
            controls.effective_score(Some(0.4), &[None; ASPECT_COUNT]),
            Some(0.4)
        );
    }

    #[test]
    fn normalized_controls_clamp_weight_values() {
        let mut controls = SimilarityAspectSettings::default();
        controls.spectrum.weight = 2.0;
        controls.timbre.weight = f32::NAN;

        let controls = controls.normalized();

        assert_eq!(controls.spectrum.weight, 1.0);
        assert_eq!(controls.timbre.weight, 1.0);
    }
}
