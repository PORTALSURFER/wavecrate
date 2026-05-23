//! Mutable interaction state and paint generation for the native shell.
//!
//! This root module is intentionally a façade over focused shell-state helpers.
//! It owns the shared [`NativeShellState`] data model and the top-level entry
//! points that other native-shell code reaches for first, while behavior-heavy
//! rendering, hit-testing, cache, and overlay logic lives in sibling
//! submodules.

use super::{
    browser_chrome_surface::{
        BrowserTabsSurfaceContent, browser_tabs_surface_content, browser_toolbar_surface_content,
        resolve_browser_tabs_surface_layout, resolve_browser_toolbar_surface_layout,
    },
    layout::{ShellLayout, ShellNodeKind},
    layout_adapter::{
        BrowserTabsRects, BrowserTabsTextLayout, BrowserToolbarTextLayout, SidebarFolderRowLayout,
        SidebarRowCounts, SidebarWorkspaceSections, compute_action_button_text_rect,
        compute_browser_footer_text_rect, compute_browser_header_text_layout,
        compute_browser_map_canvas_rect, compute_browser_map_header_text_layout,
        compute_browser_map_point_center, compute_browser_row_text_layout,
        compute_browser_tabs_text_layout, compute_browser_toolbar_text_layout,
        compute_drag_overlay_text_layout, compute_drag_overlay_visual_layout,
        compute_progress_overlay_text_layout, compute_progress_overlay_visual_layout,
        compute_prompt_overlay_text_layout, compute_prompt_overlay_visual_layout,
        compute_sidebar_folder_header_layout, compute_sidebar_folder_row_depth_indent,
        compute_sidebar_folder_row_layout, compute_sidebar_recovery_badge_text_rect,
        compute_sidebar_row_sections, compute_sidebar_source_row_text_rect,
        compute_sidebar_workspace_sections, compute_source_section_divider_rect,
        compute_status_text_line_rect, compute_waveform_annotation_rects_with_nanos,
        compute_waveform_slice_preview_rects, waveform_plot_x_for_absolute_ratio,
        waveform_plot_x_for_micros, waveform_view_window_from_bounds,
    },
    sidebar_surface::{
        SidebarFooterActionSpec, SidebarFooterSurfaceContent, SidebarFooterSurfaceLayout,
        SidebarHeaderSurfaceContent, resolve_sidebar_footer_surface_layout,
        resolve_sidebar_header_surface_layout, sidebar_footer_surface_content,
        sidebar_header_surface_content,
    },
    status_surface::{StatusSurfaceContent, resolve_status_surface_layout},
    style::{SizingTokens, StyleTokens},
    top_bar_surface::{
        TopBarSurfaceLayout, resolve_top_bar_surface_layout, top_bar_options_button_rect,
        top_bar_surface_content,
    },
    waveform_header_surface::{
        resolve_waveform_header_surface_layout, waveform_header_surface_content,
    },
    waveform_toolbar_surface::{
        WaveformToolbarSurfaceContent, WaveformToolbarSurfaceItem, WaveformToolbarSurfaceItemKind,
        resolve_waveform_toolbar_surface_layout,
    },
};
use crate::app_core::native_shell::runtime_contract::{
    AppModel, BrowserRowModel, DirtySegments, NativeMotionModel, UiAction,
};
use crate::gui::paint::{
    DrawImage, FillCircle, FillLinearGradient, FillRect, PaintFrame as NativeViewFrame, Primitive,
    TextAlign, TextRun,
};
use crate::gui::panel::FloatingPanelDrag;
use crate::gui::range::NormalizedPixelSnap;
use crate::gui::{
    input::KeyCode,
    types::{ImageRgba, Point, Rect, Rgba8},
};
use std::{
    collections::HashMap,
    hash::{Hash, Hasher},
    sync::Arc,
};

mod automation;
mod browser_rows;
mod cache;
mod cache_types;
mod frame_build;
mod frame_entrypoints;
mod frame_text_cache;
mod hit_testing;
mod model_sync;
mod motion_overlay;
mod options_panel;
mod overlays;
mod runtime_state;
mod svg_icons;
mod text_fields;
mod toolbar_helpers;
mod waveform_segments;

use self::runtime_state::{FolderPaneRuntimeState, WaveformSelectionFlashTone};
use self::{
    browser_rows::*, cache_types::*, hit_testing::*, options_panel::*, overlays::*, svg_icons::*,
    text_fields::*, toolbar_helpers::*, waveform_segments::*,
};
pub(crate) use self::{
    cache_types::{
        ChromeMotionOverlayFingerprint, CursorMoveEffect, FocusOverlayFingerprint,
        HoverOverlayFingerprint, ModalOverlayFingerprint, StaticFrameSegment, StaticFrameSegments,
        WaveformMotionOverlayFingerprint, WaveformToolbarHoverHint,
    },
    text_fields::TextFieldVisualState,
};

