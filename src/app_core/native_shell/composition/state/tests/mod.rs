use super::*;
use crate::compat_app_contract::{
    BrowserRowModel, FolderActionsModel, FolderRowModel, NativeMotionModel, NormalizedRangeModel,
    SourceRowModel,
};
use crate::gui::types::{ImageRgba, Point, Rgba8, Vector2};

pub(crate) fn populated_sidebar_model() -> AppModel {
    let mut model = AppModel::default();
    for index in 0..20 {
        model.sources.rows.push(SourceRowModel::new(
            format!("source_{index:02}"),
            format!("detail_{index:02}"),
            false,
            false,
        ));
    }
    if let Some(row) = model.sources.rows.get_mut(2) {
        row.assigned_to_upper_pane = true;
    }
    if let Some(row) = model.sources.rows.get_mut(5) {
        row.assigned_to_lower_pane = true;
    }
    model.sources.upper_folder_pane.item_label = String::from("source_02");
    model.sources.upper_folder_pane.item_detail = String::from("detail_02");
    model.sources.upper_folder_pane.active = true;
    model.sources.upper_folder_pane.has_item = true;
    model.sources.lower_folder_pane.item_label = String::from("source_05");
    model.sources.lower_folder_pane.item_detail = String::from("detail_05");
    model.sources.lower_folder_pane.has_item = true;
    for index in 0..24 {
        let row = FolderRowModel::new(
            format!("folder_{index:02}"),
            String::new(),
            index % 4,
            false,
            index == 3,
            index == 0,
            true,
            true,
        );
        model.sources.upper_folder_pane.tree_rows.push(row.clone());
        model.sources.lower_folder_pane.tree_rows.push(row);
    }
    model.sources.focused_tree_row = Some(3);
    model.sources.tree_rows = model.sources.upper_folder_pane.tree_rows.clone();
    model.sources.tree_actions = FolderActionsModel {
        can_create_child: true,
        can_create_root: true,
        can_rename: true,
        can_delete: true,
        can_restore_retained: true,
        can_purge_retained: true,
        can_clear_history: true,
    };
    model.sources.upper_folder_pane.tree_actions = model.sources.tree_actions.clone();
    model.sources.lower_folder_pane.tree_actions = model.sources.tree_actions.clone();
    model.sources.can_toggle_show_all_items = true;
    model.sources.can_toggle_flattened_view = true;
    model.sources.upper_folder_pane.can_toggle_show_all_items = true;
    model.sources.upper_folder_pane.can_toggle_flattened_view = true;
    model.sources.upper_folder_pane.focused_tree_row = Some(3);
    model.sources.lower_folder_pane.can_toggle_show_all_items = true;
    model.sources.lower_folder_pane.can_toggle_flattened_view = true;
    model.sources.lower_folder_pane.focused_tree_row = Some(3);
    model
}

pub(crate) fn push_active_folder_row(model: &mut AppModel, row: FolderRowModel) {
    model.sources.tree_rows.push(row.clone());
    model.sources.upper_folder_pane.tree_rows.push(row.clone());
    model.sources.lower_folder_pane.tree_rows.push(row);
    model.sources.upper_folder_pane.active = true;
    model.sources.upper_folder_pane.has_item = true;
}

pub(crate) fn browser_model_with_rows(total: usize, focused_visible_row: usize) -> AppModel {
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
    model.browser.autoscroll = true;
    model.browser.selected_visible_row = Some(focused_visible_row);
    model.browser.anchor_visible_row = Some(focused_visible_row.saturating_sub(2));
    model
}

/// Build cached browser rows from rects for hit-test unit coverage.
fn cached_browser_rows_from_rects(rects: &[Rect]) -> Vec<CachedBrowserRow> {
    let style = StyleTokens::default();
    rects
        .iter()
        .copied()
        .enumerate()
        .map(|(index, rect)| {
            let label = format!("row_{index}");
            CachedBrowserRow {
                visible_row: index,
                visible_row_label: index.to_string(),
                label_rendered_width: browser_approx_text_width(&label, style.sizing.font_body),
                label,
                bucket_label: String::new(),
                inline_tag_labels: Vec::new(),
                inline_tag_rects: Vec::new(),
                text_layout: compute_browser_row_text_layout(rect, style.sizing),
                playback_age_bucket: crate::compat_app_contract::PlaybackAgeBucket::Fresh,
                similarity_display_strength: None,
                column: 1,
                rating_level: 0,
                selected: false,
                focused: false,
                missing: false,
                locked: false,
                marked: false,
                processing_state: crate::compat_app_contract::BrowserRowProcessingState::None,
                rect,
            }
        })
        .collect()
}

fn assert_rect_inside(outer: Rect, inner: Rect) {
    assert!(inner.min.x >= outer.min.x);
    assert!(inner.min.y >= outer.min.y);
    assert!(inner.max.x <= outer.max.x);
    assert!(inner.max.y <= outer.max.y);
}

fn assert_text_run_inside_band(run: &TextRun, band: Rect) {
    assert!(run.position.x >= band.min.x);
    assert!(run.position.x <= band.max.x);
    assert!(run.position.y >= band.min.y);
    assert!(run.position.y + run.font_size <= band.max.y + 0.5);
}

fn playhead_trail_primitive_bounds(
    primitive: &Primitive,
    playhead_rect: Rect,
    style: &StyleTokens,
) -> Option<(f32, f32)> {
    match primitive {
        Primitive::Rect(rect)
            if rect.rect.min.y == playhead_rect.min.y
                && rect.rect.max.y == playhead_rect.max.y
                && rect.color.a > 0
                && rect.color != style.accent_copper =>
        {
            Some((rect.rect.min.x, rect.rect.max.x))
        }
        Primitive::LinearGradient(gradient)
            if gradient.rect.min.y == playhead_rect.min.y
                && gradient.rect.max.y == playhead_rect.max.y
                && (gradient.start_color.a > 0 || gradient.end_color.a > 0)
                && gradient.start_color != style.accent_copper
                && gradient.end_color != style.accent_copper =>
        {
            Some((gradient.rect.min.x, gradient.rect.max.x))
        }
        _ => None,
    }
}

fn playhead_trail_primitive_count(
    frame: &NativeViewFrame,
    playhead_rect: Rect,
    style: &StyleTokens,
) -> usize {
    frame
        .primitives
        .iter()
        .filter(|primitive| {
            playhead_trail_primitive_bounds(primitive, playhead_rect, style).is_some()
        })
        .count()
}

mod browser_rows;
mod browser_scrollbars;
mod browser_toolbar;
mod chrome_layout;
mod folder_visibility_toggle;
mod frame_build;
mod overlay_controls;
mod overlays;
mod playhead_trail_render;
mod playhead_trail_state;
mod selection_states;
mod sidebar;
mod status_bar_progress;
mod waveform_edit_fades;
mod waveform_edit_handles;
mod waveform_selection;
mod waveform_slices;
