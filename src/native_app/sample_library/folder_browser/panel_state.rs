use radiant::{prelude as ui, widgets::TextInputMessageKind};
use std::collections::{HashMap, HashSet};

use super::{DEFAULT_COLLECTIONS_PANEL_HEIGHT, FolderBrowserState};

const FILTER_PANEL_PADDING: f32 = 6.0;
const FILTER_PANEL_HEADER_HEIGHT: f32 = 20.0;
const MAX_FILTER_PANEL_HEIGHT: f32 = 180.0;
pub(in crate::native_app) const COLLAPSED_FILTER_PANEL_HEIGHT: f32 =
    FILTER_PANEL_PADDING * 2.0 + FILTER_PANEL_HEADER_HEIGHT;
const MIN_FILTER_PANEL_HEIGHT: f32 = COLLAPSED_FILTER_PANEL_HEIGHT;
pub(in crate::native_app) const DEFAULT_FILTER_PANEL_HEIGHT: f32 = 76.0;

const METADATA_PANEL_PADDING: f32 = 6.0;
const METADATA_PANEL_TITLE_HEIGHT: f32 = 20.0;
const MAX_METADATA_PANEL_HEIGHT: f32 = 240.0;
const DEFAULT_METADATA_PANEL_HEIGHT: f32 = 148.0;
pub(in crate::native_app) const COLLAPSED_METADATA_PANEL_HEIGHT: f32 =
    METADATA_PANEL_PADDING * 2.0 + METADATA_PANEL_TITLE_HEIGHT;
const MIN_METADATA_PANEL_HEIGHT: f32 = COLLAPSED_METADATA_PANEL_HEIGHT;

#[derive(Clone, Debug, Default)]
pub(super) struct BrowserFilterState {
    pub(super) name_filter: String,
    pub(super) tag_filter: String,
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
        self.panel_layout.filter.size()
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

    pub(in crate::native_app) fn tag_filter(&self) -> &str {
        self.filters.tag_filter.as_str()
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
        self.reset_file_view();
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