/// Maximum retained entries for browser-row text truncation outputs.
const BROWSER_ROW_TRUNCATION_CACHE_CAPACITY: usize = 1024;
/// Text glyph shown before browser item labels whose backing content is missing.
const BROWSER_MISSING_CONTENT_MARKER: &str = "!";
/// Number of animation ticks used for one waveform-toolbar click flash.
const WAVEFORM_TOOLBAR_FLASH_TICKS: u8 = 6;
/// Number of animation ticks used for one waveform-selection export success flash.
const WAVEFORM_SELECTION_FLASH_TICKS: u8 = 6;
/// Number of animation ticks used for one waveform edit-selection apply flash.
const WAVEFORM_EDIT_SELECTION_FLASH_TICKS: u8 = 6;
/// Number of animation ticks used for the sidebar source-add button click flash.
const SOURCE_ADD_BUTTON_FLASH_TICKS: u8 = 6;
/// Rating-filter chip levels shown left-to-right in the browser toolbar.
const BROWSER_RATING_FILTER_LEVELS: [i8; 8] = [-3, -2, -1, 0, 1, 2, 3, 4];
/// Playback-age filter chips shown left-to-right in the browser toolbar.
const BROWSER_PLAYBACK_AGE_FILTER_CHIPS:
    [crate::app_core::native_shell::runtime_contract::PlaybackAgeFilterChip; 3] = [
    crate::app_core::native_shell::runtime_contract::PlaybackAgeFilterChip::NeverPlayed,
    crate::app_core::native_shell::runtime_contract::PlaybackAgeFilterChip::OlderThanMonth,
    crate::app_core::native_shell::runtime_contract::PlaybackAgeFilterChip::OlderThanWeek,
];
/// Additional hit slop for the narrow content-list scrollbar thumb.
const BROWSER_SCROLLBAR_THUMB_HIT_SLOP: f32 = 3.0;
/// Additional hit slop for the narrow folder scrollbar thumb.
const FOLDER_SCROLLBAR_THUMB_HIT_SLOP: f32 = 3.0;

/// Mutable interaction + animation state for the native shell façade.
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct NativeShellState {
    selected_column: usize,
    hovered: Option<ShellNodeKind>,
    hovered_browser_visible_row: Option<usize>,
    hovered_browser_rating_filter_level: Option<i8>,
    hovered_browser_playback_age_filter_chip:
        Option<crate::app_core::native_shell::runtime_contract::PlaybackAgeFilterChip>,
    hovered_browser_marked_filter: bool,
    hovered_browser_search_field: bool,
    browser_search_editor_visual: Option<TextFieldVisualState>,
    browser_pill_editor_visual: Option<TextFieldVisualState>,
    folder_create_editor_visual: Option<TextFieldVisualState>,
    options_panel_origin: Option<Point>,
    options_panel_drag: Option<FloatingPanelDrag>,
    hovered_folder_pane: Option<crate::app_core::native_shell::runtime_contract::FolderPaneIdModel>,
    hovered_folder_row_index: Option<usize>,
    hovered_source_add_button: bool,
    hovered_status_options_button: bool,
    status_options_button_error: bool,
    hovered_waveform_toolbar_hint: Option<WaveformToolbarHoverHint>,
    waveform_toolbar_flash: Option<WaveformToolbarFlash>,
    waveform_selection_flash_ticks: u8,
    waveform_edit_selection_flash_ticks: u8,
    waveform_selection_flash_tone: WaveformSelectionFlashTone,
    last_waveform_selection_export_flash_nonce: u64,
    last_waveform_selection_export_failure_flash_nonce: u64,
    last_waveform_edit_selection_apply_flash_nonce: u64,
    source_add_button_flash_ticks: u8,
    status_options_button_flash_ticks: u8,
    hovered_waveform_resize_edge: Option<WaveformResizeHoverEdge>,
    waveform_bpm_input_active: bool,
    waveform_bpm_input_display: Option<String>,
    waveform_bpm_editor_visual: Option<TextFieldVisualState>,
    last_waveform_bpm_grid_identity: Option<(Option<String>, Option<u64>)>,
    last_waveform_bpm_grid_origin_micros: Option<u32>,
    waveform_hover_x: Option<f32>,
    last_waveform_playhead_micros: Option<u32>,
    last_waveform_view_window: Option<(u32, u32)>,
    playhead_trail_points: Vec<PlayheadTrailPoint>,
    playhead_trail_elapsed_seconds: f32,
    transport_running: bool,
    has_focus_emphasis: bool,
    startup_frame_ticks: u8,
    pulse_phase: f32,
    source_context_menu: Option<SourceContextMenuState>,
    browser_context_menu: Option<BrowserContextMenuState>,
    sidebar_filter_dropdown: Option<SidebarFilterDropdownState>,
    source_row_rects: Vec<CachedSourceRow>,
    source_row_cache_key: Option<SidebarRowsCacheKey>,
    upper_folder_pane: FolderPaneRuntimeState,
    lower_folder_pane: FolderPaneRuntimeState,
    browser_rows: Vec<CachedBrowserRow>,
    browser_rows_window_start: usize,
    browser_rows_cache_key: Option<BrowserRowsCacheKey>,
    browser_scrollbar: Option<BrowserScrollbarLayout>,
    browser_scrollbar_viewport_len: usize,
    browser_scrollbar_cache_key: Option<BrowserScrollbarCacheKey>,
    browser_action_buttons: Vec<ActionButton>,
    browser_column_chips: Vec<BrowserColumnChip>,
    browser_toolbar_layout: Option<BrowserToolbarLayout>,
    browser_action_hit_test_cache_key: Option<BrowserActionHitTestCacheKey>,
    waveform_toolbar_buttons: Vec<WaveformToolbarButton>,
    waveform_toolbar_hit_test_cache_key: Option<WaveformToolbarHitTestCacheKey>,
    browser_segment_text_cache: Option<Arc<BrowserSegmentTextCacheValue>>,
    browser_segment_text_cache_key: Option<BrowserSegmentTextCacheKey>,
    browser_segment_text_frame_counts: SegmentTextCacheFrameCounts,
    browser_row_truncation_cache: BrowserRowTruncationCache,
    browser_row_truncation_cache_key: Option<BrowserRowTruncationCacheKey>,
    browser_row_truncation_frame_counts: BrowserRowTruncationFrameCounts,
    status_bar_text_cache: Option<Arc<StatusBarTextCacheValue>>,
    status_bar_text_cache_key: Option<StatusBarTextCacheKey>,
    status_bar_text_frame_counts: SegmentTextCacheFrameCounts,
}

