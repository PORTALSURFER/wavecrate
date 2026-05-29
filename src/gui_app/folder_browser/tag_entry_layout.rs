use crate::gui_app::metadata_tags::{
    MetadataTagDisplayCategory, inferred_metadata_tag_category_id_for_name,
    metadata_tag_category_order,
};

pub(super) const TAG_FIELD_CONTROL_HEIGHT: f32 = 18.0;
pub(super) const TAG_FIELD_ITEM_GAP: f32 = 3.0;
pub(super) const TAG_FIELD_LINE_GAP: f32 = 3.0;
pub(super) const TAG_FIELD_HORIZONTAL_CHROME: f32 = 26.0;
pub(super) const TAG_FIELD_VERTICAL_CHROME: f32 = 6.0;
pub(super) const MAX_TAG_FIELD_ROWS: usize = 6;
const MIN_TAG_INPUT_REMAINING_WIDTH: f32 = 180.0;

#[derive(Clone, Debug, PartialEq)]
pub(super) enum TagEntryRowItem {
    Accepted(String),
    PendingCategory(String),
    Input(f32),
}

pub(super) fn tag_field_rows(
    tags: &[String],
    tag_display_categories: &[MetadataTagDisplayCategory],
    pending_category_tag: Option<&str>,
    input_width: f32,
    content_width: f32,
) -> Vec<Vec<TagEntryRowItem>> {
    let mut visible_tags = tags.to_vec();
    order_metadata_tags_for_display(&mut visible_tags, tag_display_categories);
    let mut rows = pack_tag_rows(&visible_tags, content_width);
    if should_break_before_tag_input(&visible_tags, input_width, content_width) || rows.is_empty() {
        rows.push(Vec::new());
    }

    if let Some(tag) = pending_category_tag {
        let label = format!("{tag} ->");
        push_row_item(
            &mut rows,
            TagEntryRowItem::PendingCategory(label.clone()),
            tag_pill_width(&label),
            content_width,
        );
    }
    let input_width = if rows.last().is_some_and(Vec::is_empty) {
        content_width
    } else {
        input_width
    };
    push_row_item(
        &mut rows,
        TagEntryRowItem::Input(input_width),
        input_width,
        content_width,
    );
    rows
}

fn pack_tag_rows(tags: &[String], content_width: f32) -> Vec<Vec<TagEntryRowItem>> {
    let mut rows = Vec::new();
    for tag in tags {
        push_row_item(
            &mut rows,
            TagEntryRowItem::Accepted(tag.clone()),
            tag_pill_width(tag),
            content_width,
        );
    }
    rows
}

pub(super) fn order_metadata_tags_for_display(
    tags: &mut Vec<String>,
    tag_display_categories: &[MetadataTagDisplayCategory],
) {
    let mut indexed = tags.drain(..).enumerate().collect::<Vec<_>>();
    indexed.sort_by_key(|(index, tag)| {
        (
            metadata_tag_category_order(metadata_tag_category_id_for_display(
                tag,
                tag_display_categories,
            )),
            *index,
        )
    });
    tags.extend(indexed.into_iter().map(|(_index, tag)| tag));
}

pub(super) fn metadata_tag_category_id_for_display<'a>(
    tag: &str,
    tag_display_categories: &'a [MetadataTagDisplayCategory],
) -> &'a str {
    tag_display_categories
        .iter()
        .find(|entry| entry.tag == tag)
        .map(|entry| entry.category_id)
        .unwrap_or_else(|| inferred_metadata_tag_category_id_for_name(tag))
}

fn push_row_item(
    rows: &mut Vec<Vec<TagEntryRowItem>>,
    item: TagEntryRowItem,
    width: f32,
    content_width: f32,
) {
    if rows.is_empty() {
        rows.push(Vec::new());
    }

    let current_width = row_width(rows.last().expect("row exists"));
    let proposed = if current_width <= 0.0 {
        width
    } else {
        current_width + TAG_FIELD_ITEM_GAP + width
    };
    if proposed > content_width && current_width > 0.0 {
        rows.push(Vec::new());
    }
    rows.last_mut().expect("row exists").push(item);
}

fn row_width(row: &[TagEntryRowItem]) -> f32 {
    row.iter()
        .map(tag_entry_row_item_width)
        .reduce(|total, width| total + TAG_FIELD_ITEM_GAP + width)
        .unwrap_or(0.0)
}

fn tag_entry_row_item_width(item: &TagEntryRowItem) -> f32 {
    match item {
        TagEntryRowItem::Accepted(tag) => tag_pill_width(tag),
        TagEntryRowItem::PendingCategory(tag) => tag_pill_width(tag),
        TagEntryRowItem::Input(width) => *width,
    }
}

fn should_break_before_tag_input(tags: &[String], input_width: f32, content_width: f32) -> bool {
    let mut row_width = 0.0;
    for tag in tags {
        let width = tag_pill_width(tag);
        let proposed = if row_width <= 0.0 {
            width
        } else {
            row_width + TAG_FIELD_ITEM_GAP + width
        };
        if proposed > content_width && row_width > 0.0 {
            row_width = width;
        } else {
            row_width = proposed;
        }
    }

    row_width > 0.0
        && content_width - row_width - TAG_FIELD_ITEM_GAP
            < input_width.max(MIN_TAG_INPUT_REMAINING_WIDTH)
}

pub(super) fn rows_height(row_count: usize) -> f32 {
    if row_count == 0 {
        return 0.0;
    }
    row_count as f32 * TAG_FIELD_CONTROL_HEIGHT
        + row_count.saturating_sub(1) as f32 * TAG_FIELD_LINE_GAP
}

pub(super) fn tag_input_display_value(tag_draft: &str, completion_suffix: Option<&str>) -> String {
    completion_suffix
        .filter(|suffix| !suffix.is_empty())
        .map(|suffix| format!("{tag_draft}{suffix}"))
        .unwrap_or_else(|| tag_draft.to_string())
}

pub(super) fn tag_input_width(value: &str) -> f32 {
    let char_width = value.chars().count().max(7) as f32;
    (char_width * 7.0 + 12.0).clamp(61.0, 180.0)
}

pub(super) fn tag_input_width_for_placeholder(value: &str, placeholder: &str) -> f32 {
    let content_width = value.chars().count().max(placeholder.chars().count()) as f32;
    (content_width * 7.0 + 12.0).clamp(61.0, 180.0)
}

pub(super) fn tag_pill_width(tag: &str) -> f32 {
    (tag.chars().count() as f32 * 7.0 + 22.0).clamp(38.0, 180.0)
}
