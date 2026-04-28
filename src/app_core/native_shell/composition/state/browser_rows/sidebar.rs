//! Sidebar/source-row geometry helpers shared by the native shell.

use super::*;
use crate::app::{FolderPaneIdModel, FolderPaneModel};

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
    let sidebar = browser_tag_sidebar_panel_rect(rows_rect, sizing, model);
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

pub(in crate::gui::native_shell::state) fn browser_tag_sidebar_panel_rect(
    rows_rect: Rect,
    _sizing: SizingTokens,
    model: &AppModel,
) -> Option<Rect> {
    if !model.browser.tag_sidebar.open || model.map.active || rows_rect.width() <= 1.0 {
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
        folder_rows_min: usize_to_u32(sizing.folder_rows_min),
        source_rows: rendered_source_rows(style, model) as u32,
        upper_folder_rows: usize_to_u32(model.sources.upper_folder_pane.folder_rows.len()),
        lower_folder_rows: usize_to_u32(model.sources.lower_folder_pane.folder_rows.len()),
        source_row_height: f32_to_bits(sizing.source_row_height),
        source_row_gap: f32_to_bits(sizing.source_row_gap),
        folder_row_height: f32_to_bits(sizing.folder_row_height),
        folder_row_gap: f32_to_bits(sizing.folder_row_gap),
        folder_header_block_height: f32_to_bits(sizing.folder_header_block_height),
        ui_scale: f32_to_bits(layout.ui_scale),
    }
}

