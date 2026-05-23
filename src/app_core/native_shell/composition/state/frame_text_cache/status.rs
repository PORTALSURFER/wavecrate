//! Status-bar text payload construction for retained frames.

use super::*;
use crate::app_core::native_shell::composition::status_surface::StatusSurfaceLayout;

pub(super) fn build_status_bar_text_cache(
    layout: &ShellLayout,
    style: &StyleTokens,
    model: &AppModel,
    transport_running: bool,
    selected_column: usize,
) -> StatusBarTextCacheValue {
    let sizing = style.sizing;
    let inline_progress_active = model.progress_overlay.visible && !model.progress_overlay.modal;
    let status_surface = resolve_status_surface_layout(
        layout.status_bar,
        sizing,
        &StatusSurfaceContent {
            left_label: status_left_text(model, transport_running, selected_column),
            center_label: status_center_label(model, inline_progress_active),
            right_label: status_right_label(model, selected_column),
            progress_counter: cached_status_progress_counter(model),
        },
    );
    StatusBarTextCacheValue {
        left_text_rect: status_surface.left_text_rect,
        center_text_rect: status_surface.center_text_rect,
        right_text_rect: status_surface.right_text_rect,
        progress_text_rect: status_surface.progress_text_rect,
        progress_track_rect: status_surface.progress_track_rect,
        left_label: truncated_status_left(
            model,
            transport_running,
            selected_column,
            &status_surface,
            sizing,
        ),
        center_label: truncated_status_center(model, &status_surface, sizing),
        right_label: truncated_status_right(model, selected_column, &status_surface, sizing),
        progress_label: truncated_progress_label(model, &status_surface, sizing),
        progress_counter: truncated_progress_counter(model, &status_surface, sizing),
        inline_progress_active,
    }
}

fn status_center_label(model: &AppModel, inline_progress_active: bool) -> String {
    if inline_progress_active {
        cached_status_progress_label(model)
    } else {
        status_center_text(model)
    }
}

fn truncated_status_left(
    model: &AppModel,
    transport_running: bool,
    selected_column: usize,
    status_surface: &StatusSurfaceLayout,
    sizing: SizingTokens,
) -> String {
    truncate_to_width(
        &status_left_text(model, transport_running, selected_column),
        status_surface.left_text_rect.width().max(36.0),
        sizing.font_status,
    )
}

fn truncated_status_center(
    model: &AppModel,
    status_surface: &StatusSurfaceLayout,
    sizing: SizingTokens,
) -> String {
    truncate_to_width(
        &status_center_text(model),
        status_surface.center_text_rect.width().max(36.0),
        sizing.font_status,
    )
}

fn truncated_status_right(
    model: &AppModel,
    selected_column: usize,
    status_surface: &StatusSurfaceLayout,
    sizing: SizingTokens,
) -> String {
    truncate_to_width(
        &status_right_label(model, selected_column),
        status_surface.right_text_rect.width().max(36.0),
        sizing.font_status,
    )
}

fn truncated_progress_label(
    model: &AppModel,
    status_surface: &StatusSurfaceLayout,
    sizing: SizingTokens,
) -> String {
    truncate_to_width(
        &cached_status_progress_label(model),
        status_surface.center_text_rect.width().max(36.0),
        sizing.font_status,
    )
}

fn truncated_progress_counter(
    model: &AppModel,
    status_surface: &StatusSurfaceLayout,
    sizing: SizingTokens,
) -> String {
    truncate_to_width(
        &cached_status_progress_counter(model),
        status_surface.progress_text_rect.width().max(24.0),
        sizing.font_status,
    )
}

fn status_left_text(model: &AppModel, transport_running: bool, selected_column: usize) -> String {
    if !model.status.left.is_empty() {
        return model.status.left.clone();
    }
    if model.status_text.is_empty() {
        return format!(
            "Transport: {} | Selected column: {}",
            if transport_running {
                "running"
            } else {
                "stopped"
            },
            selected_column + 1
        );
    }
    model.status_text.clone()
}

fn status_center_text(model: &AppModel) -> String {
    if !model.status.center.is_empty() {
        return model.status.center.clone();
    }
    format!(
        "rows: {} | selected: {} | anchor: {} | search: {}{}",
        model.browser.visible_count,
        model.browser.selected_item_count,
        browser_anchor_label(model),
        browser_search_label(model),
        if model.browser.busy {
            " | filtering…"
        } else {
            ""
        }
    )
}

fn browser_anchor_label(model: &AppModel) -> String {
    model
        .browser
        .anchor_visible_row
        .map(|row| row.to_string())
        .unwrap_or_else(|| String::from("—"))
}

fn browser_search_label(model: &AppModel) -> &str {
    if model.browser.search_query.is_empty() {
        "—"
    } else {
        model.browser.search_query.as_str()
    }
}

fn status_right_label(model: &AppModel, selected_column: usize) -> String {
    if model.status.right.is_empty() {
        return format!("col: {}/3", selected_column + 1);
    }
    model.status.right.clone()
}

fn cached_status_progress_label(model: &AppModel) -> String {
    if model.progress_overlay.cancel_requested {
        return format!("Cancelling {}", cached_progress_title(model));
    }
    match model.progress_overlay.detail.as_deref() {
        Some(detail) if !detail.trim().is_empty() => {
            format!("{} • {}", cached_progress_title(model), detail.trim())
        }
        _ => cached_progress_title(model),
    }
}

fn cached_status_progress_counter(model: &AppModel) -> String {
    if model.progress_overlay.cancel_requested {
        return String::from("cancelling");
    }
    if model.progress_overlay.total == 0 {
        return open_progress_counter(model.progress_overlay.completed);
    }
    format!(
        "{}/{}",
        model.progress_overlay.completed, model.progress_overlay.total
    )
}

fn open_progress_counter(completed: usize) -> String {
    if completed > 0 {
        cached_format_file_counter(completed)
    } else {
        String::from("busy")
    }
}

fn cached_progress_title(model: &AppModel) -> String {
    let title = model.progress_overlay.title.trim();
    if title.is_empty() {
        String::from("Working")
    } else {
        title.to_string()
    }
}

fn cached_format_file_counter(completed: usize) -> String {
    match completed {
        1 => String::from("1 file"),
        _ => format!("{completed} files"),
    }
}
