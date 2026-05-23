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
mod browser_pill_editor;
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
#[path = "state/tests/opt_272.rs"]
mod opt_272_tests;
