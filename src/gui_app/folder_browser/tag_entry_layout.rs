use crate::gui_app::metadata_tag_metrics::{
    metadata_tag_input_width_for_char_count, metadata_tag_pill_width,
};
use crate::gui_app::metadata_tags::{
    MetadataTagDisplayCategory, inferred_metadata_tag_category_id_for_name,
    metadata_tag_category_order,
};
use radiant::prelude as ui;

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

impl ui::FlowItemWidth for TagEntryRowItem {
    fn flow_width(&self) -> f32 {
        tag_entry_row_item_width(self)
    }
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

    if let Some(tag) = pending_category_tag {
        return tag_field_rows_with_pending_category(
            &visible_tags,
            tag,
            input_width,
            content_width,
        );
    }

    ui::pack_flow_rows_with_trailing_item(
        tag_entry_flow_items(&visible_tags),
        ui::FlowTrailingItemParts::new(
            TagEntryRowItem::Input,
            input_width,
            content_width,
            MIN_TAG_INPUT_REMAINING_WIDTH,
        ),
        content_width,
        tag_field_flow_metrics(),
    )
}

fn tag_field_rows_with_pending_category(
    tags: &[String],
    pending_category_tag: &str,
    input_width: f32,
    content_width: f32,
) -> Vec<Vec<TagEntryRowItem>> {
    let mut rows = ui::pack_flow_rows(
        tag_entry_flow_items(tags),
        content_width,
        tag_field_flow_metrics(),
    );
    if ui::flow_trailing_item_starts_new_row(
        tags.iter().map(|tag| tag_pill_width(tag)),
        input_width,
        MIN_TAG_INPUT_REMAINING_WIDTH,
        content_width,
        tag_field_flow_metrics(),
    ) || rows.is_empty()
    {
        rows.push(Vec::new());
    }

    let label = format!("{pending_category_tag} ->");
    ui::push_flow_row_item(
        &mut rows,
        TagEntryRowItem::PendingCategory(label.clone()),
        tag_pill_width(&label),
        content_width,
        tag_field_flow_metrics(),
    );
    let input_width = if rows.last().is_some_and(Vec::is_empty) {
        content_width
    } else {
        input_width
    };
    ui::push_flow_row_item(
        &mut rows,
        TagEntryRowItem::Input(input_width),
        input_width,
        content_width,
        tag_field_flow_metrics(),
    );
    rows
}

fn tag_entry_flow_items(tags: &[String]) -> Vec<ui::FlowItem<TagEntryRowItem>> {
    tags.iter()
        .map(|tag| ui::FlowItem::new(TagEntryRowItem::Accepted(tag.clone()), tag_pill_width(tag)))
        .collect()
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

fn tag_entry_row_item_width(item: &TagEntryRowItem) -> f32 {
    match item {
        TagEntryRowItem::Accepted(tag) => tag_pill_width(tag),
        TagEntryRowItem::PendingCategory(tag) => tag_pill_width(tag),
        TagEntryRowItem::Input(width) => *width,
    }
}

pub(super) fn rows_height(row_count: usize) -> f32 {
    ui::flow_rows_height(row_count, tag_field_flow_metrics())
}

fn tag_field_flow_metrics() -> ui::FlowLayoutMetrics {
    ui::FlowLayoutMetrics::new(
        TAG_FIELD_ITEM_GAP,
        TAG_FIELD_LINE_GAP,
        TAG_FIELD_CONTROL_HEIGHT,
    )
}

pub(super) fn tag_input_display_value(tag_draft: &str, completion_suffix: Option<&str>) -> String {
    completion_suffix
        .filter(|suffix| !suffix.is_empty())
        .map(|suffix| format!("{tag_draft}{suffix}"))
        .unwrap_or_else(|| tag_draft.to_string())
}

pub(super) fn tag_input_width(value: &str) -> f32 {
    metadata_tag_input_width_for_char_count(value.chars().count().max(7), 61.0, 180.0)
}

pub(super) fn tag_input_width_for_placeholder(value: &str, placeholder: &str) -> f32 {
    metadata_tag_input_width_for_char_count(
        value.chars().count().max(placeholder.chars().count()),
        61.0,
        180.0,
    )
}

pub(super) fn tag_pill_width(tag: &str) -> f32 {
    metadata_tag_pill_width(tag)
}
