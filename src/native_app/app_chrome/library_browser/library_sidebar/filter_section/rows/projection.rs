use crate::native_app::app_chrome::view_models::library_sidebar::{
    CurationFilterOptionViewModel, CurationFilterViewModel, FilterSectionViewModel,
    HarvestFilterOptionViewModel, HarvestFilterViewModel, PlaybackTypeFilterToggleViewModel,
    RatingFilterToggleViewModel,
};
use crate::native_app::sample_library::folder_browser::commands::FilterFamily;
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
    pub(super) family: FilterFamily,
    pub(super) label: &'static str,
    pub(super) value: String,
    pub(super) enabled: bool,
    pub(super) placeholder: &'static str,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) struct PlaybackTypeFilterRowProjection {
    pub(super) family: FilterFamily,
    pub(super) label: &'static str,
    pub(super) enabled: bool,
    pub(super) toggles: Vec<PlaybackTypeFilterToggleProjection>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) struct CurationFilterRowProjection {
    pub(super) family: FilterFamily,
    pub(super) label: &'static str,
    pub(super) enabled: bool,
    pub(super) dropdown_open: bool,
    pub(super) menu_width: u16,
    pub(super) selected_label: &'static str,
    pub(super) options: Vec<CurationFilterOptionProjection>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) struct CurationFilterOptionProjection {
    pub(super) scope: BrowserCurationScope,
    pub(super) label: &'static str,
    pub(super) selected: bool,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) struct HarvestFilterRowProjection {
    pub(super) family: FilterFamily,
    pub(super) label: &'static str,
    pub(super) enabled: bool,
    pub(super) dropdown_open: bool,
    pub(super) menu_width: u16,
    pub(super) selected_label: &'static str,
    pub(super) options: Vec<HarvestFilterOptionProjection>,
    pub(super) family_available: bool,
    pub(super) family_open: bool,
    pub(super) help_tooltips_enabled: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) struct HarvestFilterOptionProjection {
    pub(super) filter: HarvestFilter,
    pub(super) label: &'static str,
    pub(super) tooltip: &'static str,
    pub(super) selected: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) struct PlaybackTypeFilterToggleProjection {
    pub(super) filter: PlaybackTypeFilter,
    pub(super) label: &'static str,
    pub(super) active: bool,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) struct RatingFilterRowProjection {
    pub(super) family: FilterFamily,
    pub(super) label: &'static str,
    pub(super) enabled: bool,
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
            family: FilterFamily::Name,
            label: "Name",
            value: model.name_filter.clone(),
            enabled: model.name_filter_enabled,
            placeholder: "Any",
        },
        tag_filter: TextFilterRowProjection {
            field: TextFilterField::Tags,
            family: FilterFamily::Tags,
            label: "Tags",
            value: model.tag_filter.clone(),
            enabled: model.tag_filter_enabled,
            placeholder: "Any",
        },
        curation: CurationFilterRowProjection::from_view_model(
            &model.curation,
            model.sidebar_width,
        ),
        harvest: HarvestFilterRowProjection::from_view_model(&model.harvest, model.sidebar_width),
        playback_type: PlaybackTypeFilterRowProjection {
            family: FilterFamily::PlaybackType,
            label: "Type",
            enabled: model.playback_type_enabled,
            toggles: model
                .playback_type_filters
                .iter()
                .map(PlaybackTypeFilterToggleProjection::from_view_model)
                .collect(),
        },
        rating: RatingFilterRowProjection {
            family: FilterFamily::Rating,
            label: "Ratin",
            enabled: model.rating_enabled,
            toggles: model
                .rating_filters
                .iter()
                .map(RatingFilterToggleProjection::from_view_model)
                .collect(),
        },
    }
}

impl CurationFilterRowProjection {
    fn from_view_model(model: &CurationFilterViewModel, sidebar_width: f32) -> Self {
        Self {
            family: FilterFamily::Curation,
            label: "Curat",
            enabled: model.enabled,
            dropdown_open: model.dropdown_open,
            menu_width: curation_dropdown_menu_width(sidebar_width),
            selected_label: model
                .options
                .iter()
                .find(|option| option.scope == model.selected_scope)
                .map(|option| option.label)
                .unwrap_or_else(|| model.selected_scope.label()),
            options: model
                .options
                .iter()
                .map(|option| {
                    CurationFilterOptionProjection::from_view_model(option, model.selected_scope)
                })
                .collect(),
        }
    }
}

fn curation_dropdown_menu_width(sidebar_width: f32) -> u16 {
    let section_padding = super::super::FILTER_PANEL_PADDING * 2.0;
    let content_width = sidebar_width - section_padding;
    (content_width - super::FILTER_LABEL_WIDTH - super::FILTER_LABEL_CONTROL_SPACING)
        .max(1.0)
        .round() as u16
}

impl CurationFilterOptionProjection {
    fn from_view_model(
        model: &CurationFilterOptionViewModel,
        selected_scope: BrowserCurationScope,
    ) -> Self {
        Self {
            scope: model.scope,
            label: model.label,
            selected: model.scope == selected_scope,
        }
    }
}

impl HarvestFilterRowProjection {
    fn from_view_model(model: &HarvestFilterViewModel, sidebar_width: f32) -> Self {
        Self {
            family: FilterFamily::Harvest,
            label: "Harvest",
            enabled: model.enabled,
            dropdown_open: model.dropdown_open,
            menu_width: curation_dropdown_menu_width(sidebar_width),
            selected_label: model
                .selected_filter
                .and_then(|selected_filter| {
                    model
                        .options
                        .iter()
                        .find(|option| option.filter == selected_filter)
                        .map(|option| option.label)
                })
                .unwrap_or("Any"),
            options: model
                .options
                .iter()
                .map(|option| {
                    HarvestFilterOptionProjection::from_view_model(option, model.selected_filter)
                })
                .collect(),
            family_available: model.family_available,
            family_open: model.family_open && model.family_available,
            help_tooltips_enabled: model.help_tooltips_enabled,
        }
    }
}

