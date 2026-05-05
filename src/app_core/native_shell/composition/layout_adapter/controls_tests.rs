//! Tests for slotized control-row and toolbar partition helpers.

use super::{
    compute_browser_toolbar_sections, compute_sidebar_action_button_rects,
    compute_update_action_button_rects,
};
use crate::gui::native_shell::style::StyleTokens;
use crate::gui::types::{Point, Rect};

#[test]
fn update_action_buttons_right_align_and_fit_cluster() {
    let style = StyleTokens::for_viewport_width(1280.0);
    let row = Rect::from_min_max(Point::new(0.0, 0.0), Point::new(1280.0, 24.0));
    let cluster = Rect::from_min_max(Point::new(980.0, 0.0), Point::new(1260.0, 24.0));
    let rects = compute_update_action_button_rects(
        row,
        cluster,
        style.sizing,
        &["Open", "Install", "Dismiss"],
    );
    assert!(!rects.is_empty());
    for rect in &rects {
        assert!(rect.min.x >= cluster.min.x);
        assert!(rect.max.x <= cluster.max.x);
        assert!(rect.min.y >= row.min.y);
        assert!(rect.max.y <= row.max.y);
    }
}

#[test]
fn toolbar_sections_stay_left_of_action_cluster() {
    let style = StyleTokens::for_viewport_width(1280.0);
    let toolbar = Rect::from_min_max(Point::new(300.0, 200.0), Point::new(1180.0, 220.0));
    let sections = compute_browser_toolbar_sections(toolbar, style.sizing);
    assert!(sections.rating_filter_chips.iter().all(|rect| {
        rect.min.x >= toolbar.min.x && rect.max.x <= sections.action_slots[0].min.x
    }));
    assert!(sections.action_slots[1].max.x <= sections.search_field.min.x);
    assert!(sections.search_field.min.x >= toolbar.min.x);
    assert!(sections.search_field.max.x <= toolbar.max.x - style.sizing.text_inset_x);
    assert!(sections.search_field.width() < toolbar.width());
    assert!(sections.activity_chip.width() <= 0.0);
    assert!(sections.sort_chip.width() <= 0.0);
    assert!(
        sections
            .triage_chips
            .into_iter()
            .all(|chip| chip.width() <= 0.0)
    );
}

#[test]
fn toolbar_search_field_uses_ratio_width_inside_full_host() {
    let style = StyleTokens::for_viewport_width(1280.0);
    let toolbar = Rect::from_min_max(Point::new(300.0, 200.0), Point::new(1180.0, 220.0));
    let sections = compute_browser_toolbar_sections(toolbar, style.sizing);
    let host_width = toolbar.width() - (style.sizing.text_inset_x * 2.0);
    let last_chip = sections
        .rating_filter_chips
        .iter()
        .copied()
        .filter(|rect| rect.width() > 1.0)
        .last()
        .expect("rating filter chips should render");
    assert_eq!(
        sections.rating_filter_chips[0].min.x,
        toolbar.min.x + style.sizing.text_inset_x
    );
    assert!(sections.action_slots[0].min.x > last_chip.max.x);
    assert!(sections.search_field.min.x > sections.action_slots[1].max.x);
    assert!(sections.search_field.width() >= style.sizing.browser_search_field_min_width);
    assert!(sections.search_field.width() < toolbar.width() - (style.sizing.text_inset_x * 2.0));
    assert!(sections.search_field.width() <= host_width * 0.26);
    assert!(
        sections
            .action_slots
            .iter()
            .take(2)
            .all(|rect| (rect.width() - rect.height()).abs() <= 0.01)
    );
    assert!(
        sections.action_slots[2].width() >= sections.action_slots[0].width(),
        "right-side status slot should remain at least as wide as the square action buttons"
    );
    assert!(
        sections
            .action_slots
            .iter()
            .all(|rect| rect.min.y >= toolbar.min.y && rect.max.y <= toolbar.max.y)
    );
    assert!(
        sections
            .rating_filter_chips
            .iter()
            .filter(|rect| rect.width() > 1.0)
            .all(|rect| (rect.width() - rect.height()).abs() <= 0.01)
    );
    assert_eq!(
        sections
            .rating_filter_chips
            .iter()
            .filter(|rect| rect.width() > 1.0)
            .count(),
        8
    );
}

#[test]
fn sidebar_buttons_stay_inside_footer() {
    let style = StyleTokens::for_viewport_width(1280.0);
    let footer = Rect::from_min_max(Point::new(20.0, 640.0), Point::new(280.0, 700.0));
    let rects = compute_sidebar_action_button_rects(footer, style.sizing, 5);
    assert_eq!(rects.len(), 5);
    for rect in &rects {
        assert!(rect.min.x >= footer.min.x);
        assert!(rect.max.x <= footer.max.x);
        assert!(rect.min.y >= footer.min.y);
        assert!(rect.max.y <= footer.max.y);
    }
}
