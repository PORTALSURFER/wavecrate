use radiant::prelude as ui;
use wavecrate_analysis::aspects::SimilarityAspect;

use super::super::row_projection::{SampleColumnContent, SampleColumnDisplay};
use super::super::row_widgets::RatingIndicator;
use crate::native_app::sample_library::folder_browser::commands::FileRenameView;
use crate::native_app::sample_library::folder_browser::model::SimilarityAspectStrengths;

#[derive(Clone, Debug, PartialEq)]
pub(super) struct SampleCellProjection {
    pub(super) width: f32,
    pub(super) content: SampleCellContentProjection,
}

#[derive(Clone, Debug, PartialEq)]
pub(super) enum SampleCellContentProjection {
    Name { text: String, badges: Vec<String> },
    Text(String),
    Rename(FileRenameView),
    Rating(RatingCellProjection),
    PlaybackType(PlaybackTypeCellProjection),
    Collection(Vec<ui::Rgba8>),
    Similarity(SimilarityCellProjection),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) enum RatingCellProjection {
    LockedKeepMarker,
    MarkerRun { color: Option<ui::Rgba8>, count: u8 },
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) struct PlaybackTypeCellProjection {
    pub(super) label: String,
    pub(super) available: bool,
}

#[derive(Clone, Debug, PartialEq)]
pub(super) struct SimilarityCellProjection {
    pub(super) overall: Option<f32>,
    pub(super) aspects: Vec<SimilarityAspectProjection>,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub(super) struct SimilarityAspectProjection {
    pub(super) aspect: SimilarityAspect,
    pub(super) strength: Option<f32>,
    pub(super) enabled: bool,
}

/// Convert row projection data into a cell render projection.
pub(super) fn sample_cell_projection(column: SampleColumnDisplay) -> SampleCellProjection {
    SampleCellProjection {
        width: column.width,
        content: match column.content {
            SampleColumnContent::Name { text, badges } => {
                SampleCellContentProjection::Name { text, badges }
            }
            SampleColumnContent::Text(value) => SampleCellContentProjection::Text(value),
            SampleColumnContent::Rename(rename) => SampleCellContentProjection::Rename(rename),
            SampleColumnContent::Rating(indicator) => {
                SampleCellContentProjection::Rating(RatingCellProjection::from_indicator(indicator))
            }
            SampleColumnContent::PlaybackType(label) => {
                SampleCellContentProjection::PlaybackType(PlaybackTypeCellProjection::new(label))
            }
            SampleColumnContent::Collection(colors) => {
                SampleCellContentProjection::Collection(colors)
            }
            SampleColumnContent::Similarity {
                overall,
                aspects,
                aspect_enabled,
            } => SampleCellContentProjection::Similarity(SimilarityCellProjection::new(
                overall,
                aspects,
                aspect_enabled,
            )),
        },
    }
}

impl SampleCellProjection {
    #[cfg(test)]
    /// Build a playback-type projection for focused cell rendering tests.
    pub(super) fn playback_type(width: f32, label: Option<&'static str>) -> Self {
        Self {
            width,
            content: SampleCellContentProjection::PlaybackType(PlaybackTypeCellProjection::new(
                label,
            )),
        }
    }
}

impl RatingCellProjection {
    pub(super) fn from_indicator(indicator: RatingIndicator) -> Self {
        if indicator.shows_locked_keep_marker() {
            Self::LockedKeepMarker
        } else {
            Self::MarkerRun {
                color: indicator.color(),
                count: indicator.count() as u8,
            }
        }
    }

    pub(super) fn marker_color(&self) -> Option<ui::Rgba8> {
        match self {
            Self::LockedKeepMarker => None,
            Self::MarkerRun { color, .. } => *color,
        }
    }

    pub(super) fn marker_count(&self) -> u8 {
        match self {
            Self::LockedKeepMarker => 0,
            Self::MarkerRun { count, .. } => *count,
        }
    }
}

impl PlaybackTypeCellProjection {
    fn new(label: Option<&'static str>) -> Self {
        Self {
            label: label.unwrap_or("-").to_string(),
            available: label.is_some(),
        }
    }
}

impl SimilarityCellProjection {
    pub(super) fn new(
        overall: Option<f32>,
        strengths: SimilarityAspectStrengths,
        enabled: [bool; wavecrate_analysis::aspects::ASPECT_COUNT],
    ) -> Self {
        Self {
            overall,
            aspects: SimilarityAspect::ORDER
                .into_iter()
                .map(|aspect| SimilarityAspectProjection {
                    aspect,
                    strength: strengths[aspect.index()],
                    enabled: enabled[aspect.index()],
                })
                .collect(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::native_app::app_chrome::library_browser::sample_browser_view::row_widgets::RatingIndicator;
    use crate::native_app::sample_library::folder_browser::model::EMPTY_SIMILARITY_ASPECT_STRENGTHS;
    use wavecrate::sample_sources::Rating;

    #[test]
    fn playback_type_projection_marks_missing_label_as_muted_dash_intent() {
        let projection = SampleCellProjection::playback_type(76.0, None);

        assert!(matches!(
            projection.content,
            SampleCellContentProjection::PlaybackType(PlaybackTypeCellProjection {
                label,
                available: false,
            }) if label == "-"
        ));
    }

    #[test]
    fn rating_projection_uses_locked_keep_marker_only_for_locked_keep_three() {
        assert_eq!(
            RatingCellProjection::from_indicator(RatingIndicator::new(Rating::KEEP_3, true)),
            RatingCellProjection::LockedKeepMarker
        );
        assert!(matches!(
            RatingCellProjection::from_indicator(RatingIndicator::new(Rating::KEEP_3, false)),
            RatingCellProjection::MarkerRun {
                count: 3,
                color: Some(_)
            }
        ));
    }

    #[test]
    fn similarity_projection_preserves_aspect_order_strengths_and_enabled_state() {
        let mut strengths = EMPTY_SIMILARITY_ASPECT_STRENGTHS;
        strengths[SimilarityAspect::Spectrum.index()] = Some(0.8);
        strengths[SimilarityAspect::Pitch.index()] = Some(0.2);
        let mut enabled = [true; wavecrate_analysis::aspects::ASPECT_COUNT];
        enabled[SimilarityAspect::Pitch.index()] = false;

        let projection = SimilarityCellProjection::new(Some(0.65), strengths, enabled);

        assert_eq!(projection.overall, Some(0.65));
        assert_eq!(
            projection
                .aspects
                .iter()
                .map(|aspect| aspect.aspect)
                .collect::<Vec<_>>(),
            SimilarityAspect::ORDER.to_vec()
        );
        assert_eq!(
            projection
                .aspects
                .iter()
                .find(|aspect| aspect.aspect == SimilarityAspect::Spectrum)
                .map(|aspect| aspect.strength),
            Some(Some(0.8))
        );
        assert_eq!(
            projection
                .aspects
                .iter()
                .find(|aspect| aspect.aspect == SimilarityAspect::Pitch)
                .map(|aspect| aspect.enabled),
            Some(false)
        );
    }
}