impl NativeShellState {
    /// Create a default shell state.
    pub(crate) fn new() -> Self {
        Self {
            selected_column: 1,
            hovered: None,
            hovered_browser_visible_row: None,
            hovered_browser_rating_filter_level: None,
            hovered_browser_playback_age_filter_chip: None,
            hovered_browser_marked_filter: false,
            hovered_browser_search_field: false,
            browser_search_editor_visual: None,
            browser_pill_editor_visual: None,
            folder_create_editor_visual: None,
            options_panel_origin: None,
            options_panel_drag: None,
            hovered_folder_pane: None,
            hovered_folder_row_index: None,
            hovered_source_add_button: false,
            hovered_status_options_button: false,
            status_options_button_error: false,
            hovered_waveform_toolbar_hint: None,
            waveform_toolbar_flash: None,
            waveform_selection_flash_ticks: 0,
            waveform_edit_selection_flash_ticks: 0,
            waveform_selection_flash_tone: WaveformSelectionFlashTone::Optimistic,
            last_waveform_selection_export_flash_nonce: 0,
            last_waveform_selection_export_failure_flash_nonce: 0,
            last_waveform_edit_selection_apply_flash_nonce: 0,
            source_add_button_flash_ticks: 0,
            status_options_button_flash_ticks: 0,
            hovered_waveform_resize_edge: None,
            waveform_bpm_input_active: false,
            waveform_bpm_input_display: None,
            waveform_bpm_editor_visual: None,
            last_waveform_bpm_grid_identity: None,
            last_waveform_bpm_grid_origin_micros: None,
            waveform_hover_x: None,
            last_waveform_playhead_micros: None,
            last_waveform_view_window: None,
            playhead_trail_points: Vec::new(),
            playhead_trail_elapsed_seconds: 0.0,
            transport_running: true,
            has_focus_emphasis: false,
            startup_frame_ticks: 2,
            pulse_phase: 0.0,
            source_context_menu: None,
            browser_context_menu: None,
            sidebar_filter_dropdown: None,
            source_row_rects: Vec::new(),
            source_row_cache_key: None,
            upper_folder_pane: FolderPaneRuntimeState::default(),
            lower_folder_pane: FolderPaneRuntimeState::default(),
            browser_rows: Vec::new(),
            browser_rows_window_start: 0,
            browser_rows_cache_key: None,
            browser_scrollbar: None,
            browser_scrollbar_viewport_len: 0,
            browser_scrollbar_cache_key: None,
            browser_action_buttons: Vec::new(),
            browser_column_chips: Vec::new(),
            browser_toolbar_layout: None,
            browser_action_hit_test_cache_key: None,
            waveform_toolbar_buttons: Vec::new(),
            waveform_toolbar_hit_test_cache_key: None,
            browser_segment_text_cache: None,
            browser_segment_text_cache_key: None,
            browser_segment_text_frame_counts: SegmentTextCacheFrameCounts::default(),
            browser_row_truncation_cache: BrowserRowTruncationCache::default(),
            browser_row_truncation_cache_key: None,
            browser_row_truncation_frame_counts: BrowserRowTruncationFrameCounts::default(),
            status_bar_text_cache: None,
            status_bar_text_cache_key: None,
            status_bar_text_frame_counts: SegmentTextCacheFrameCounts::default(),
        }
    }

