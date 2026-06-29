use radiant::{prelude as ui, widgets::TextInputMessageKind};
use std::collections::{BTreeSet, HashMap, HashSet};

use super::{
    DEFAULT_COLLECTIONS_PANEL_HEIGHT, FolderBrowserState,
    curation::{BrowserCurationMode, BrowserCurationScope},
    harvest_filter::{DEFAULT_HARVEST_FILTER, HARVEST_FILTERS, HarvestFilter},
    messages::FilterFamily,
    playback_type_filter::{PLAYBACK_TYPE_FILTERS, PlaybackTypeFilter},
    rating_filter::RATING_FILTER_LEVELS,
};

const FILTER_PANEL_PADDING: f32 = 6.0;
const FILTER_PANEL_HEADER_HEIGHT: f32 = super::SIDEBAR_PANEL_HEADER_HEIGHT;
const FILTER_PANEL_HEADER_CONTENT_SPACING: f32 = super::SIDEBAR_PANEL_HEADER_CONTENT_SPACING;
const MAX_FILTER_PANEL_HEIGHT: f32 = filter_panel_geometry()
    .section_height_for_content_height(super::FILTER_CONTROLS_VIEWPORT_HEIGHT);
pub(in crate::native_app) const COLLAPSED_FILTER_PANEL_HEIGHT: f32 =
    filter_panel_geometry().header_only_height();
const MIN_FILTER_PANEL_HEIGHT: f32 = COLLAPSED_FILTER_PANEL_HEIGHT;
pub(in crate::native_app) const DEFAULT_FILTER_PANEL_HEIGHT: f32 = MAX_FILTER_PANEL_HEIGHT;

const METADATA_PANEL_PADDING: f32 = 6.0;
const METADATA_PANEL_TITLE_HEIGHT: f32 = super::SIDEBAR_PANEL_HEADER_HEIGHT;
const METADATA_PANEL_HEADER_CONTENT_SPACING: f32 = super::SIDEBAR_PANEL_HEADER_CONTENT_SPACING;
const MAX_METADATA_PANEL_HEIGHT: f32 = 240.0;
const DEFAULT_METADATA_PANEL_HEIGHT: f32 = 130.0;
pub(in crate::native_app) const COLLAPSED_METADATA_PANEL_HEIGHT: f32 =
    metadata_panel_geometry().header_only_height();
const MIN_METADATA_PANEL_HEIGHT: f32 = COLLAPSED_METADATA_PANEL_HEIGHT;

const fn filter_panel_geometry() -> ui::PanelSectionGeometry {
    ui::PanelSectionGeometry::new()
        .padding(FILTER_PANEL_PADDING)
        .spacing(FILTER_PANEL_HEADER_CONTENT_SPACING)
        .title_height(FILTER_PANEL_HEADER_HEIGHT)
}

const fn metadata_panel_geometry() -> ui::PanelSectionGeometry {
    ui::PanelSectionGeometry::new()
        .padding(METADATA_PANEL_PADDING)
        .spacing(METADATA_PANEL_HEADER_CONTENT_SPACING)
        .title_height(METADATA_PANEL_TITLE_HEIGHT)
}

#[derive(Clone, Debug, Default)]
pub(super) struct BrowserFilterState {
    pub(super) name_filter: String,
    pub(super) name_enabled: bool,
    pub(super) tag_filter: String,
    pub(super) tags_enabled: bool,
    pub(super) playback_type_filter: BTreeSet<PlaybackTypeFilter>,
    pub(super) playback_type_enabled: bool,
    pub(super) rating_filter: BTreeSet<i8>,
    pub(super) rating_enabled: bool,
    pub(super) curation: BrowserCurationMode,
    pub(super) harvest: Option<HarvestFilter>,
    pub(super) harvest_enabled: bool,
}

#[derive(Clone, Debug)]
pub(super) struct BrowserPanelLayoutState {
    pub(super) collections: ui::PanelResizeState,
    pub(super) filter: ui::PanelResizeState,
    pub(super) metadata: ui::PanelResizeState,
}

impl BrowserPanelLayoutState {
    pub(super) fn new() -> Self {
        Self {
            collections: ui::PanelResizeState::new(DEFAULT_COLLECTIONS_PANEL_HEIGHT),
            filter: ui::PanelResizeState::new(DEFAULT_FILTER_PANEL_HEIGHT),
            metadata: ui::PanelResizeState::new(DEFAULT_METADATA_PANEL_HEIGHT),
        }
    }
}

