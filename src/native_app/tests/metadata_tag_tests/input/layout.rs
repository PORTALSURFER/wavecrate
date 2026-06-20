use super::*;

#[test]
fn folder_browser_metadata_hides_tag_entry_when_no_file_is_selected() {
    let browser = crate::native_app::test_support::state::FolderBrowserState::load_default();
    let tags = vec![String::from("kick")];
    let theme = radiant::theme::ThemeTokens::default();
    let frame = crate::native_app::test_support::metadata_sidebar::library_sidebar_view(
        &browser,
        260.0,
        false,
        "",
        &[],
        None,
        "add tag",
        None,
        &[],
        &tags,
        &[],
        None,
    )
    .view_frame_at_size(Vector2::new(260.0, 620.0), &theme);

    assert!(!frame.paint_plan.contains_text("Tags"));
    assert!(!frame.paint_plan.contains_text("Metadata"));
    assert!(!frame.paint_plan.contains_text("Tags (1)"));
    assert!(!frame.paint_plan.contains_text("kick"));
    assert!(metadata_tag_text_input(&frame).is_none());
}

#[test]
fn folder_browser_metadata_tags_grow_combined_entry_field() {
    let browser = crate::native_app::test_support::state::FolderBrowserState::load_default();
    let small_tags = vec![String::from("kick")];
    let larger_tags = vec![
        String::from("kick"),
        String::from("warm"),
        String::from("one-shot"),
        String::from("distorted"),
    ];
    let small = crate::native_app::test_support::metadata_sidebar::library_sidebar_view(
        &browser,
        260.0,
        true,
        "",
        &[],
        None,
        "add tag",
        None,
        &[],
        &small_tags,
        &[],
        None,
    )
    .view_frame_at_size_with_default_theme(Vector2::new(260.0, 620.0));
    let larger = crate::native_app::test_support::metadata_sidebar::library_sidebar_view(
        &browser,
        260.0,
        true,
        "",
        &[],
        None,
        "add tag",
        None,
        &[],
        &larger_tags,
        &[],
        None,
    )
    .view_frame_at_size_with_default_theme(Vector2::new(260.0, 620.0));

    assert!(larger.paint_plan.contains_text("distorted"));
    assert!(!larger.paint_plan.contains_text("More"));
    assert!(frame_has_clip_height(&small, 24.0));
    let first_tag = larger
        .paint_plan
        .first_text_rect("kick")
        .expect("first tag should paint");
    let wrapped_tag = larger
        .paint_plan
        .first_text_rect("distorted")
        .expect("wrapped tag should paint");
    assert!(wrapped_tag.min.y > first_tag.min.y);
}

#[test]
fn folder_browser_metadata_tag_field_caps_at_six_rows_then_scrolls() {
    let browser = crate::native_app::test_support::state::FolderBrowserState::load_default();
    let tags = (0..24)
        .map(|index| format!("tag-{index:02}"))
        .collect::<Vec<_>>();
    let frame = crate::native_app::test_support::metadata_sidebar::library_sidebar_view(
        &browser,
        260.0,
        true,
        "",
        &[],
        None,
        "add tag",
        None,
        &[],
        &tags,
        &[],
        None,
    )
    .view_frame_at_size_with_default_theme(Vector2::new(260.0, 620.0));

    let tag_clip = frame
        .paint_plan
        .clip_starts()
        .find_map(|clip| ((clip.rect.height() - 129.0).abs() < 0.01).then_some(clip.rect));
    assert!(
        tag_clip.is_some(),
        "combined tag field should clip overflowing tag rows"
    );
}
