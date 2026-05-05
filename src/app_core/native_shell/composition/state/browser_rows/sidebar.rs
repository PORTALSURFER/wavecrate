//! Sidebar/source-row geometry helpers shared by the native shell.

use super::*;
use crate::compat_app_contract::{FolderPaneIdModel, FolderPaneModel};
use crate::gui::list::{
    VirtualListScrollbarRequest, VirtualListStackMetrics, VirtualListWindowRequest,
    resolve_virtual_list_scrollbar, resolve_virtual_list_window,
    virtual_list_scrollbar_view_start_for_pointer, virtual_list_viewport_len_for_extent,
};

pub(in crate::gui::native_shell::state) fn rendered_source_rows(
    style: &StyleTokens,
    model: &AppModel,
) -> usize {
    model.sources.rows.len().min(style.sizing.source_rows_max)
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub(in crate::gui::native_shell::state) struct BrowserRowsSplitRects {
    pub list: Rect,
    pub sidebar: Option<Rect>,
}

pub(in crate::gui::native_shell::state) fn browser_rows_split_rects(
    rows_rect: Rect,
    sizing: SizingTokens,
    model: &AppModel,
) -> BrowserRowsSplitRects {
    let sidebar = browser_pill_editor_panel_rect(rows_rect, sizing, model);
    let list = if let Some(sidebar_rect) = sidebar {
        Rect::from_min_max(
            rows_rect.min,
            Point::new(sidebar_rect.min.x.max(rows_rect.min.x), rows_rect.max.y),
        )
    } else {
        rows_rect
    };
    BrowserRowsSplitRects { list, sidebar }
}

pub(in crate::gui::native_shell::state) fn browser_rows_list_rect(
    rows_rect: Rect,
    sizing: SizingTokens,
    model: &AppModel,
) -> Rect {
    browser_rows_split_rects(rows_rect, sizing, model).list
}

pub(in crate::gui::native_shell::state) fn browser_pill_editor_panel_rect(
    rows_rect: Rect,
    _sizing: SizingTokens,
    model: &AppModel,
) -> Option<Rect> {
    if !model.browser.pill_editor().open || model.map.active || rows_rect.width() <= 1.0 {
        return None;
    }
    let width = (rows_rect.width() * 0.34).clamp(220.0, 320.0);
    Some(Rect::from_min_max(
        Point::new(
            (rows_rect.max.x - width).max(rows_rect.min.x),
            rows_rect.min.y,
        ),
        rows_rect.max,
    ))
}

pub(in crate::gui::native_shell::state) fn sidebar_rows_cache_key(
    layout: &ShellLayout,
    style: &StyleTokens,
    model: &AppModel,
) -> SidebarRowsCacheKey {
    let sizing = style.sizing;
    SidebarRowsCacheKey {
        root_min_x: f32_to_bits(layout.root.rect.min.x),
        root_min_y: f32_to_bits(layout.root.rect.min.y),
        root_max_x: f32_to_bits(layout.root.rect.max.x),
        root_max_y: f32_to_bits(layout.root.rect.max.y),
        sidebar_rows_min_x: f32_to_bits(layout.sidebar_rows.min.x),
        sidebar_rows_min_y: f32_to_bits(layout.sidebar_rows.min.y),
        sidebar_rows_max_x: f32_to_bits(layout.sidebar_rows.max.x),
        sidebar_rows_max_y: f32_to_bits(layout.sidebar_rows.max.y),
        sidebar_section_gap: f32_to_bits(sizing.sidebar_section_gap),
        panel_section_padding_top: f32_to_bits(sizing.panel_section_padding_top),
        panel_section_padding_bottom: f32_to_bits(sizing.panel_section_padding_bottom),
        source_rows_min_when_split: usize_to_u32(sizing.source_rows_min_when_split),
        tree_rows_min: usize_to_u32(sizing.tree_rows_min),
        active_folder_pane: match model.sources.active_folder_pane {
            FolderPaneIdModel::Upper => 0,
            FolderPaneIdModel::Lower => 1,
        },
        source_rows: rendered_source_rows(style, model) as u32,
        upper_tree_rows: usize_to_u32(model.sources.active_folder_pane_model().tree_rows.len()),
        lower_tree_rows: 0,
        source_row_height: f32_to_bits(sizing.source_row_height),
        source_row_gap: f32_to_bits(sizing.source_row_gap),
        folder_row_height: f32_to_bits(sizing.folder_row_height),
        folder_row_gap: f32_to_bits(sizing.folder_row_gap),
        folder_header_block_height: f32_to_bits(sizing.folder_header_block_height),
        ui_scale: f32_to_bits(layout.ui_scale),
    }
}

pub(in crate::gui::native_shell::state) fn tree_rows_cache_key(
    layout: &ShellLayout,
    style: &StyleTokens,
    model: &AppModel,
    pane: FolderPaneIdModel,
    folder_view_start_row: usize,
    autoscroll: bool,
) -> FolderRowsCacheKey {
    FolderRowsCacheKey {
        sidebar: sidebar_rows_cache_key(layout, style, model),
        pane: match pane {
            FolderPaneIdModel::Upper => 0,
            FolderPaneIdModel::Lower => 1,
        },
        folder_view_start_row: usize_to_u32(folder_view_start_row),
        focused_tree_row: folder_pane_model(model, pane)
            .focused_tree_row
            .map(usize_to_u32)
            .unwrap_or(u32::MAX),
        autoscroll: u32::from(autoscroll),
    }
}

pub(in crate::gui::native_shell::state) fn rendered_source_row_rects(
    layout: &ShellLayout,
    style: &StyleTokens,
    model: &AppModel,
) -> Vec<CachedSourceRow> {
    let sections = sidebar_sections(layout, style, model);
    let row_count = rendered_source_rows(style, model);
    build_stacked_rows(
        sections.source_rows(model.sources.active_folder_pane),
        row_count,
        style.sizing.source_row_gap,
        style.sizing.source_row_height,
    )
    .into_iter()
    .enumerate()
    .map(|(row_index, rect)| CachedSourceRow {
        pane: model.sources.active_folder_pane,
        row_index,
        rect,
    })
    .collect()
}

/// Return the visual folder-row paint bounds while preserving the sidebar seams.
pub(in crate::gui::native_shell::state) fn folder_row_visual_rect(
    row_rect: Rect,
    sizing: SizingTokens,
) -> Rect {
    let seam_stroke = sizing.border_width.max(1.0);
    if row_rect.width() <= seam_stroke * 2.0 {
        return row_rect;
    }
    Rect::from_min_max(
        Point::new(row_rect.min.x + seam_stroke, row_rect.min.y),
        Point::new(row_rect.max.x - seam_stroke, row_rect.max.y),
    )
}

pub(in crate::gui::native_shell::state) fn tree_rows_capacity(
    tree_rows_rect: Rect,
    sizing: SizingTokens,
) -> usize {
    virtual_list_viewport_len_for_extent(
        tree_rows_rect.height(),
        VirtualListStackMetrics::new(sizing.folder_row_height, sizing.folder_row_gap),
    )
}

fn folder_scrollbar_track_metrics(sizing: SizingTokens) -> (f32, f32, f32) {
    let track_inset_x = sizing.text_inset_x.clamp(2.0, 6.0);
    let track_inset_y = 0.0;
    let track_width = (sizing.border_width + 4.0).clamp(4.0, 8.0);
    (track_inset_x, track_inset_y, track_width)
}

pub(in crate::gui::native_shell::state) fn tree_rows_content_rect(
    tree_rows_rect: Rect,
    total_rows: usize,
    sizing: SizingTokens,
) -> Rect {
    let row_capacity = tree_rows_capacity(tree_rows_rect, sizing);
    if total_rows <= row_capacity {
        return tree_rows_rect;
    }
    let (track_inset_x, _, track_width) = folder_scrollbar_track_metrics(sizing);
    let reserved_width = track_inset_x + track_width + super::FOLDER_SCROLLBAR_CONTENT_GAP;
    let content_max_x = (tree_rows_rect.max.x - reserved_width)
        .round()
        .max(tree_rows_rect.min.x + 1.0);
    Rect::from_min_max(
        tree_rows_rect.min,
        Point::new(content_max_x, tree_rows_rect.max.y),
    )
}

fn folder_window_start(
    total_rows: usize,
    window_len: usize,
    focused_row: Option<usize>,
    autoscroll: bool,
    current_view_start: usize,
) -> usize {
    resolve_virtual_list_window(VirtualListWindowRequest {
        total_items: total_rows,
        viewport_len: window_len,
        requested_start: current_view_start,
        previous_start: Some(current_view_start),
        focused_index: focused_row.filter(|_| autoscroll),
        guard_band: super::FOLDER_VIEW_EDGE_MARGIN_ROWS,
        overscan: 0,
    })
    .viewport_start
}

pub(in crate::gui::native_shell::state) fn rendered_tree_rows_with_state(
    layout: &ShellLayout,
    model: &AppModel,
    style: &StyleTokens,
    pane: FolderPaneIdModel,
    current_view_start: usize,
    autoscroll: bool,
) -> (Vec<CachedFolderRow>, usize) {
    let sections = sidebar_sections(layout, style, model);
    let pane_model = folder_pane_model(model, pane);
    let total_rows = pane_model.tree_rows.len();
    if total_rows == 0 {
        return (Vec::new(), 0);
    }
    let rows_rect = sections.tree_rows(pane);
    let row_capacity = tree_rows_capacity(rows_rect, style.sizing);
    let view_start = folder_window_start(
        total_rows,
        row_capacity,
        pane_model.focused_tree_row,
        autoscroll,
        current_view_start,
    );
    let visible_rows = total_rows.saturating_sub(view_start).min(row_capacity);
    let row_rects = build_stacked_rows(
        tree_rows_content_rect(rows_rect, total_rows, style.sizing),
        visible_rows,
        style.sizing.folder_row_gap,
        style.sizing.folder_row_height,
    );
    let rows = row_rects
        .into_iter()
        .enumerate()
        .map(|(offset, rect)| CachedFolderRow {
            pane,
            row_index: view_start + offset,
            rect,
        })
        .collect();
    (rows, view_start)
}

#[cfg(test)]
pub(crate) fn rendered_folder_row_rects(
    layout: &ShellLayout,
    style: &StyleTokens,
    model: &AppModel,
) -> Vec<Rect> {
    rendered_tree_rows_with_state(
        layout,
        model,
        style,
        model.sources.active_folder_pane,
        0,
        true,
    )
    .0
    .into_iter()
    .map(|row| row.rect)
    .collect()
}

pub(in crate::gui::native_shell::state) fn folder_scrollbar_layout(
    tree_rows_rect: Rect,
    rows: &[CachedFolderRow],
    total_rows: usize,
    sizing: SizingTokens,
) -> Option<FolderScrollbarLayout> {
    if rows.is_empty() || total_rows <= rows.len() {
        return None;
    }
    let viewport_start = rows.first()?.row_index.min(total_rows.saturating_sub(1));
    let viewport_len = rows.len().min(total_rows);
    let (track_inset_x, track_inset_y, track_width) = folder_scrollbar_track_metrics(sizing);
    let track_max_x = tree_rows_rect.max.x - track_inset_x;
    let track_min_x = (track_max_x - track_width).max(tree_rows_rect.min.x);
    let track_min_y = (tree_rows_rect.min.y + track_inset_y).min(tree_rows_rect.max.y);
    let track_max_y = (tree_rows_rect.max.y - track_inset_y).max(track_min_y + 1.0);
    let track = Rect::from_min_max(
        Point::new(track_min_x, track_min_y),
        Point::new(track_max_x, track_max_y),
    );
    if track.height() <= 1.0 {
        return None;
    }
    resolve_virtual_list_scrollbar(VirtualListScrollbarRequest {
        track,
        total_items: total_rows,
        viewport_len,
        viewport_start,
        min_thumb_extent: (sizing.folder_row_height * 0.85).round().clamp(18.0, 32.0),
    })
    .map(|scrollbar| FolderScrollbarLayout {
        track: scrollbar.track,
        thumb: scrollbar.thumb,
    })
}

pub(in crate::gui::native_shell::state) fn folder_scrollbar_view_start_for_pointer(
    scrollbar: FolderScrollbarLayout,
    viewport_len: usize,
    total_rows: usize,
    pointer_y: f32,
    thumb_pointer_offset_y: f32,
) -> Option<usize> {
    virtual_list_scrollbar_view_start_for_pointer(
        crate::gui::list::VirtualListScrollbar {
            track: scrollbar.track,
            thumb: scrollbar.thumb,
        },
        viewport_len,
        total_rows,
        pointer_y,
        thumb_pointer_offset_y,
    )
}

pub(in crate::gui::native_shell::state) fn folder_pane_model(
    model: &AppModel,
    pane: FolderPaneIdModel,
) -> &FolderPaneModel {
    model.sources.folder_pane(pane)
}
