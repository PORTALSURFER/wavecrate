use crate::native_app::app_chrome::view_models::library_sidebar::{
    CurationFilterToggleViewModel, CurationFilterViewModel, FilterSectionViewModel,
    HarvestFilterToggleViewModel, HarvestFilterViewModel, PlaybackTypeFilterToggleViewModel,
    RatingFilterToggleViewModel,
};
use crate::native_app::sample_library::folder_browser::model::{
    BrowserCurationScope, HarvestFilter, PlaybackTypeFilter,
};

#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) struct FilterRowsProjection {
    pub(super) name_filter: TextFilterRowProjection,
    pub(super) tag_filter: TextFilterRowProjection,
    pub(super) curation: CurationFilterRowProjection,
    pub(super) harvest: HarvestFilterRowProjection,
    pub(super) playback_type: PlaybackTypeFilterRowProjection,
    pub(super) rating: RatingFilterRowProjection,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum TextFilterField {
    Name,
    Tags,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) struct TextFilterRowProjection {
    pub(super) field: TextFilterField,
    pub(super) label: &'static str,
    pub(super) value: String,
    pub(super) placeholder: &'static str,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) struct PlaybackTypeFilterRowProjection {
    pub(super) label: &'static str,
    pub(super) toggles: Vec<PlaybackTypeFilterToggleProjection>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) struct CurationFilterRowProjection {
    pub(super) label: &'static str,
    pub(super) toggles: Vec<CurationFilterToggleProjection>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) struct CurationFilterToggleProjection {
    pub(super) scope: BrowserCurationScope,
    pub(super) label: &'static str,
    pub(super) active: bool,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) struct HarvestFilterRowProjection {
    pub(super) label: &'static str,
    pub(super) toggles: Vec<HarvestFilterToggleProjection>,
    pub(super) family_available: bool,
    pub(super) family_open: bool,
    pub(super) help_tooltips_enabled: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) struct HarvestFilterToggleProjection {
    pub(super) filter: HarvestFilter,
    pub(super) label: &'static str,
    pub(super) tooltip: &'static str,
    pub(super) active: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) struct PlaybackTypeFilterToggleProjection {
    pub(super) filter: PlaybackTypeFilter,
    pub(super) label: &'static str,
    pub(super) active: bool,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) struct RatingFilterRowProjection {
    pub(super) label: &'static str,
    pub(super) toggles: Vec<RatingFilterToggleProjection>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) struct RatingFilterToggleProjection {
    pub(super) level: i8,
    pub(super) label: &'static str,
    pub(super) active: bool,
}

pub(super) fn filter_rows_projection(model: &FilterSectionViewModel) -> FilterRowsProjection {
    FilterRowsProjection {
        name_filter: TextFilterRowProjection {
            field: TextFilterField::Name,
            label: "Name",
            value: model.name_filter.clone(),
            placeholder: "Any",
        },
        tag_filter: TextFilterRowProjection {
            field: TextFilterField::Tags,
            label: "Tags",
            value: model.tag_filter.clone(),
            placeholder: "Any",
        },
        curation: CurationFilterRowProjection::from_view_model(&model.curation),
        harvest: HarvestFilterRowProjection::from_view_model(&model.harvest),
        playback_type: PlaybackTypeFilterRowProjection {
            label: "Type",
            toggles: model
                .playback_type_filters
                .iter()
                .map(PlaybackTypeFilterToggleProjection::from_view_model)
                .collect(),
        },
        rating: RatingFilterRowProjection {
            label: "Rating",
            toggles: model
                .rating_filters
                .iter()
                .map(RatingFilterToggleProjection::from_view_model)
                .collect(),
        },
    }
}

impl CurationFilterRowProjection {
    fn from_view_model(model: &CurationFilterViewModel) -> Self {
        Self {
            label: "Curate",
            toggles: model
                .toggles
                .iter()
                .map(CurationFilterToggleProjection::from_view_model)
                .collect(),
        }
    }
}

impl CurationFilterToggleProjection {
    fn from_view_model(model: &CurationFilterToggleViewModel) -> Self {
        Self {
            scope: model.scope,
            label: model.label,
            active: model.active,
        }
    }
}

