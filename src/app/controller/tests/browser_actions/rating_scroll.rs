use super::super::super::test_support::{prepare_with_source_and_wav_entries, sample_entry};
use crate::app_core::ui::MAX_RENDERED_BROWSER_ROWS;
use crate::sample_sources::Rating;

#[test]
fn rating_auto_advance_works() {
    let (mut controller, _) = prepare_with_source_and_wav_entries(vec![
        sample_entry("one.wav", Rating::NEUTRAL),
        sample_entry("two.wav", Rating::NEUTRAL),
        sample_entry("three.wav", Rating::NEUTRAL),
    ]);

    controller.focus_browser_row(0);
    controller.set_advance_after_rating(true);
    assert_eq!(controller.ui.browser.selection.selected_visible, Some(0));

    controller.adjust_selected_rating(1);
    assert_eq!(controller.ui.browser.selection.selected_visible, Some(1));

    controller.adjust_selected_rating(-1);
    assert_eq!(controller.ui.browser.selection.selected_visible, Some(2));

    controller.adjust_selected_rating(1);
    assert_eq!(controller.ui.browser.selection.selected_visible, Some(2));

    controller.set_advance_after_rating(false);
    controller.focus_browser_row(0);
    controller.adjust_selected_rating(1);
    assert_eq!(controller.ui.browser.selection.selected_visible, Some(0));

    controller.set_advance_after_rating(true);
    controller.focus_browser_row(0);
    controller.tag_selected(Rating::KEEP_1);
    assert_eq!(controller.ui.browser.selection.selected_visible, Some(1));
}

#[test]
fn keyboard_focus_near_bottom_edge_advances_render_slice_without_overwriting_viewport_start() {
    let mut entries = Vec::new();
    for index in 0..(MAX_RENDERED_BROWSER_ROWS + 8) {
        entries.push(sample_entry(
            &format!("row_{index:03}.wav"),
            Rating::NEUTRAL,
        ));
    }
    let (mut controller, _source) = prepare_with_source_and_wav_entries(entries);

    controller.focus_browser_row_only(MAX_RENDERED_BROWSER_ROWS.saturating_sub(3));

    assert_eq!(
        controller.ui.browser.selection.selected_visible,
        Some(MAX_RENDERED_BROWSER_ROWS.saturating_sub(3))
    );
    assert_eq!(controller.ui.browser.viewport.render_window_start, 1);
    assert_eq!(controller.ui.browser.viewport.view_window_start, 0);
}