impl HarvestFilterOptionProjection {
    fn from_view_model(
        model: &HarvestFilterOptionViewModel,
        selected_filter: Option<HarvestFilter>,
    ) -> Self {
        Self {
            filter: model.filter,
            label: model.label,
            tooltip: harvest_filter_tooltip(model.filter),
            selected: selected_filter == Some(model.filter),
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
                family: FilterFamily::Name,
                label: "Name",
                value: "kick".to_string(),
                enabled: true,
                placeholder: "Any",
            }
        );
        assert_eq!(
            projection.tag_filter,
            TextFilterRowProjection {
                field: TextFilterField::Tags,
                family: FilterFamily::Tags,
                label: "Tags",
                value: "drum".to_string(),
                enabled: true,
                placeholder: "Any",
            }
        );
    }

    #[test]
    fn filter_rows_projection_preserves_dropdown_options_and_filter_state() {
        let projection = filter_rows_projection(&filter_model());

        assert_eq!(projection.curation.label, "Curat");
        assert!(projection.curation.enabled);
        assert!(projection.curation.dropdown_open);
        assert_eq!(projection.curation.selected_label, "All");
        assert_eq!(
            projection
                .curation
                .options
                .iter()
                .map(|option| (option.scope, option.label, option.selected))
                .collect::<Vec<_>>(),
            vec![
                (BrowserCurationScope::All, "All", true),
                (BrowserCurationScope::Ratings, "Rate", false),
                (BrowserCurationScope::Tags, "Tags", false),
            ]
        );
        assert_eq!(projection.harvest.label, "Harvest");
        assert!(projection.harvest.enabled);
        assert!(projection.harvest.dropdown_open);
        assert_eq!(projection.harvest.selected_label, "Touched");
        assert_eq!(
            projection
                .harvest
                .options
                .iter()
                .map(|option| (option.filter, option.label, option.tooltip, option.selected))
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
                    "New + Touched",
                    "New, seen, and touched files still in the active queue.",
                    false
                ),
                (
                    HarvestFilter::NeedsReview,
                    "Needs Review",
                    "Files not done or ignored that do not have derivatives yet.",
                    false
                ),
                (
                    HarvestFilter::Touched,
                    "Touched",
                    "Files you have rated, tagged, marked, edited, copied, or processed.",
                    true
                ),
                (
                    HarvestFilter::HasDerivatives,
                    "Has Derivatives",
                    "Files with one or more derived files recorded in the harvest graph.",
                    false
                ),
                (
                    HarvestFilter::NoDerivatives,
                    "No Derivatives",
                    "Files without any derived files recorded yet.",
                    false
                ),
                (HarvestFilter::Done, "Done", "Files you marked done.", false),
                (
                    HarvestFilter::Ignored,
                    "Ignored",
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
        assert!(projection.playback_type.enabled);
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
        assert_eq!(projection.rating.label, "Ratin");
        assert!(projection.rating.enabled);
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
            sidebar_width: 240.0,
            name_filter: "kick".to_string(),
            name_filter_enabled: true,
            tag_filter: "drum".to_string(),
            tag_filter_enabled: true,
            curation: CurationFilterViewModel {
                enabled: true,
                dropdown_open: true,
                selected_scope: BrowserCurationScope::All,
                options: vec![
                    CurationFilterOptionViewModel {
                        scope: BrowserCurationScope::All,
                        label: "All",
                    },
                    CurationFilterOptionViewModel {
                        scope: BrowserCurationScope::Ratings,
                        label: "Rate",
                    },
                    CurationFilterOptionViewModel {
                        scope: BrowserCurationScope::Tags,
                        label: "Tags",
                    },
                ],
            },
            harvest: HarvestFilterViewModel {
                enabled: true,
                dropdown_open: true,
                selected_filter: Some(HarvestFilter::Touched),
                options: vec![
                    HarvestFilterOptionViewModel {
                        filter: HarvestFilter::New,
                        label: "New",
                    },
                    HarvestFilterOptionViewModel {
                        filter: HarvestFilter::NewAndTouched,
                        label: "New + Touched",
                    },
                    HarvestFilterOptionViewModel {
                        filter: HarvestFilter::NeedsReview,
                        label: "Needs Review",
                    },
                    HarvestFilterOptionViewModel {
                        filter: HarvestFilter::Touched,
                        label: "Touched",
                    },
                    HarvestFilterOptionViewModel {
                        filter: HarvestFilter::HasDerivatives,
                        label: "Has Derivatives",
                    },
                    HarvestFilterOptionViewModel {
                        filter: HarvestFilter::NoDerivatives,
                        label: "No Derivatives",
                    },
                    HarvestFilterOptionViewModel {
                        filter: HarvestFilter::Done,
                        label: "Done",
                    },
                    HarvestFilterOptionViewModel {
                        filter: HarvestFilter::Ignored,
                        label: "Ignored",
                    },
                    HarvestFilterOptionViewModel {
                        filter: HarvestFilter::All,
                        label: "All",
                    },
                ],
                family_available: true,
                family_open: false,
                help_tooltips_enabled: true,
            },
            playback_type_enabled: true,
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
            rating_enabled: true,
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