impl HarvestFilterRowProjection {
    fn from_view_model(model: &HarvestFilterViewModel) -> Self {
        Self {
            label: "Harvest",
            toggles: model
                .toggles
                .iter()
                .map(HarvestFilterToggleProjection::from_view_model)
                .collect(),
            family_available: model.family_available,
            family_open: model.family_open && model.family_available,
            help_tooltips_enabled: model.help_tooltips_enabled,
        }
    }
}

impl HarvestFilterToggleProjection {
    fn from_view_model(model: &HarvestFilterToggleViewModel) -> Self {
        Self {
            filter: model.filter,
            label: model.label,
            tooltip: harvest_filter_tooltip(model.filter),
            active: model.active,
        }
    }
}

fn harvest_filter_tooltip(filter: HarvestFilter) -> &'static str {
    match filter {
        HarvestFilter::New => "New or seen files that have not been acted on.",
        HarvestFilter::NewAndTouched => "New, seen, and touched files still in the active queue.",
        HarvestFilter::NeedsReview => "Files not done or ignored that do not have derivatives yet.",
        HarvestFilter::Touched => {
            "Files you have rated, tagged, marked, edited, copied, or processed."
        }
        HarvestFilter::HasDerivatives => {
            "Files with one or more derived files recorded in the harvest graph."
        }
        HarvestFilter::NoDerivatives => "Files without any derived files recorded yet.",
        HarvestFilter::Done => "Files you marked done.",
        HarvestFilter::Ignored => "Files you intentionally hid from harvest queues.",
        HarvestFilter::All => "All harvest-tracked files, including done and ignored files.",
    }
}

impl PlaybackTypeFilterToggleProjection {
    fn from_view_model(model: &PlaybackTypeFilterToggleViewModel) -> Self {
        Self {
            filter: model.filter,
            label: model.label,
            active: model.active,
        }
    }
}

