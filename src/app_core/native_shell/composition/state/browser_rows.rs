//! Browser-row caches plus focused geometry, truncation, and visual helper modules.

use super::svg_icons::WaveformToolbarIcon;
use super::*;
use crate::app::FolderPaneIdModel;
use crate::gui::native_shell::layout_adapter::BrowserRowTextLayout;

#[path = "browser_rows/sidebar.rs"]
mod sidebar;
#[path = "browser_rows/truncation.rs"]
mod truncation;
#[path = "browser_rows/visuals.rs"]
mod visuals;
#[path = "browser_rows/windowing.rs"]
mod windowing;

pub(in crate::gui::native_shell::state) use self::{
    sidebar::*, truncation::*, visuals::*, windowing::*,
};

#[derive(Clone, Debug, PartialEq)]
pub(super) struct CachedBrowserRow {
    pub(super) visible_row: usize,
    pub(super) visible_row_label: String,
    pub(super) label: String,
    pub(super) bucket_label: String,
    pub(super) inline_tag_labels: Vec<String>,
    pub(super) inline_tag_rects: Vec<Rect>,
    pub(super) text_layout: BrowserRowTextLayout,
    pub(super) label_rendered_width: f32,
    pub(super) column: usize,
    pub(super) rating_level: i8,
    pub(super) playback_age_bucket: crate::app::PlaybackAgeBucket,
    pub(super) similarity_display_strength: Option<u8>,
    pub(super) selected: bool,
    pub(super) focused: bool,
    pub(super) missing: bool,
    pub(super) locked: bool,
    pub(super) marked: bool,
    pub(super) rect: Rect,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub(super) struct CachedFolderRow {
    pub(super) pane: FolderPaneIdModel,
    pub(super) row_index: usize,
    pub(super) rect: Rect,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub(super) struct CachedSourceRow {
    pub(super) pane: FolderPaneIdModel,
    pub(super) row_index: usize,
    pub(super) rect: Rect,
}

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub(super) struct SidebarRowsCacheKey {
    pub(super) root_min_x: u32,
    pub(super) root_min_y: u32,
    pub(super) root_max_x: u32,
    pub(super) root_max_y: u32,
    pub(super) sidebar_rows_min_x: u32,
    pub(super) sidebar_rows_min_y: u32,
    pub(super) sidebar_rows_max_x: u32,
    pub(super) sidebar_rows_max_y: u32,
    pub(super) sidebar_section_gap: u32,
    pub(super) panel_section_padding_top: u32,
    pub(super) panel_section_padding_bottom: u32,
    pub(super) source_rows_min_when_split: u32,
    pub(super) folder_rows_min: u32,
    pub(super) source_rows: u32,
    pub(super) upper_folder_rows: u32,
    pub(super) lower_folder_rows: u32,
    pub(super) source_row_height: u32,
    pub(super) source_row_gap: u32,
    pub(super) folder_row_height: u32,
    pub(super) folder_row_gap: u32,
    pub(super) folder_header_block_height: u32,
    pub(super) ui_scale: u32,
}

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub(super) struct FolderRowsCacheKey {
    pub(super) sidebar: SidebarRowsCacheKey,
    pub(super) pane: u32,
    pub(super) folder_view_start_row: u32,
    pub(super) focused_folder_row: u32,
    pub(super) autoscroll: u32,
}

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub(super) struct BrowserRowsCacheKey {
    pub(super) root_min_x: u32,
    pub(super) root_min_y: u32,
    pub(super) root_max_x: u32,
    pub(super) root_max_y: u32,
    pub(super) browser_rows_min_x: u32,
    pub(super) browser_rows_min_y: u32,
    pub(super) browser_rows_max_x: u32,
    pub(super) browser_rows_max_y: u32,
    pub(super) browser_row_height: u32,
    pub(super) browser_row_gap: u32,
    pub(super) browser_rows_max_per_column: u32,
    pub(super) row_capacity: u32,
    pub(super) browser_row_count: u32,
    pub(super) focused_visible_row: u32,
    pub(super) map_active: u32,
    pub(super) duplicate_cleanup_active: u32,
    pub(super) visible_count: u32,
    pub(super) window_start: u32,
    pub(super) row_text_revision: u64,
    pub(super) ui_scale: u32,
}

#[derive(Clone, Debug, PartialEq)]
pub(super) struct ActionButton {
    pub(super) rect: Rect,
    pub(super) label: &'static str,
    pub(super) icon: Option<WaveformToolbarIcon>,
    pub(super) enabled: bool,
    pub(super) active: bool,
    pub(super) action: UiAction,
    pub(super) text_color: Rgba8,
}

#[derive(Clone, Debug, PartialEq)]
pub(super) struct BrowserColumnChip {
    pub(super) rect: Rect,
    pub(super) column: usize,
    pub(super) label: String,
    pub(super) item_count: usize,
    pub(super) selected: bool,
}

#[derive(Clone, Debug, PartialEq)]
pub(super) struct WaveformToolbarButton {
    pub(super) rect: Rect,
    pub(super) label: &'static str,
    pub(super) icon: Option<WaveformToolbarIcon>,
    pub(super) overlay_icon: Option<WaveformToolbarIcon>,
    pub(super) display_text: Option<String>,
    pub(super) enabled: bool,
    pub(super) active: bool,
    pub(super) action: Option<UiAction>,
    pub(super) text_color: Rgba8,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub(super) struct SidebarPaneSections {
    pub(super) bounds: Rect,
    pub(super) source_rows: Rect,
    pub(super) folder_header: Rect,
    pub(super) folder_rows: Rect,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub(super) struct SidebarSections {
    pub(super) upper: SidebarPaneSections,
    pub(super) lower: SidebarPaneSections,
}

impl SidebarSections {
    pub(super) fn source_rows(self, pane: FolderPaneIdModel) -> Rect {
        match pane {
            FolderPaneIdModel::Upper => self.upper.source_rows,
            FolderPaneIdModel::Lower => self.lower.source_rows,
        }
    }

    pub(super) fn folder_header(self, pane: FolderPaneIdModel) -> Rect {
        match pane {
            FolderPaneIdModel::Upper => self.upper.folder_header,
            FolderPaneIdModel::Lower => self.lower.folder_header,
        }
    }

    pub(super) fn folder_rows(self, pane: FolderPaneIdModel) -> Rect {
        match pane {
            FolderPaneIdModel::Upper => self.upper.folder_rows,
            FolderPaneIdModel::Lower => self.lower.folder_rows,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub(super) struct BrowserToolbarLayout {
    pub(super) rating_filter_chips: [Rect; 8],
    pub(super) playback_age_filter_chips: [Rect; 3],
    pub(super) marked_filter_chip: Rect,
    pub(super) action_slots: [Rect; 3],
    pub(super) search_field: Rect,
    pub(super) activity_chip: Rect,
    pub(super) sort_chip: Rect,
    pub(super) triage_chips: [Rect; 3],
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub(super) struct BrowserScrollbarLayout {
    pub(super) track: Rect,
    pub(super) thumb: Rect,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub(super) struct FolderScrollbarLayout {
    pub(super) track: Rect,
    pub(super) thumb: Rect,
}

/// Number of visible rows kept between focus and the viewport edge before scrolling.
const BROWSER_VIEW_EDGE_MARGIN_ROWS: usize = 3;
/// Horizontal gap left between browser rows and the visual scrollbar lane.
const BROWSER_SCROLLBAR_CONTENT_GAP: f32 = 3.0;
/// Number of folder rows kept between focus and the viewport edge before scrolling.
const FOLDER_VIEW_EDGE_MARGIN_ROWS: usize = 2;
/// Horizontal gap left between folder rows and the visual scrollbar lane.
const FOLDER_SCROLLBAR_CONTENT_GAP: f32 = 3.0;