    /// Return whether the shell currently needs continuous animation.
    /// Focus emphasis is intentionally not included so selection and focus rendering
    /// remains static without forcing redraws when transport is idle.
    pub(crate) fn needs_animation(&self) -> bool {
        self.animation_reasons().needs_animation()
    }

    /// Return the currently hovered folder-row index, when any.
    pub(crate) fn hovered_folder_row_index(&self) -> Option<usize> {
        self.hovered_folder_row_index
    }

    /// Return the pane currently associated with the hovered folder row, when any.
    pub(crate) fn hovered_folder_pane(
        &self,
    ) -> Option<crate::app_core::native_shell::runtime_contract::FolderPaneIdModel> {
        self.hovered_folder_pane
    }

    /// Override the hovered folder row during focused tests.
    #[cfg(test)]
    pub(crate) fn set_hovered_folder_row_index_for_tests(&mut self, row_index: Option<usize>) {
        self.hovered_folder_row_index = row_index;
    }

    fn animation_reasons(&self) -> NativeAnimationReasons {
        NativeAnimationReasons {
            transport_running: self.transport_running,
            startup_frame_tick: self.startup_frame_ticks > 0,
            playhead_trail_active: !self.playhead_trail_points.is_empty(),
            waveform_toolbar_flash_active: self.waveform_toolbar_flash.is_some(),
            waveform_selection_flash_active: self.waveform_selection_flash_ticks > 0,
            waveform_edit_selection_flash_active: self.waveform_edit_selection_flash_ticks > 0,
            source_add_button_flash_active: self.source_add_button_flash_ticks > 0,
            status_options_button_flash_active: self.status_options_button_flash_ticks > 0,
        }
    }

    /// Return whether playback transport is currently reported as running.
    pub(crate) fn is_transport_running(&self) -> bool {
        self.transport_running
    }

    /// Handle a primary button click at the pointer position.
    pub(crate) fn handle_primary_click(&mut self, layout: &ShellLayout, point: Point) -> bool {
        let Some(column) = layout.column_at_point(point) else {
            return false;
        };
        if self.selected_column == column {
            return false;
        }
        self.selected_column = column;
        true
    }

    /// Handle backend-agnostic key input.
    pub(crate) fn handle_key(&mut self, key: KeyCode) -> bool {
        match key {
            KeyCode::ArrowLeft => {
                self.selected_column = (self.selected_column + 2) % 3;
                true
            }
            KeyCode::ArrowRight => {
                self.selected_column = (self.selected_column + 1) % 3;
                true
            }
            KeyCode::Num1 => {
                if self.selected_column == 0 {
                    false
                } else {
                    self.selected_column = 0;
                    true
                }
            }
            KeyCode::Num2 => {
                if self.selected_column == 1 {
                    false
                } else {
                    self.selected_column = 1;
                    true
                }
            }
            KeyCode::Num3 => {
                if self.selected_column == 2 {
                    false
                } else {
                    self.selected_column = 2;
                    true
                }
            }
            _ => false,
        }
    }

    /// Open the transient source context menu for one source row.
    pub(crate) fn open_source_context_menu_for_row(
        &mut self,
        pane: crate::app_core::native_shell::runtime_contract::FolderPaneIdModel,
        row_index: usize,
        anchor: Point,
    ) {
        self.source_context_menu = Some(SourceContextMenuState {
            pane,
            row_index,
            anchor,
        });
    }

    /// Close the transient source context menu.
    ///
    /// Returns `true` when a visible menu was dismissed.
    pub(crate) fn close_source_context_menu(&mut self) -> bool {
        if self.source_context_menu.is_some() {
            self.source_context_menu = None;
            return true;
        }
        false
    }

    /// Open the transient browser context menu for one browser row.
    pub(crate) fn open_browser_context_menu_for_row(&mut self, visible_row: usize, anchor: Point) {
        self.browser_context_menu = Some(BrowserContextMenuState {
            visible_row,
            anchor,
        });
    }

    /// Close the transient browser context menu.
    pub(crate) fn close_browser_context_menu(&mut self) -> bool {
        if self.browser_context_menu.is_some() {
            self.browser_context_menu = None;
            return true;
        }
        false
    }

    /// Open the transient sidebar filter dropdown for one facet.
    pub(crate) fn open_sidebar_filter_dropdown(&mut self, facet: SidebarFilterDropdownFacet) {
        self.sidebar_filter_dropdown = Some(SidebarFilterDropdownState { facet });
    }

    /// Return whether a sidebar filter dropdown is visible.
    pub(crate) fn sidebar_filter_dropdown_visible(&self) -> bool {
        self.sidebar_filter_dropdown.is_some()
    }

    /// Close the transient sidebar filter dropdown.
    pub(crate) fn close_sidebar_filter_dropdown(&mut self) -> bool {
        if self.sidebar_filter_dropdown.is_some() {
            self.sidebar_filter_dropdown = None;
            return true;
        }
        false
    }
}

#[cfg(test)]
mod opt_272_tests {
    use super::*;
    use crate::app_core::native_shell::runtime_contract::{
        FolderPaneIdModel, SourceRowModel, folder_row_model,
    };
    use crate::gui::types::Vector2;

