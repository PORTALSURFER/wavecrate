use std::collections::HashMap;
use std::hash::{Hash, Hasher};

use radiant::prelude as ui;
use wavecrate_analysis::aspects::SimilarityAspect;

use super::{FolderBrowserState, SimilarityAspectStrengths};

const GROUP_CENTERS: [(f32, f32); wavecrate_analysis::aspects::ASPECT_COUNT] = [
    (0.50, 0.50),
    (0.22, 0.36),
    (0.42, 0.28),
    (0.66, 0.36),
    (0.78, 0.62),
];

#[derive(Clone, Copy)]
pub(in crate::native_app) struct SampleMapProjection<'a> {
    pub(in crate::native_app) tags_by_file: &'a HashMap<String, Vec<String>>,
}

#[derive(Clone, Debug, PartialEq)]
pub(in crate::native_app) struct SampleMapItem {
    pub(in crate::native_app) file_id: String,
    pub(in crate::native_app) label: String,
    pub(in crate::native_app) x: f32,
    pub(in crate::native_app) y: f32,
    pub(in crate::native_app) color: ui::Rgba8,
    pub(in crate::native_app) selected: bool,
    pub(in crate::native_app) similarity_anchor: bool,
    pub(in crate::native_app) missing: bool,
}

impl FolderBrowserState {
    pub(in crate::native_app) fn sample_map_projection(
        &self,
        projection: SampleMapProjection<'_>,
    ) -> Vec<SampleMapItem> {
        let snapshot = self.browser_listing_snapshot(projection.tags_by_file);
        snapshot
            .rows()
            .iter()
            .map(|file| {
                let aspects = self.similarity_aspect_display_strengths_for_file(&file.id);
                let strength = self.similarity_display_strength_for_file(&file.id);
                let group = strongest_enabled_aspect(&aspects, self.similarity_controls());
                let (x, y) = sample_map_position(&file.id, group, strength);
                SampleMapItem {
                    file_id: file.id.clone(),
                    label: file.stem.clone(),
                    x,
                    y,
                    color: sample_map_color(group, strength),
                    selected: self.is_file_selected(&file.id),
                    similarity_anchor: self.file_is_similarity_anchor(&file.id),
                    missing: file.is_missing(),
                }
            })
            .collect()
    }
}

fn strongest_enabled_aspect(
    aspects: &SimilarityAspectStrengths,
    controls: &wavecrate::sample_sources::config::SimilarityAspectSettings,
) -> SimilarityAspect {
    let enabled = controls.aspect_enabled_flags();
    SimilarityAspect::ORDER
        .iter()
        .copied()
        .filter(|aspect| enabled[aspect.index()])
        .filter(|aspect| *aspect != SimilarityAspect::Overall)
        .max_by(|left, right| {
            aspect_strength(aspects, *left)
                .partial_cmp(&aspect_strength(aspects, *right))
                .unwrap_or(std::cmp::Ordering::Equal)
        })
        .unwrap_or(SimilarityAspect::Overall)
}

fn aspect_strength(aspects: &SimilarityAspectStrengths, aspect: SimilarityAspect) -> f32 {
    aspects
        .get(aspect.index())
        .copied()
        .flatten()
        .unwrap_or(0.0)
}

fn sample_map_position(
    file_id: &str,
    group: SimilarityAspect,
    similarity_strength: Option<f32>,
) -> (f32, f32) {
    let (jx, jy) = stable_jitter(file_id);
    let (gx, gy) = GROUP_CENTERS[group.index()];
    let strength = similarity_strength.unwrap_or(0.0);
    let spread = 0.31 - strength.clamp(0.0, 1.0) * 0.13;
    (
        (gx + (jx - 0.5) * spread).clamp(0.04, 0.96),
        (gy + (jy - 0.5) * spread).clamp(0.06, 0.94),
    )
}

fn stable_jitter(file_id: &str) -> (f32, f32) {
    let mut first = std::collections::hash_map::DefaultHasher::new();
    file_id.hash(&mut first);
    let mut second = std::collections::hash_map::DefaultHasher::new();
    "sample-map-y".hash(&mut second);
    file_id.hash(&mut second);
    (
        unit_from_hash(first.finish()),
        unit_from_hash(second.finish()),
    )
}

fn unit_from_hash(hash: u64) -> f32 {
    (hash as f64 / u64::MAX as f64) as f32
}

fn sample_map_color(group: SimilarityAspect, strength: Option<f32>) -> ui::Rgba8 {
    let alpha = (150.0 + strength.unwrap_or(0.35).clamp(0.0, 1.0) * 90.0) as u8;
    match group {
        SimilarityAspect::Overall => ui::Rgba8::new(122, 226, 96, alpha),
        SimilarityAspect::Spectrum => ui::Rgba8::new(239, 216, 66, alpha),
        SimilarityAspect::Timbre => ui::Rgba8::new(255, 142, 56, alpha),
        SimilarityAspect::Pitch => ui::Rgba8::new(255, 55, 96, alpha),
        SimilarityAspect::Amplitude => ui::Rgba8::new(57, 187, 245, alpha),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sample_map_position_is_stable_and_bounded() {
        let first = sample_map_position("kick.wav", SimilarityAspect::Spectrum, Some(0.8));
        let second = sample_map_position("kick.wav", SimilarityAspect::Spectrum, Some(0.8));

        assert_eq!(first, second);
        assert!((0.0..=1.0).contains(&first.0));
        assert!((0.0..=1.0).contains(&first.1));
    }

    #[test]
    fn strongest_enabled_aspect_uses_similarity_strengths() {
        let mut aspects = [None; wavecrate_analysis::aspects::ASPECT_COUNT];
        aspects[SimilarityAspect::Spectrum.index()] = Some(0.2);
        aspects[SimilarityAspect::Timbre.index()] = Some(0.9);

        assert_eq!(
            strongest_enabled_aspect(
                &aspects,
                &wavecrate::sample_sources::config::SimilarityAspectSettings::default(),
            ),
            SimilarityAspect::Timbre
        );
    }
}
