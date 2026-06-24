use super::*;

#[test]
fn pending_category_and_input_wrap_as_one_group() {
    let accepted = String::from("short");
    let pending_label = "deep-kick ->";
    let input_width = tag_input_width_with_completion("sound-type", None);
    let content_width =
        tag_pill_width(&accepted) + TAG_FIELD_ITEM_GAP + tag_pill_width(pending_label) + 1.0;

    let rows = tag_field_rows(
        std::slice::from_ref(&accepted),
        &[],
        Some("deep-kick"),
        input_width,
        content_width,
        None,
    );

    assert_eq!(rows.len(), 2);
    assert_eq!(rows[0], [TagEntryRowItem::Accepted(accepted)]);
    assert_eq!(
        rows[1],
        [
            TagEntryRowItem::PendingCategory(String::from(pending_label)),
            TagEntryRowItem::Input(input_width),
        ]
    );
}

#[test]
fn library_toggle_stays_after_trailing_input() {
    let accepted = String::from("short");
    let input_width = tag_input_width_with_completion("sound-type", None);
    let toggle_width = 22.0;

    let rows = tag_field_rows(
        std::slice::from_ref(&accepted),
        &[],
        None,
        input_width,
        320.0,
        Some(toggle_width),
    );

    assert_eq!(rows.len(), 1);
    assert_eq!(
        rows[0],
        [
            TagEntryRowItem::Accepted(accepted),
            TagEntryRowItem::Input(TAG_INPUT_MIN_WIDTH),
            TagEntryRowItem::LibraryToggle(toggle_width),
        ]
    );
}

#[test]
fn pending_category_reserves_library_toggle_width() {
    let pending_label = "deep-kick ->";
    let input_width = tag_input_width_with_completion("sound-type", None);
    let toggle_width = 22.0;

    let rows = tag_field_rows(
        &[],
        &[],
        Some("deep-kick"),
        input_width,
        320.0,
        Some(toggle_width),
    );

    assert_eq!(
        rows,
        [vec![
            TagEntryRowItem::PendingCategory(String::from(pending_label)),
            TagEntryRowItem::Input(TAG_INPUT_MIN_WIDTH),
            TagEntryRowItem::LibraryToggle(toggle_width),
        ]]
    );
}

#[test]
fn tag_entry_field_projection_merges_tokens_once_and_orders_by_category() {
    let tags = vec![String::from("kick")];
    let tokens = vec![String::from("kick"), String::from("loop")];
    let categories = vec![
        MetadataTagDisplayCategory {
            tag: String::from("loop"),
            category_id: "playback-type",
        },
        MetadataTagDisplayCategory {
            tag: String::from("kick"),
            category_id: "sound-type",
        },
    ];

    let projection = TagEntryFieldProjection::from_input(TagEntryFieldInput {
        draft: "",
        tokens: &tokens,
        pending_category_tag: None,
        placeholder: "add tag",
        completion_suffix: None,
        tags: &tags,
        display_categories: &categories,
        content_width: 420.0,
        library_toggle_width: None,
    });

    assert!(
        projection
            .rows
            .iter()
            .flatten()
            .any(|item| { matches!(item, TagEntryRowItem::Accepted(tag) if tag == "loop") })
    );
    assert_eq!(
        projection
            .rows
            .iter()
            .flatten()
            .filter(|item| matches!(item, TagEntryRowItem::Accepted(tag) if tag == "kick"))
            .count(),
        1
    );
}