impl RatingFilterToggleProjection {
    fn from_view_model(model: &RatingFilterToggleViewModel) -> Self {
        Self {
            level: model.level,
            label: model.label,
            active: model.active,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn filter_rows_projection_carries_text_filter_intent() {
        let projection = filter_rows_projection(&filter_model());

        assert_eq!(
            projection.name_filter,
            TextFilterRowProjection {
                field: TextFilterField::Name,
                label: "Name",
                value: "kick".to_string(),
                placeholder: "Any",
            }
        );
        assert_eq!(
            projection.tag_filter,
            TextFilterRowProjection {
                field: TextFilterField::Tags,
                label: "Tags",
                value: "drum".to_string(),
                placeholder: "Any",
            }
        );
    }

    #[test]
    fn filter_rows_projection_preserves_toggle_order_and_state() {
        let projection = filter_rows_projection(&filter_model());

        assert_eq!(projection.curation.label, "Curate");
        assert_eq!(
            projection
                .curation
                .toggles
                .iter()
                .map(|toggle| (toggle.scope, toggle.label, toggle.active))
                .collect::<Vec<_>>(),
            vec![
                (BrowserCurationScope::All, "All", true),
                (BrowserCurationScope::Ratings, "Rate", false),
                (BrowserCurationScope::Tags, "Tags", false),
            ]
        );
        assert_eq!(projection.harvest.label, "Harvest");
        assert_eq!(
            projection
                .harvest
                .toggles
                .iter()
                .map(|toggle| (toggle.filter, toggle.label, toggle.tooltip, toggle.active))
                .collect::<Vec<_>>(),
            vec![
                (
                    HarvestFilter::New,
                    "New",
                    "New or seen files that have not been acted on.",
                    false
                ),
                (
                    HarvestFilter::NewAndTouched,
                    "N+T",
                    "New, seen, and touched files still in the active queue.",
                    false
                ),
                (
                    HarvestFilter::NeedsReview,
                    "Need",
                    "Files not done or ignored that do not have derivatives yet.",
                    false
                ),
                (
                    HarvestFilter::Touched,
                    "Tch",
                    "Files you have rated, tagged, marked, edited, copied, or processed.",
                    true
                ),
                (
                    HarvestFilter::HasDerivatives,
                    "Der",
                    "Files with one or more derived files recorded in the harvest graph.",
                    false
                ),
                (
                    HarvestFilter::NoDerivatives,
                    "NoD",
                    "Files without any derived files recorded yet.",
                    false
                ),
                (HarvestFilter::Done, "Done", "Files you marked done.", false),
                (
                    HarvestFilter::Ignored,
                    "Ign",
                    "Files you intentionally hid from harvest queues.",
                    false
                ),
                (
                    HarvestFilter::All,
                    "All",
                    "All harvest-tracked files, including done and ignored files.",
                    false
                ),
            ]
        );
        assert_eq!(projection.playback_type.label, "Type");
        assert_eq!(
            projection
                .playback_type
                .toggles
                .iter()
                .map(|toggle| (toggle.filter, toggle.label, toggle.active))
                .collect::<Vec<_>>(),
            vec![
                (PlaybackTypeFilter::OneShot, "1-Shot", false),
                (PlaybackTypeFilter::Loop, "Loop", true),
            ]
        );
        assert_eq!(projection.rating.label, "Rating");
        assert_eq!(
            projection
                .rating
                .toggles
                .iter()
                .map(|toggle| (toggle.level, toggle.label, toggle.active))
                .collect::<Vec<_>>(),
            vec![(-1, "T1", true), (0, "U", false), (4, "K4", true)]
        );
    }

    fn filter_model() -> FilterSectionViewModel {
        FilterSectionViewModel {
            name_filter: "kick".to_string(),
            tag_filter: "drum".to_string(),
            curation: CurationFilterViewModel {
                toggles: vec![
                    CurationFilterToggleViewModel {
                        scope: BrowserCurationScope::All,
                        label: "All",
                        active: true,
                    },
                    CurationFilterToggleViewModel {
                        scope: BrowserCurationScope::Ratings,
                        label: "Rate",
                        active: false,
                    },
                    CurationFilterToggleViewModel {
                        scope: BrowserCurationScope::Tags,
                        label: "Tags",
                        active: false,
                    },
                ],
            },
            harvest: HarvestFilterViewModel {
                toggles: vec![
                    HarvestFilterToggleViewModel {
                        filter: HarvestFilter::New,
                        label: "New",
                        active: false,
                    },
                    HarvestFilterToggleViewModel {
                        filter: HarvestFilter::NewAndTouched,
                        label: "N+T",
                        active: false,
                    },
                    HarvestFilterToggleViewModel {
                        filter: HarvestFilter::NeedsReview,
                        label: "Need",
                        active: false,
                    },
                    HarvestFilterToggleViewModel {
                        filter: HarvestFilter::Touched,
                        label: "Tch",
                        active: true,
                    },
                    HarvestFilterToggleViewModel {
                        filter: HarvestFilter::HasDerivatives,
                        label: "Der",
                        active: false,
                    },
                    HarvestFilterToggleViewModel {
                        filter: HarvestFilter::NoDerivatives,
                        label: "NoD",
                        active: false,
                    },
                    HarvestFilterToggleViewModel {
                        filter: HarvestFilter::Done,
                        label: "Done",
                        active: false,
                    },
                    HarvestFilterToggleViewModel {
                        filter: HarvestFilter::Ignored,
                        label: "Ign",
                        active: false,
                    },
                    HarvestFilterToggleViewModel {
                        filter: HarvestFilter::All,
                        label: "All",
                        active: false,
                    },
                ],
                family_available: true,
                family_open: false,
                help_tooltips_enabled: true,
            },
            playback_type_filters: vec![
                PlaybackTypeFilterToggleViewModel {
                    filter: PlaybackTypeFilter::OneShot,
                    label: "1-Shot",
                    active: false,
                },
                PlaybackTypeFilterToggleViewModel {
                    filter: PlaybackTypeFilter::Loop,
                    label: "Loop",
                    active: true,
                },
            ],
            rating_filters: vec![
                RatingFilterToggleViewModel {
                    level: -1,
                    label: "T1",
                    active: true,
                },
                RatingFilterToggleViewModel {
                    level: 0,
                    label: "U",
                    active: false,
                },
                RatingFilterToggleViewModel {
                    level: 4,
                    label: "K4",
                    active: true,
                },
            ],
            panel_height: 120.0,
        }
    }
}
