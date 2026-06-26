use radiant::prelude as ui;

use crate::native_app::app::SampleNameViewMode;
use crate::native_app::sample_library::folder_browser::model::{FileColumn, FileColumnKind};
use wavecrate::sample_sources::config::SimilarityAspectSettings;
use wavecrate_analysis::aspects::SimilarityAspect;

use super::SampleBrowserHeaderBar;

pub(super) const RANDOM_NAVIGATION_TOOLTIP: &str =
    "Random audition within the selected folder or active filter.";
pub(super) const SAMPLE_NAME_VIEW_MODE_TOOLTIP: &str =
    "Switch sample names between disk filenames and metadata labels.";
pub(super) const SAMPLE_MAP_VIEW_TOOLTIP: &str = "Switch between list and sample map views.";

pub(super) struct SampleBrowserHeaderProjection<'a> {
    pub(super) columns: Vec<HeaderColumnProjection<'a>>,
    pub(super) sort: &'a ui::DetailsSort,
    pub(super) drag_marker_x: Option<f32>,
    pub(super) random_navigation: RandomNavigationButtonProjection,
    pub(super) map_view: SampleMapViewButtonProjection,
    pub(super) name_view_mode: SampleNameViewModeButtonProjection,
    pub(super) similarity_header: SampleSimilarityHeaderProjection,
    pub(super) help_tooltips_enabled: bool,
}

impl<'a> SampleBrowserHeaderProjection<'a> {
    pub(super) fn from_model(model: SampleBrowserHeaderBar<'a>) -> Self {
        Self {
            columns: projected_header_columns(model.columns, model.similarity_mode_active),
            sort: model.sort,
            drag_marker_x: model.drag_feedback.map(|feedback| feedback.marker_x),
            random_navigation: RandomNavigationButtonProjection::new(
                model.random_navigation_enabled,
            ),
            map_view: SampleMapViewButtonProjection::new(model.map_view_active),
            name_view_mode: SampleNameViewModeButtonProjection::from_mode(model.mode),
            similarity_header: SampleSimilarityHeaderProjection::from_settings(
                model.similarity_controls,
            ),
            help_tooltips_enabled: model.help_tooltips_enabled,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) struct RandomNavigationButtonProjection {
    pub(super) active: bool,
    pub(super) tooltip: &'static str,
}

impl RandomNavigationButtonProjection {
    pub(super) fn new(active: bool) -> Self {
        Self {
            active,
            tooltip: RANDOM_NAVIGATION_TOOLTIP,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) struct SampleMapViewButtonProjection {
    pub(super) active: bool,
    pub(super) tooltip: &'static str,
}

impl SampleMapViewButtonProjection {
    pub(super) fn new(active: bool) -> Self {
        Self {
            active,
            tooltip: SAMPLE_MAP_VIEW_TOOLTIP,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) struct SampleNameViewModeButtonProjection {
    pub(super) label: &'static str,
    pub(super) tooltip: &'static str,
}

impl SampleNameViewModeButtonProjection {
    pub(super) fn from_mode(mode: SampleNameViewMode) -> Self {
        Self {
            label: match mode {
                SampleNameViewMode::DiskFilename => "Disk",
                SampleNameViewMode::MetadataLabel => "Label",
            },
            tooltip: SAMPLE_NAME_VIEW_MODE_TOOLTIP,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub(super) struct HeaderColumnProjection<'a> {
    pub(super) column: &'a FileColumn,
    pub(super) show_similarity_after: bool,
}

pub(super) fn projected_header_columns<'a>(
    columns: &'a [&'a FileColumn],
    similarity_mode_active: bool,
) -> Vec<HeaderColumnProjection<'a>> {
    columns
        .iter()
        .map(|column| HeaderColumnProjection {
            column,
            show_similarity_after: similarity_mode_active && column.kind() == FileColumnKind::Name,
        })
        .collect()
}

#[derive(Clone, Debug, PartialEq)]
pub(super) struct SampleSimilarityControlsProjection {
    pub(super) weighting_label: &'static str,
    pub(super) weighting_enabled: bool,
    pub(super) aspects: Vec<SampleSimilarityAspectControlProjection>,
}

impl SampleSimilarityControlsProjection {
    pub(super) fn from_settings(settings: &SimilarityAspectSettings) -> Self {
        Self {
            weighting_label: "Weight",
            weighting_enabled: settings.weighting_enabled,
            aspects: SimilarityAspect::ORDER
                .into_iter()
                .map(|aspect| {
                    SampleSimilarityAspectControlProjection::from_settings(aspect, settings)
                })
                .collect(),
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub(super) struct SampleSimilarityAspectControlProjection {
    pub(super) aspect: SimilarityAspect,
    pub(super) label: &'static str,
    pub(super) enabled: bool,
    pub(super) weight: f32,
}

impl SampleSimilarityAspectControlProjection {
    fn from_settings(aspect: SimilarityAspect, settings: &SimilarityAspectSettings) -> Self {
        let control = settings.control(aspect);
        Self {
            aspect,
            label: similarity_aspect_short_label(aspect),
            enabled: control.enabled,
            weight: control.weight,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) struct SampleSimilarityHeaderProjection {
    pub(super) aspects: Vec<SampleSimilarityHeaderAspectProjection>,
    pub(super) score_label: &'static str,
}

impl SampleSimilarityHeaderProjection {
    pub(super) fn from_settings(settings: &SimilarityAspectSettings) -> Self {
        Self {
            aspects: SimilarityAspect::ORDER
                .into_iter()
                .map(|aspect| SampleSimilarityHeaderAspectProjection {
                    label: similarity_aspect_short_label(aspect),
                    enabled: settings.aspect_enabled(aspect),
                })
                .collect(),
            score_label: "Sim",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) struct SampleSimilarityHeaderAspectProjection {
    pub(super) label: &'static str,
    pub(super) enabled: bool,
}

fn similarity_aspect_short_label(aspect: SimilarityAspect) -> &'static str {
    match aspect {
        SimilarityAspect::Overall => "O",
        SimilarityAspect::Spectrum => "S",
        SimilarityAspect::Timbre => "T",
        SimilarityAspect::Pitch => "P",
        SimilarityAspect::Amplitude => "A",
    }
}
