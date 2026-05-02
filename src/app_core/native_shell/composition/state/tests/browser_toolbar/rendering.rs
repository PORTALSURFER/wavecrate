use super::*;

fn rect_contains_rect(outer: Rect, inner: Rect) -> bool {
    inner.min.x >= outer.min.x
        && inner.min.y >= outer.min.y
        && inner.max.x <= outer.max.x
        && inner.max.y <= outer.max.y
}

#[test]
fn browser_filter_icons_replace_legacy_age_and_mark_labels() {
    let layout = ShellLayout::build(Vector2::new(1280.0, 720.0));
    let model = AppModel::default();
    let mut state = NativeShellState::new();
    let frame = state.build_frame(&layout, &model);

    assert!(
        !frame
            .text_runs
            .iter()
            .any(|run| matches!(run.text.as_str(), "NVR" | "1M" | "1W" | "MARK"))
    );

    for chip in [
        crate::compat_app_contract::PlaybackAgeFilterChip::NeverPlayed,
        crate::compat_app_contract::PlaybackAgeFilterChip::OlderThanMonth,
        crate::compat_app_contract::PlaybackAgeFilterChip::OlderThanWeek,
    ] {
        let chip_rect = state
            .browser_playback_age_filter_chip_rect(&layout, &model, chip)
            .expect("playback-age chip should render");
        assert!(frame.primitives.iter().any(|primitive| {
            matches!(
                primitive,
                Primitive::Image(image) if rect_contains_rect(chip_rect, image.rect)
            )
        }));
    }

    let marked_chip = state
        .browser_marked_filter_chip_rect(&layout, &model)
        .expect("marked filter chip should render");
    assert!(frame.primitives.iter().any(|primitive| {
        matches!(
            primitive,
            Primitive::Image(image) if rect_contains_rect(marked_chip, image.rect)
        )
    }));
}

#[test]
fn browser_frame_build_reuses_cached_toolbar_geometry() {
    let layout = ShellLayout::build(Vector2::new(1280.0, 720.0));
    let style = style_for_layout(&layout);
    let model = browser_model_with_rows(24, 8);
    let mut state = NativeShellState::new();
    let mut segments = StaticFrameSegments::default();

    state.build_static_segment_with_style_into(
        &layout,
        &style,
        &model,
        None,
        StaticFrameSegment::BrowserFrame,
        &mut segments,
    );

    assert!(state.browser_action_hit_test_cache_key.is_some());
    assert!(state.browser_toolbar_layout.is_some());
    assert!(!state.browser_action_buttons.is_empty());
}

#[test]
fn browser_frame_text_cache_reuses_and_invalidates_on_search_changes() {
    let layout = ShellLayout::build(Vector2::new(1280.0, 720.0));
    let style = style_for_layout(&layout);
    let mut model = browser_model_with_rows(24, 8);
    model.browser.search_query = String::from("kick");
    let mut state = NativeShellState::new();
    let mut segments = StaticFrameSegments::default();

    state.build_static_segment_with_style_into(
        &layout,
        &style,
        &model,
        None,
        StaticFrameSegment::BrowserFrame,
        &mut segments,
    );
    let first = state.browser_segment_text_frame_counts();
    assert_eq!(first.lookup_count, 2);
    assert_eq!(first.cache_hit_count, 1);
    assert_eq!(first.cache_miss_count, 1);

    state.build_static_segment_with_style_into(
        &layout,
        &style,
        &model,
        None,
        StaticFrameSegment::BrowserFrame,
        &mut segments,
    );
    let second = state.browser_segment_text_frame_counts();
    assert_eq!(second.lookup_count, 2);
    assert_eq!(second.cache_hit_count, 2);
    assert_eq!(second.cache_miss_count, 0);

    model.browser.search_query = String::from("snare");
    state.build_static_segment_with_style_into(
        &layout,
        &style,
        &model,
        None,
        StaticFrameSegment::BrowserFrame,
        &mut segments,
    );
    let third = state.browser_segment_text_frame_counts();
    assert_eq!(third.lookup_count, 2);
    assert_eq!(third.cache_hit_count, 1);
    assert_eq!(third.cache_miss_count, 1);
}