pub(in crate::gui::native_shell::state) fn folder_rows_cache_key(
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
        focused_folder_row: folder_pane_model(model, pane)
            .focused_folder_row
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
    let mut rows = Vec::with_capacity(row_count.saturating_mul(2));
    for pane in [FolderPaneIdModel::Upper, FolderPaneIdModel::Lower] {
        rows.extend(
            build_stacked_rows(
                sections.source_rows(pane),
                row_count,
                style.sizing.source_row_gap,
                style.sizing.source_row_height,
            )
            .into_iter()
            .enumerate()
            .map(|(row_index, rect)| CachedSourceRow {
                pane,
                row_index,
                rect,
            }),
        );
    }
    rows
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

pub(in crate::gui::native_shell::state) fn folder_rows_capacity(
    folder_rows_rect: Rect,
    sizing: SizingTokens,
) -> usize {
    let row_height = sizing.folder_row_height.max(1.0);
    let row_gap = sizing.folder_row_gap.max(0.0);
    ((folder_rows_rect.height() + row_gap) / (row_height + row_gap))
        .floor()
        .max(1.0) as usize
}

fn folder_scrollbar_track_metrics(sizing: SizingTokens) -> (f32, f32, f32) {
    let track_inset_x = sizing.text_inset_x.clamp(2.0, 6.0);
    let track_inset_y = 0.0;
    let track_width = (sizing.border_width + 4.0).clamp(4.0, 8.0);
    (track_inset_x, track_inset_y, track_width)
}

pub(in crate::gui::native_shell::state) fn folder_rows_content_rect(
    folder_rows_rect: Rect,
    total_rows: usize,
    sizing: SizingTokens,
) -> Rect {
    let row_capacity = folder_rows_capacity(folder_rows_rect, sizing);
    if total_rows <= row_capacity {
        return folder_rows_rect;
    }
    let (track_inset_x, _, track_width) = folder_scrollbar_track_metrics(sizing);
    let reserved_width = track_inset_x + track_width + super::FOLDER_SCROLLBAR_CONTENT_GAP;
    let content_max_x = (folder_rows_rect.max.x - reserved_width)
        .round()
        .max(folder_rows_rect.min.x + 1.0);
    Rect::from_min_max(
        folder_rows_rect.min,
        Point::new(content_max_x, folder_rows_rect.max.y),
    )
}

fn folder_window_start(
    total_rows: usize,
    window_len: usize,
    focused_row: Option<usize>,
    autoscroll: bool,
    current_view_start: usize,
) -> usize {
    if total_rows <= window_len {
        return 0;
    }
    let max_start = total_rows - window_len;
    let mut view_start = current_view_start.min(max_start);
    if !autoscroll {
        return view_start;
    }
    let Some(focused_row) = focused_row else {
        return view_start;
    };
    let edge_margin = super::FOLDER_VIEW_EDGE_MARGIN_ROWS.min(window_len.saturating_sub(1) / 2);
    let focused_row = focused_row.min(total_rows.saturating_sub(1));
    let view_end = view_start + window_len;
    let top_guard = view_start + edge_margin;
    let bottom_guard = view_end.saturating_sub(edge_margin);
    if focused_row < top_guard {
        view_start = focused_row.saturating_sub(edge_margin);
    } else if focused_row >= bottom_guard {
        view_start = focused_row
            .saturating_add(edge_margin + 1)
            .saturating_sub(window_len);
    }
    view_start.min(max_start)
}

pub(in crate::gui::native_shell::state) fn rendered_folder_rows_with_state(
    layout: &ShellLayout,
    model: &AppModel,
    style: &StyleTokens,
    pane: FolderPaneIdModel,
    current_view_start: usize,
    autoscroll: bool,
) -> (Vec<CachedFolderRow>, usize) {
    let sections = sidebar_sections(layout, style, model);
    let pane_model = folder_pane_model(model, pane);
    let total_rows = pane_model.folder_rows.len();
    if total_rows == 0 {
        return (Vec::new(), 0);
    }
    let rows_rect = sections.folder_rows(pane);
    let row_capacity = folder_rows_capacity(rows_rect, style.sizing);
    let view_start = folder_window_start(
        total_rows,
        row_capacity,
        pane_model.focused_folder_row,
        autoscroll,
        current_view_start,
    );
    let visible_rows = total_rows.saturating_sub(view_start).min(row_capacity);
    let row_rects = build_stacked_rows(
        folder_rows_content_rect(rows_rect, total_rows, style.sizing),
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
    rendered_folder_rows_with_state(
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
    folder_rows_rect: Rect,
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
    let track_max_x = folder_rows_rect.max.x - track_inset_x;
    let track_min_x = (track_max_x - track_width).max(folder_rows_rect.min.x);
    let track_min_y = (folder_rows_rect.min.y + track_inset_y).min(folder_rows_rect.max.y);
    let track_max_y = (folder_rows_rect.max.y - track_inset_y).max(track_min_y + 1.0);
    let track = Rect::from_min_max(
        Point::new(track_min_x, track_min_y),
        Point::new(track_max_x, track_max_y),
    );
    if track.height() <= 1.0 {
        return None;
    }
    let min_thumb_height = (sizing.folder_row_height * 0.85).round().clamp(18.0, 32.0);
    let thumb_height = (track.height() * (viewport_len as f32 / total_rows as f32))
        .round()
        .clamp(min_thumb_height, track.height());
    let travel = (track.height() - thumb_height).max(0.0);
    let max_viewport_start = total_rows.saturating_sub(viewport_len);
    let start_ratio = if max_viewport_start == 0 {
        0.0
    } else {
        viewport_start.min(max_viewport_start) as f32 / max_viewport_start as f32
    };
    let thumb_min_y = track.min.y + (travel * start_ratio);
    let thumb_max_y = (thumb_min_y + thumb_height).min(track.max.y);
    let thumb = Rect::from_min_max(
        Point::new(track.min.x, thumb_min_y),
        Point::new(track.max.x, thumb_max_y.max(thumb_min_y + 1.0)),
    );
    Some(FolderScrollbarLayout { track, thumb })
}

pub(in crate::gui::native_shell::state) fn folder_scrollbar_view_start_for_pointer(
    scrollbar: FolderScrollbarLayout,
    viewport_len: usize,
    total_rows: usize,
    pointer_y: f32,
    thumb_pointer_offset_y: f32,
) -> Option<usize> {
    if viewport_len == 0 || total_rows <= viewport_len {
        return None;
    }
    let max_viewport_start = total_rows.saturating_sub(viewport_len);
    let thumb_height = scrollbar.thumb.height().max(1.0);
    let travel = (scrollbar.track.height() - thumb_height).max(0.0);
    if travel <= f32::EPSILON || max_viewport_start == 0 {
        return Some(0);
    }
    let thumb_min_y = (pointer_y - thumb_pointer_offset_y)
        .clamp(scrollbar.track.min.y, scrollbar.track.max.y - thumb_height);
    let start_ratio = ((thumb_min_y - scrollbar.track.min.y) / travel).clamp(0.0, 1.0);
    Some(((start_ratio * max_viewport_start as f32).round() as usize).min(max_viewport_start))
}

pub(in crate::gui::native_shell::state) fn folder_pane_model<'a>(
    model: &'a AppModel,
    pane: FolderPaneIdModel,
) -> &'a FolderPaneModel {
    model.sources.folder_pane(pane)
}
