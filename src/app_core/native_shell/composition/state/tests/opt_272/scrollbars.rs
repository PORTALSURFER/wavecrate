use super::*;

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