    fn browser_model_with_rows(total: usize, focused_visible_row: usize) -> AppModel {
        let mut model = AppModel::default();
        for visible_row in 0..total {
            model.browser.rows.push(BrowserRowModel::new(
                visible_row,
                format!("row_{visible_row:04}"),
                1,
                false,
                visible_row == focused_visible_row,
            ));
        }
        model.browser.visible_count = model.browser.rows.len();
        model.browser.selected_visible_row = Some(focused_visible_row);
        model.browser.anchor_visible_row = Some(focused_visible_row.saturating_sub(2));
        model.browser.autoscroll = true;
        model
    }

    fn folder_model_with_rows(total_rows: usize, focused_row: usize) -> AppModel {
        let mut model = AppModel::default();
        model.sources.rows.push(SourceRowModel::new(
            String::from("source"),
            String::from("detail"),
            true,
            false,
        ));
        model.sources.upper_folder_pane.active = true;
        model.sources.upper_folder_pane.has_item = true;
        model.sources.upper_folder_pane.focused_tree_row = Some(focused_row);
        model.sources.active_folder_pane = FolderPaneIdModel::Upper;
        for row_index in 0..total_rows {
            model
                .sources
                .upper_folder_pane
                .tree_rows
                .push(folder_row_model(
                    format!("folder_{row_index:03}"),
                    String::new(),
                    row_index % 3,
                    false,
                    row_index == focused_row,
                    row_index == 0,
                    row_index + 1 < total_rows,
                    true,
                ));
        }
        model
    }

    /// Build a populated single-sidebar fixture for source/folder geometry checks.
    fn populated_single_sidebar_model() -> AppModel {
        let mut model = folder_model_with_rows(48, 4);
        model.sources.rows.clear();
        for index in 0..12 {
            model.sources.rows.push(SourceRowModel::new(
                format!("source_{index:02}"),
                format!("detail_{index:02}"),
                index == 4,
                false,
            ));
        }
        model
    }

    #[test]
    /// The sidebar reserves one source list and one folder browser at all densities.
    fn sidebar_sections_render_one_source_and_folder_browser_across_viewports() {
        let sizes = [
            Vector2::new(820.0, 520.0),
            Vector2::new(1280.0, 720.0),
            Vector2::new(2300.0, 1080.0),
        ];
        let mut state = NativeShellState::new();
        let model = populated_single_sidebar_model();
        for viewport in sizes {
            let layout = ShellLayout::build(viewport);
            let style = style_for_layout(&layout);
            let sections = sidebar_sections(&layout, &style, &model);
            let rendered_sources = state.rendered_source_row_rects(&layout, &model);
            let expected_source_rows = rendered_source_rows(&style, &model);
            assert!(sections.upper.bounds.height() > sections.lower.bounds.height());
            assert!(sections.lower.bounds.height() <= 0.01);
            assert_eq!(rendered_sources.len(), expected_source_rows);
        }
    }

    /// Compact sidebar workspace keeps sources, tags, and filters ordered.
    #[test]
    fn compact_sidebar_workspace_anchors_tags_and_filters_below_sources() {
        let model = populated_single_sidebar_model();
        for viewport in [Vector2::new(820.0, 420.0), Vector2::new(1280.0, 720.0)] {
            let layout = ShellLayout::build(viewport);
            let style = style_for_layout(&layout);
            let workspace = sidebar_workspace_sections(&layout, &style);
            let sections = sidebar_sections(&layout, &style, &model);

            assert!(layout.sidebar_rows.contains(workspace.sources.center()));
            assert!(layout.sidebar_rows.contains(workspace.tags.center()));
            assert!(layout.sidebar_rows.contains(workspace.filters.center()));
            assert!(workspace.sources.max.y <= workspace.tags.min.y);
            assert!(workspace.tags.max.y <= workspace.filters.min.y);
            assert!(workspace.sources.contains(sections.upper.bounds.center()));
        }
    }

    /// Left-sidebar rating chips route through the source hit-test path.
    #[test]
    fn left_sidebar_rating_chip_routes_browser_filter_action() {
        let layout = ShellLayout::build(Vector2::new(1280.0, 720.0));
        let model = populated_single_sidebar_model();
        let mut state = NativeShellState::new();
        let chip = state
            .sidebar_rating_filter_chip_rect(&layout, &model, 3)
            .expect("left-sidebar rating chip should exist");

        assert_eq!(
            state.source_action_at_point(&layout, &model, chip.center()),
            Some(
                crate::app_core::native_shell::runtime_contract::UiAction::ToggleBrowserRatingFilter {
                    level: 3,
                    invert: false,
                }
            )
        );
    }