impl FolderBrowserState {
    pub(in crate::native_app) fn filter_panel_height(&self) -> f32 {
        self.panel_layout
            .filter
            .size()
            .clamp(MIN_FILTER_PANEL_HEIGHT, MAX_FILTER_PANEL_HEIGHT)
    }

    pub(in crate::native_app) fn resize_filter_panel(&mut self, message: ui::DragHandleMessage) {
        self.panel_layout.filter.resize_collapsible(
            message,
            ui::CollapsiblePanelResizeConstraints::top(
                MIN_FILTER_PANEL_HEIGHT,
                MAX_FILTER_PANEL_HEIGHT,
                COLLAPSED_FILTER_PANEL_HEIGHT,
            ),
        );
    }

    pub(in crate::native_app) fn name_filter(&self) -> &str {
        self.filters.name_filter.as_str()
    }

    pub(in crate::native_app) fn name_filter_enabled(&self) -> bool {
        self.filters.name_enabled
    }

    pub(in crate::native_app) fn tag_filter(&self) -> &str {
        self.filters.tag_filter.as_str()
    }

    pub(in crate::native_app) fn tag_filter_enabled(&self) -> bool {
        self.filters.tags_enabled
    }

    pub(in crate::native_app) fn rating_filter(&self) -> &BTreeSet<i8> {
        &self.filters.rating_filter
    }

    pub(in crate::native_app) fn rating_filter_enabled(&self) -> bool {
        self.filters.rating_enabled
    }

    pub(in crate::native_app) fn playback_type_filter(&self) -> &BTreeSet<PlaybackTypeFilter> {
        &self.filters.playback_type_filter
    }

    pub(in crate::native_app) fn playback_type_filter_enabled(&self) -> bool {
        self.filters.playback_type_enabled
    }

    pub(in crate::native_app) fn curation_mode(&self) -> &BrowserCurationMode {
        &self.filters.curation
    }

    pub(in crate::native_app) fn curation_mode_enabled(&self) -> bool {
        self.filters.curation.enabled
    }

    pub(in crate::native_app) fn curation_scope(&self) -> BrowserCurationScope {
        self.filters.curation.scope
    }

    pub(in crate::native_app) fn harvest_filter(&self) -> Option<HarvestFilter> {
        self.filters
            .harvest_enabled
            .then_some(self.selected_harvest_filter())
    }

    pub(in crate::native_app) fn selected_harvest_filter(&self) -> HarvestFilter {
        self.filters.harvest.unwrap_or(DEFAULT_HARVEST_FILTER)
    }

    pub(in crate::native_app) fn harvest_filter_enabled(&self) -> bool {
        self.filters.harvest_enabled
    }

    pub(in crate::native_app) fn apply_name_filter_input(
        &mut self,
        message: radiant::widgets::TextInputMessage,
    ) {
        if message.kind() == TextInputMessageKind::CompletionRequested {
            return;
        }
        let value = message.into_value();
        if self.filters.name_filter == value {
            return;
        }
        self.filters.name_filter = value;
        self.filters.name_enabled = !self.filters.name_filter.trim().is_empty();
        self.clear_listing_reveals();
        self.retain_visible_file_selection_after_filter();
        self.reset_file_view();
    }

    pub(in crate::native_app) fn apply_tag_filter_input(
        &mut self,
        message: radiant::widgets::TextInputMessage,
    ) {
        if message.kind() == TextInputMessageKind::CompletionRequested {
            return;
        }
        let value = message.into_value();
        if self.filters.tag_filter == value {
            return;
        }
        self.filters.tag_filter = value;
        self.filters.tags_enabled = !self.filters.tag_filter.trim().is_empty();
        self.clear_listing_reveals();
        self.reset_file_view();
    }

    pub(in crate::native_app) fn set_filter_family_enabled(
        &mut self,
        family: FilterFamily,
        enabled: bool,
    ) {
        let changed = match family {
            FilterFamily::Name => set_bool(&mut self.filters.name_enabled, enabled),
            FilterFamily::Tags => set_bool(&mut self.filters.tags_enabled, enabled),
            FilterFamily::Curation => set_bool(&mut self.filters.curation.enabled, enabled),
            FilterFamily::Harvest => {
                if enabled && self.filters.harvest.is_none() {
                    self.filters.harvest = Some(DEFAULT_HARVEST_FILTER);
                }
                set_bool(&mut self.filters.harvest_enabled, enabled)
            }
            FilterFamily::PlaybackType => {
                set_bool(&mut self.filters.playback_type_enabled, enabled)
            }
            FilterFamily::Rating => set_bool(&mut self.filters.rating_enabled, enabled),
        };
        if !changed {
            return;
        }
        self.clear_listing_reveals();
        self.retain_visible_file_selection_after_filter();
        self.reset_file_view();
    }

