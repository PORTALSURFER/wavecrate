use super::*;
use crate::gui::native_shell::style::StyleTokens;

#[test]
fn folder_recovery_badge_compacts_label_when_header_is_narrow() {
    let style = StyleTokens::for_viewport_width(820.0);
    let header_rect = Rect::from_min_max(
        Point::new(0.0, 0.0),
        Point::new(58.0, style.sizing.folder_header_block_height),
    );
    let layout = compute_sidebar_folder_header_layout(
        header_rect,
        style.sizing,
        false,
        153,
        true,
        true,
        false,
        true,
    );
    if let Some(badge) = layout.badge {
        assert!(badge.label.chars().count() <= 3);
        assert!(badge.rect.min.x >= header_rect.min.x);
        assert!(badge.rect.max.x <= header_rect.max.x);
    } else {
        let toggle = layout
            .visibility_toggle_button
            .expect("narrow headers should preserve either the badge or the visibility toggle");
        assert!(toggle.rect.min.x >= header_rect.min.x);
        assert!(toggle.rect.max.x <= header_rect.max.x);
    }
}

#[test]
fn folder_header_text_rows_do_not_overlap_recovery_badge() {
    let style = StyleTokens::for_viewport_width(820.0);
    let header_rect = Rect::from_min_max(
        Point::new(24.0, 40.0),
        Point::new(120.0, 40.0 + style.sizing.folder_header_block_height),
    );
    let layout = compute_sidebar_folder_header_layout(
        header_rect,
        style.sizing,
        true,
        0,
        true,
        true,
        false,
        true,
    );
    let badge = layout
        .badge
        .expect("badge should render for active recovery");
    assert!(layout.title_row.max.x <= badge.rect.min.x);
    if let Some(meta) = layout.metadata_row {
        assert!(meta.max.x <= badge.rect.min.x);
    }
}

#[test]
fn folder_visibility_toggle_stays_inside_header_bounds() {
    let style = StyleTokens::for_viewport_width(1280.0);
    let header_rect = Rect::from_min_max(
        Point::new(24.0, 40.0),
        Point::new(220.0, 40.0 + style.sizing.folder_header_block_height),
    );
    let layout = compute_sidebar_folder_header_layout(
        header_rect,
        style.sizing,
        false,
        0,
        false,
        true,
        true,
        true,
    );
    let toggle = layout
        .visibility_toggle_button
        .expect("toggle should render when the header is wide enough");
    assert!(toggle.rect.min.x >= header_rect.min.x);
    assert!(toggle.rect.max.x <= header_rect.max.x);
    assert!(toggle.rect.min.y >= header_rect.min.y);
    assert!(toggle.rect.max.y <= header_rect.max.y);
    assert!((toggle.rect.width() - toggle.rect.height()).abs() <= 0.5);
    assert!(toggle.rect.height() <= style.sizing.sidebar_action_button_height + 0.5);
}

#[test]
fn folder_header_two_toggles_fit_without_overlap() {
    let style = StyleTokens::for_viewport_width(1280.0);
    let header_rect = Rect::from_min_max(
        Point::new(24.0, 40.0),
        Point::new(220.0, 40.0 + style.sizing.folder_header_block_height),
    );
    let layout = compute_sidebar_folder_header_layout(
        header_rect,
        style.sizing,
        false,
        0,
        false,
        true,
        true,
        true,
    );
    let visibility = layout
        .visibility_toggle_button
        .expect("visibility toggle should render");
    let flatten = layout
        .flatten_toggle_button
        .expect("flatten toggle should render");
    assert!(visibility.rect.max.x <= flatten.rect.min.x);
    assert!(flatten.rect.max.x <= header_rect.max.x);
}

#[test]
fn source_divider_stays_between_sections_when_space_is_tight() {
    let style = StyleTokens::for_viewport_width(820.0);
    let source_rows = Rect::from_min_max(Point::new(12.0, 80.0), Point::new(220.0, 220.0));
    let folder_header = Rect::from_min_max(Point::new(12.0, 224.0), Point::new(220.0, 252.0));
    let divider = compute_source_section_divider_rect(source_rows, folder_header, style.sizing)
        .expect("divider should exist");
    assert!(divider.min.x >= source_rows.min.x);
    assert!(divider.max.x <= source_rows.max.x);
    assert!(divider.min.y >= source_rows.max.y);
    assert!(divider.max.y <= folder_header.max.y);
}