    /// Left-sidebar metadata filter rows open dropdowns that route sidebar filter actions.
    #[test]
    fn left_sidebar_metadata_filter_dropdowns_route_browser_filter_actions() {
        let layout = ShellLayout::build(Vector2::new(1280.0, 720.0));
        let model = populated_single_sidebar_model();
        let mut state = NativeShellState::new();

        let expected = [
            (
                0,
                0,
                crate::app_core::app_api::state::BrowserSidebarFilterOption::Format(
                    crate::app_core::app_api::state::BrowserFormatFacet::Wav,
                ),
            ),
            (
                1,
                0,
                crate::app_core::app_api::state::BrowserSidebarFilterOption::BitDepth(
                    crate::app_core::app_api::state::BrowserBitDepthFacet::Unavailable,
                ),
            ),
            (
                2,
                3,
                crate::app_core::app_api::state::BrowserSidebarFilterOption::Channels(
                    crate::app_core::app_api::state::BrowserChannelFacet::Unavailable,
                ),
            ),
            (
                3,
                0,
                crate::app_core::app_api::state::BrowserSidebarFilterOption::Bpm(
                    crate::app_core::app_api::state::BrowserBpmFacet::Unknown,
                ),
            ),
            (
                4,
                0,
                crate::app_core::app_api::state::BrowserSidebarFilterOption::Key(
                    crate::app_core::app_api::state::BrowserKeyFacet::Unknown,
                ),
            ),
        ];

        for (row_index, option_index, option) in expected {
            let row = state
                .sidebar_filter_row_rect(&layout, &model, row_index)
                .expect("left-sidebar filter row should exist");
            assert_eq!(
                state.source_action_at_point(&layout, &model, row.center()),
                Some(crate::app_core::native_shell::runtime_contract::UiAction::FocusBrowserPanel)
            );
            assert!(state.sidebar_filter_dropdown_visible());
            let option_rect = state
                .sidebar_filter_dropdown_option_rect(&layout, &model, option_index)
                .expect("left-sidebar filter dropdown option should exist");
            assert_eq!(
                state.sidebar_filter_dropdown_action_at_point(
                    &layout,
                    &model,
                    option_rect.center()
                ),
                Some(
                    crate::app_core::native_shell::runtime_contract::UiAction::ToggleBrowserSidebarFilter {
                        option,
                        additive: true,
                    }
                )
            );
        }
    }

    /// Left-sidebar filter dropdowns expose clear actions for active facets.
    #[test]
    fn left_sidebar_filter_dropdown_clear_routes_browser_filter_action() {
        let layout = ShellLayout::build(Vector2::new(1280.0, 720.0));
        let mut model = populated_single_sidebar_model();
        model.sidebar_filters.toggle(
            crate::app_core::app_api::state::BrowserSidebarFilterOption::Bpm(
                crate::app_core::app_api::state::BrowserBpmFacet::Mid,
            ),
            true,
        );
        let mut state = NativeShellState::new();
        let row = state
            .sidebar_filter_row_rect(&layout, &model, 3)
            .expect("left-sidebar BPM filter row should exist");

        assert_eq!(
            state.source_action_at_point(&layout, &model, row.center()),
            Some(crate::app_core::native_shell::runtime_contract::UiAction::FocusBrowserPanel)
        );
        let clear_rect = state
            .sidebar_filter_dropdown_option_rect(&layout, &model, 4)
            .expect("left-sidebar filter dropdown clear option should exist");

        assert_eq!(
            state.sidebar_filter_dropdown_action_at_point(&layout, &model, clear_rect.center()),
            Some(
                crate::app_core::native_shell::runtime_contract::UiAction::ClearBrowserSidebarFilter {
                    facet: crate::app_core::app_api::state::BrowserSidebarFilterFacet::Bpm,
                }
            )
        );
    }

    #[test]
    /// The single visible folder browser keeps its scrollbar thumb hit target active.
    fn single_folder_browser_scrollbar_thumb_is_hittable() {
        let layout = ShellLayout::build(Vector2::new(1280.0, 720.0));
        let style = style_for_layout(&layout);
        let model = folder_model_with_rows(240, 72);
        let mut state = NativeShellState::new();
        let rows = state
            .cached_tree_rows(&layout, &style, &model, FolderPaneIdModel::Upper)
            .to_vec();
        let sections = sidebar_sections(&layout, &style, &model);
        let scrollbar = folder_scrollbar_layout(
            sections.tree_rows(FolderPaneIdModel::Upper),
            &rows,
            model.sources.upper_folder_pane.tree_rows.len(),
            style.sizing,
        )
        .expect("overflowing single folder browser should render a scrollbar");
        let point = scrollbar.thumb.center();

        let (slot, offset) = state
            .folder_scrollbar_thumb_offset_at_point(&layout, &model, point)
            .expect("single folder scrollbar thumb should be hittable");

        assert_eq!(slot, FolderPaneIdModel::Upper);
        assert!((offset - (scrollbar.thumb.height() * 0.5)).abs() <= 0.001);
    }