    pub(in crate::native_app) fn set_rating_filter(&mut self, level: i8, enabled: bool) {
        if !RATING_FILTER_LEVELS.contains(&level) {
            return;
        }
        let changed = if enabled {
            self.filters.rating_filter.insert(level)
        } else {
            self.filters.rating_filter.remove(&level)
        };
        if !changed {
            return;
        }
        self.filters.rating_enabled = !self.filters.rating_filter.is_empty();
        self.clear_listing_reveals();
        self.retain_visible_file_selection_after_filter();
        self.reset_file_view();
    }

    pub(in crate::native_app) fn set_playback_type_filter(
        &mut self,
        filter: PlaybackTypeFilter,
        enabled: bool,
    ) {
        if !PLAYBACK_TYPE_FILTERS.contains(&filter) {
            return;
        }
        let changed = if enabled {
            self.filters.playback_type_filter.insert(filter)
        } else {
            self.filters.playback_type_filter.remove(&filter)
        };
        if changed {
            self.filters.playback_type_enabled = !self.filters.playback_type_filter.is_empty();
            self.clear_listing_reveals();
            self.reset_file_view();
        }
    }

    pub(in crate::native_app) fn set_curation_scope(
        &mut self,
        scope: BrowserCurationScope,
        enabled: bool,
    ) {
        let next_enabled = enabled || self.filters.curation.scope != scope;
        if self.filters.curation.enabled == next_enabled && self.filters.curation.scope == scope {
            return;
        }
        self.filters.curation.enabled = next_enabled;
        self.filters.curation.scope = scope;
        self.clear_listing_reveals();
        self.retain_visible_file_selection_after_filter();
        self.reset_file_view();
    }

    pub(in crate::native_app) fn set_harvest_filter(
        &mut self,
        filter: HarvestFilter,
        enabled: bool,
    ) {
        if !HARVEST_FILTERS.contains(&filter) {
            return;
        }
        if self.filters.harvest == Some(filter) && self.filters.harvest_enabled == enabled {
            return;
        }
        self.filters.harvest = Some(filter);
        self.filters.harvest_enabled = enabled;
        self.clear_listing_reveals();
        self.retain_visible_file_selection_after_filter();
        self.reset_file_view();
    }

    pub(in crate::native_app) fn refresh_after_harvest_state_change(&mut self) {
        self.clear_listing_reveals();
        self.retain_visible_file_selection_after_filter();
        self.reset_file_view();
        self.bump_file_content_revision();
    }

    pub(in crate::native_app) fn retain_visible_file_selection_after_tag_filter(
        &mut self,
        tags_by_file: &HashMap<String, Vec<String>>,
    ) {
        let visible_ids = self
            .selected_audio_files_matching_tags(tags_by_file)
            .into_iter()
            .map(|file| file.id.clone())
            .collect::<HashSet<_>>();
        self.selection.retain_visible_files(&visible_ids);
    }

    fn retain_visible_file_selection_after_filter(&mut self) {
        let visible_ids = self
            .selected_audio_files()
            .into_iter()
            .map(|file| file.id.clone())
            .collect::<HashSet<_>>();
        self.selection.retain_visible_files(&visible_ids);
    }

    pub(in crate::native_app) fn metadata_panel_height(&self) -> f32 {
        self.panel_layout.metadata.size()
    }

    pub(in crate::native_app) fn resize_metadata_panel(&mut self, message: ui::DragHandleMessage) {
        self.panel_layout.metadata.resize_collapsible(
            message,
            ui::CollapsiblePanelResizeConstraints::top(
                MIN_METADATA_PANEL_HEIGHT,
                MAX_METADATA_PANEL_HEIGHT,
                COLLAPSED_METADATA_PANEL_HEIGHT,
            ),
        );
    }
}

fn set_bool(target: &mut bool, value: bool) -> bool {
    if *target == value {
        return false;
    }
    *target = value;
    true
}
