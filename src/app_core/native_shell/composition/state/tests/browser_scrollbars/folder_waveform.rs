use super::*;
use crate::compat_app_contract::FolderPaneIdModel;

#[allow(dead_code)]
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
    model.browser.autoscroll = true;
    model.browser.selected_visible_row = Some(focused_visible_row);
    model.browser.anchor_visible_row = Some(focused_visible_row.saturating_sub(2));
    model
}

#[test]
fn prewindowed_browser_scrollbar_uses_manual_view_start_at_bottom() {
    let layout = ShellLayout::build(Vector2::new(1280.0, 720.0));
    let style = style_for_layout(&layout);
    let viewport_len = browser_rows_capacity(layout.browser_rows, style.sizing);
    let visible_count = 1_506usize;
    let host_window_len = 256usize;
    let render_window_start = visible_count - host_window_len;
    let requested_view_start = visible_count - viewport_len;

    let mut model = AppModel::default();
    model.browser.visible_count = visible_count;
    model.browser.autoscroll = false;
    model.browser.view_start_row = requested_view_start;
    model.browser.selected_visible_row = Some(render_window_start + 32);
    model.browser.anchor_visible_row = Some(render_window_start + 30);
    for visible_row in render_window_start..visible_count {
        model.browser.rows.push(BrowserRowModel::new(
            visible_row,
            format!("row_{visible_row:04}"),
            1,
            false,
            visible_row == render_window_start + 32,
        ));
    }

    let rows = rendered_browser_rows(&layout, &model, &style);
    let scrollbar = browser_scrollbar_layout(
        layout.browser_rows,
        &rows,
        model.browser.visible_count,
        style.sizing,
    )
    .expect("prewindowed browser list should render a scrollbar");

    assert_eq!(
        rows.first().map(|row| row.visible_row),
        Some(requested_view_start)
    );
    assert_eq!(scrollbar.thumb.max.y, scrollbar.track.max.y);
}

#[test]
fn overflowing_folder_lists_render_scrollbar_thumb_at_view_position() {
    let layout = ShellLayout::build(Vector2::new(1280.0, 720.0));
    let style = style_for_layout(&layout);
    let mut state = NativeShellState::new();
    let top_model = folder_model_with_rows(240, 6);
    let top_rows = state
        .cached_tree_rows(&layout, &style, &top_model, FolderPaneIdModel::Upper)
        .to_vec();
    let top_sections = sidebar_sections(&layout, &style, &top_model);
    let top_content_rect = tree_rows_content_rect(
        top_sections.tree_rows(FolderPaneIdModel::Upper),
        top_model.sources.tree_rows.len(),
        style.sizing,
    );
    let top_scrollbar = folder_scrollbar_layout(
        top_sections.tree_rows(FolderPaneIdModel::Upper),
        &top_rows,
        top_model.sources.tree_rows.len(),
        style.sizing,
    )
    .expect("overflowing folder list should render a scrollbar");

    let lower_model = folder_model_with_rows(240, 148);
    let lower_rows = state
        .cached_tree_rows(&layout, &style, &lower_model, FolderPaneIdModel::Lower)
        .to_vec();
    let lower_sections = sidebar_sections(&layout, &style, &lower_model);
    let lower_content_rect = tree_rows_content_rect(
        lower_sections.tree_rows(FolderPaneIdModel::Lower),
        lower_model.sources.tree_rows.len(),
        style.sizing,
    );
    let lower_scrollbar = folder_scrollbar_layout(
        lower_sections.tree_rows(FolderPaneIdModel::Lower),
        &lower_rows,
        lower_model.sources.tree_rows.len(),
        style.sizing,
    )
    .expect("overflowing folder list should render a scrollbar");

    assert_rect_inside(
        top_sections.tree_rows(FolderPaneIdModel::Upper),
        top_scrollbar.track,
    );
    assert_rect_inside(
        top_sections.tree_rows(FolderPaneIdModel::Upper),
        top_scrollbar.thumb,
    );
    assert!(top_content_rect.max.x < top_scrollbar.track.min.x);
    assert!(lower_content_rect.max.x < lower_scrollbar.track.min.x);
    assert!(
        top_rows
            .iter()
            .all(|row| row.rect.max.x <= top_content_rect.max.x)
    );
    assert!(
        lower_rows
            .iter()
            .all(|row| row.rect.max.x <= lower_content_rect.max.x)
    );
    assert!(lower_scrollbar.thumb.min.y > top_scrollbar.thumb.min.y);

    let frame = state.build_frame(&layout, &lower_model);
    let track_color = blend_color(style.border, style.bg_secondary, 0.22);
    let thumb_color = blend_color(style.text_muted, style.text_primary, 0.32);
    assert!(frame.primitives.iter().any(|primitive| matches!(
        primitive,
        Primitive::Rect(rect)
            if rect.rect == lower_scrollbar.track && rect.color == track_color
    )));
    assert!(frame.primitives.iter().any(|primitive| matches!(
        primitive,
        Primitive::Rect(rect)
            if rect.rect == lower_scrollbar.thumb && rect.color == thumb_color
    )));
}