    #[test]
    fn browser_rows_use_generic_list_window_hit_testing_and_scrollbar_primitives() {
        let layout = ShellLayout::build(Vector2::new(1280.0, 720.0));
        let style = style_for_layout(&layout);
        let model = browser_model_with_rows(240, 118);
        let mut state = NativeShellState::new();

        let rows = state.cached_browser_rows(&layout, &style, &model).to_vec();
        let list_rect = browser_rows_list_rect(layout.browser_rows, style.sizing, &model);
        let expected_len = browser_rows_capacity(list_rect, style.sizing);

        assert_eq!(rows.len(), expected_len);
        assert!(rows.iter().any(|row| row.visible_row == 118));

        let target = rows[3].rect.center();
        assert_eq!(
            row_index_for_visible_rows(&rows, target, list_rect),
            Some(3)
        );

        let scrollbar =
            browser_scrollbar_layout(list_rect, &rows, model.browser.visible_count, style.sizing)
                .expect("overflowing browser rows should expose a scrollbar");
        assert!(scrollbar.track.contains(scrollbar.thumb.center()));
        assert_eq!(
            browser_scrollbar_view_start_for_pointer(
                scrollbar,
                rows.len(),
                model.browser.visible_count,
                scrollbar.track.max.y,
                scrollbar.thumb.height(),
            ),
            Some(model.browser.visible_count - rows.len())
        );
    }

    #[test]
    fn source_folder_rows_use_generic_list_window_and_scrollbar_primitives() {
        let layout = ShellLayout::build(Vector2::new(1280.0, 720.0));
        let style = style_for_layout(&layout);
        let model = folder_model_with_rows(160, 112);
        let mut state = NativeShellState::new();

        let rows = state
            .cached_tree_rows(&layout, &style, &model, FolderPaneIdModel::Upper)
            .to_vec();
        let sections = sidebar_sections(&layout, &style, &model);
        let tree_rect = sections.tree_rows(FolderPaneIdModel::Upper);
        let expected_len = tree_rows_capacity(tree_rect, style.sizing);

        assert_eq!(rows.len(), expected_len);
        assert!(rows.iter().any(|row| row.row_index == 112));

        let scrollbar = folder_scrollbar_layout(
            tree_rect,
            &rows,
            model.sources.upper_folder_pane.tree_rows.len(),
            style.sizing,
        )
        .expect("overflowing source folders should expose a scrollbar");
        assert!(scrollbar.track.contains(scrollbar.thumb.center()));
        assert_eq!(
            folder_scrollbar_view_start_for_pointer(
                scrollbar,
                rows.len(),
                model.sources.upper_folder_pane.tree_rows.len(),
                scrollbar.track.max.y,
                scrollbar.thumb.height(),
            ),
            Some(model.sources.upper_folder_pane.tree_rows.len() - rows.len())
        );
    }

    #[test]
    fn waveform_scrollbar_thumb_tracks_zoomed_view_position() {
        let layout = ShellLayout::build(Vector2::new(1280.0, 720.0));
        let scrollbar = waveform_scrollbar_layout(layout.waveform_scrollbar_lane, 250_000, 500_000)
            .expect("zoomed waveform view should render a scrollbar");

        assert!(scrollbar.track.min.y >= layout.waveform_scrollbar_lane.min.y);
        assert!(scrollbar.track.max.y <= layout.waveform_scrollbar_lane.max.y);
        assert!(scrollbar.track.min.y >= layout.waveform_plot.max.y);
        assert!(scrollbar.thumb.min.x > scrollbar.track.min.x);
        assert!(scrollbar.thumb.max.x < scrollbar.track.max.x);
        assert!(scrollbar.track.height() <= 3.0);
    }

    #[test]
    fn waveform_scrollbar_hides_when_view_is_fully_zoomed_out() {
        let layout = ShellLayout::build(Vector2::new(1280.0, 720.0));

        assert!(waveform_scrollbar_layout(layout.waveform_scrollbar_lane, 0, 1_000_000).is_none());
    }

    #[test]
    fn waveform_toolbar_renders_svg_backed_icon_images() {
        let layout = ShellLayout::build(Vector2::new(1280.0, 720.0));
        let mut model = AppModel::default();
        model.transport_running = false;
        let mut state = NativeShellState::new();
        let play_rect = state
            .waveform_toolbar_button_rect(&layout, &model, "Play")
            .expect("play waveform toolbar button should be present");
        let channel_rect = state
            .waveform_toolbar_button_rect(&layout, &model, "Channel")
            .expect("channel waveform toolbar button should be present");

        let frame = state.build_frame(&layout, &model);

        for button_rect in [play_rect, channel_rect] {
            let image = frame
                .primitives
                .iter()
                .find_map(|primitive| match primitive {
                    Primitive::Image(image)
                        if image.rect.min.x >= button_rect.min.x
                            && image.rect.min.y >= button_rect.min.y
                            && image.rect.max.x <= button_rect.max.x
                            && image.rect.max.y <= button_rect.max.y =>
                    {
                        Some(image)
                    }
                    _ => None,
                })
                .expect("toolbar button should render an SVG-backed image primitive");
            assert!(
                image.image.pixels.chunks_exact(4).any(|rgba| rgba[3] > 0),
                "toolbar SVG image should contain visible pixels"
            );
        }
    }

    #[test]
    fn options_panel_omits_inner_title_drag_surface() {
        let layout = ShellLayout::build(Vector2::new(1280.0, 720.0));
        let style = style_for_layout(&layout);
        let mut state = NativeShellState::new();
        let model = AppModel {
            options_panel: crate::app_core::native_shell::runtime_contract::OptionsPanelModel {
                visible: true,
                ..crate::app_core::native_shell::runtime_contract::OptionsPanelModel::default()
            },
            ..AppModel::default()
        };
        let initial = options_panel_layout(&layout, &style, &model)
            .expect("visible options panel should resolve layout");
        let grab = initial.title_rect.center();

        assert!(initial.title.is_empty());
        assert_eq!(initial.title_rect.height(), 0.0);
        assert!(!state.begin_options_panel_drag(&layout, &model, grab));
    }

    #[test]
    fn options_panel_drag_clamps_window_inside_shell_bounds() {
        let layout = ShellLayout::build(Vector2::new(900.0, 520.0));
        let style = style_for_layout(&layout);
        let model = AppModel {
            options_panel: crate::app_core::native_shell::runtime_contract::OptionsPanelModel {
                visible: true,
                ..crate::app_core::native_shell::runtime_contract::OptionsPanelModel::default()
            },
            ..AppModel::default()
        };

        let panel = options_panel_layout_for_origin(
            &layout,
            &style,
            &model,
            Some(Point::new(-10_000.0, 10_000.0)),
        )
        .expect("visible options panel should resolve layout");

        assert!(panel.panel_rect.min.x >= layout.root.rect.min.x);
        assert!(panel.panel_rect.min.y >= layout.top_bar.max.y);
        assert!(panel.panel_rect.max.x <= layout.root.rect.max.x);
        assert!(panel.panel_rect.max.y <= layout.status_bar.min.y);
    }

    #[test]
    fn options_panel_picker_mode_expands_inline_dropdown_actions() {
        let layout = ShellLayout::build(Vector2::new(1280.0, 720.0));
        let style = style_for_layout(&layout);
        let state = NativeShellState::new();
        let model = AppModel {
            options_panel: crate::app_core::native_shell::runtime_contract::OptionsPanelModel {
                visible: true,
                ..crate::app_core::native_shell::runtime_contract::OptionsPanelModel::default()
            },
            paired_device: crate::app_core::native_shell::runtime_contract::PairedDevicePanelModel {
                active_picker: Some(
                    crate::app_core::native_shell::runtime_contract::PairedPickerTargetModel::PrimaryNumber,
                ),
                primary_number: crate::app_core::native_shell::runtime_contract::SummaryFieldModel {
                    label: String::from("Sample Rate"),
                    value_label: String::from("48 kHz"),
                },
                primary_number_options: vec![
                    crate::app_core::native_shell::runtime_contract::PairedPickerOptionModel {
                        label: String::from("Device default"),
                        selected: false,
                        value: crate::app_core::native_shell::runtime_contract::PairedPickerValueModel::PrimaryNumber(None),
                    },
                    crate::app_core::native_shell::runtime_contract::PairedPickerOptionModel {
                        label: String::from("48 kHz"),
                        selected: true,
                        value: crate::app_core::native_shell::runtime_contract::PairedPickerValueModel::PrimaryNumber(Some(
                            48_000,
                        )),
                    },
                ],
                ..crate::app_core::native_shell::runtime_contract::PairedDevicePanelModel::default()
            },
            ..AppModel::default()
        };

        let panel = options_panel_layout(&layout, &style, &model)
            .expect("visible picker panel should resolve layout");
        assert!(panel.title.is_empty());
        let dropdown_row = panel
            .buttons
            .iter()
            .position(|button| button.action == UiAction::OpenPrimaryNumberPicker)
            .expect("active picker row should remain in the overview");
        assert!(panel.buttons[dropdown_row].active);
        assert!(panel.buttons[dropdown_row].text.starts_with("Sample Rate"));
        assert_eq!(
            panel.buttons[dropdown_row + 1].action,
            UiAction::SetPrimaryNumber { value: None }
        );
        assert_eq!(
            panel.buttons[dropdown_row + 2].action,
            UiAction::SetPrimaryNumber {
                value: Some(48_000),
            }
        );
        assert!(panel.buttons[dropdown_row + 2].active);

        let dropdown_point = panel.buttons[dropdown_row].rect.center();
        assert_eq!(
            state.options_panel_action_at_point(&layout, &model, dropdown_point),
            Some(UiAction::OpenPrimaryNumberPicker)
        );

        let sample_rate_point = panel.buttons[dropdown_row + 2].rect.center();
        assert_eq!(
            state.options_panel_action_at_point(&layout, &model, sample_rate_point),
            Some(UiAction::SetPrimaryNumber {
                value: Some(48_000),
            })
        );
    }
}