#[test]
fn prewindowed_folder_scrollbar_uses_manual_view_start_at_bottom() {
    let layout = ShellLayout::build(Vector2::new(1280.0, 720.0));
    let style = style_for_layout(&layout);
    let mut model = folder_model_with_rows(240, 0);
    model.sources.focused_tree_row = Some(0);
    let mut state = NativeShellState::new();
    let initial_rows = state
        .cached_tree_rows(&layout, &style, &model, FolderPaneIdModel::Upper)
        .to_vec();
    let viewport_len = initial_rows.len();
    let requested_view_start = model.sources.tree_rows.len().saturating_sub(viewport_len);
    assert!(state.set_folder_view_start_row(FolderPaneIdModel::Upper, requested_view_start));

    let rows = state
        .cached_tree_rows(&layout, &style, &model, FolderPaneIdModel::Upper)
        .to_vec();
    let sections = sidebar_sections(&layout, &style, &model);
    let scrollbar = folder_scrollbar_layout(
        sections.tree_rows(FolderPaneIdModel::Upper),
        &rows,
        model.sources.tree_rows.len(),
        style.sizing,
    )
    .expect("overflowing folder list should render a scrollbar");

    assert_eq!(
        rows.first().map(|row| row.row_index),
        Some(requested_view_start)
    );
    assert_eq!(scrollbar.thumb.max.y, scrollbar.track.max.y);
}

#[test]
fn waveform_scrollbar_thumb_tracks_view_position() {
    let layout = ShellLayout::build(Vector2::new(1280.0, 720.0));
    let scrollbar = waveform_scrollbar_layout(layout.waveform_scrollbar_lane, 250_000, 500_000)
        .expect("waveform scrollbar should render for valid plot geometry");

    assert!(scrollbar.track.min.y >= layout.waveform_scrollbar_lane.min.y);
    assert!(scrollbar.track.max.y <= layout.waveform_scrollbar_lane.max.y);
    assert!(scrollbar.track.min.y >= layout.waveform_plot.max.y);
    assert!(scrollbar.thumb.min.x > scrollbar.track.min.x);
    assert!(scrollbar.thumb.max.x < scrollbar.track.max.x);
}

#[test]
fn waveform_scrollbar_thumb_hit_test_returns_drag_offset() {
    let layout = ShellLayout::build(Vector2::new(1280.0, 720.0));
    let mut model = AppModel::default();
    model.waveform.view_start_micros = 250_000;
    model.waveform.view_end_micros = 500_000;
    let scrollbar = waveform_scrollbar_layout(
        layout.waveform_scrollbar_lane,
        model.waveform.view_start_micros,
        model.waveform.view_end_micros,
    )
    .expect("waveform scrollbar should render for valid plot geometry");
    let point = Point::new(
        (scrollbar.thumb.min.x + scrollbar.thumb.max.x) * 0.5,
        (scrollbar.thumb.min.y + scrollbar.thumb.max.y) * 0.5,
    );

    let state = NativeShellState::new();
    let offset = state
        .waveform_scrollbar_thumb_offset_at_point(&layout, &model, point)
        .expect("waveform thumb center should be hittable");
    assert!((offset - (scrollbar.thumb.width() * 0.5)).abs() <= 0.001);
}

#[test]
fn waveform_scrollbar_track_click_maps_to_centered_view() {
    let layout = ShellLayout::build(Vector2::new(1280.0, 720.0));
    let mut model = AppModel::default();
    model.waveform.view_start_micros = 250_000;
    model.waveform.view_end_micros = 500_000;
    let scrollbar = waveform_scrollbar_layout(
        layout.waveform_scrollbar_lane,
        model.waveform.view_start_micros,
        model.waveform.view_end_micros,
    )
    .expect("waveform scrollbar should render for valid plot geometry");
    let point = Point::new(
        scrollbar.track.max.x - 24.0,
        (scrollbar.track.min.y + scrollbar.track.max.y) * 0.5,
    );
    let expected_center = waveform_scrollbar_center_for_pointer(
        scrollbar,
        model.waveform.view_start_micros,
        model.waveform.view_end_micros,
        point.x,
        scrollbar.thumb.width() * 0.5,
    )
    .expect("track click should resolve a waveform view center");

    let state = NativeShellState::new();
    assert_eq!(
        state.waveform_scrollbar_view_center_at_point(&layout, &model, point),
        Some(expected_center)
    );
}

fn folder_model_with_rows(total_rows: usize, focused_row: usize) -> AppModel {
    let mut model = AppModel::default();
    model.sources.focused_tree_row = Some(focused_row.min(total_rows.saturating_sub(1)));
    for row_index in 0..total_rows {
        let row = FolderRowModel::new(
            format!("folder_{row_index:03}"),
            format!("folder_{row_index:03}"),
            0,
            false,
            row_index == focused_row,
            row_index == 0,
            row_index < total_rows.saturating_sub(1),
            true,
        );
        model.sources.tree_rows.push(row.clone());
        model.sources.upper_folder_pane.tree_rows.push(row.clone());
        model.sources.lower_folder_pane.tree_rows.push(row);
    }
    model.sources.upper_folder_pane.focused_tree_row = model.sources.focused_tree_row;
    model.sources.lower_folder_pane.focused_tree_row = model.sources.focused_tree_row;
    model
}
