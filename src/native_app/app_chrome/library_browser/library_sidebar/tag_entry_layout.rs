use crate::native_app::metadata::{
    MetadataTagDisplayCategory, inferred_metadata_tag_category_id_for_name,
    metadata_tag_category_order,
};
use crate::native_app::metadata::{metadata_tag_input_width_policy, metadata_tag_pill_width};
use radiant::prelude as ui;

pub(super) const TAG_FIELD_CONTROL_HEIGHT: f32 = 18.0;
pub(super) const TAG_FIELD_ITEM_GAP: f32 = 3.0;
pub(super) const TAG_FIELD_LINE_GAP: f32 = 3.0;
const TAG_FIELD_HORIZONTAL_CHROME: f32 = 26.0;
const TAG_FIELD_VERTICAL_CHROME: f32 = 6.0;
const TAG_FIELD_MIN_CONTENT_WIDTH: f32 = 120.0;
const MAX_TAG_FIELD_ROWS: usize = 6;
const MIN_TAG_INPUT_REMAINING_WIDTH: f32 = 180.0;
const TAG_INPUT_MIN_WIDTH: f32 = 61.0;
const TAG_INPUT_MAX_WIDTH: f32 = 180.0;
const TAG_INPUT_MIN_VISIBLE_CHARS: usize = 7;

#[derive(Clone, Debug, PartialEq)]
pub(super) enum TagEntryRowItem {
    Accepted(String),
    PendingCategory(String),
    Input(f32),
}

#[derive(Clone, Debug, PartialEq)]
pub(super) struct TagEntryFieldProjection {
    pub(super) rows: Vec<Vec<TagEntryRowItem>>,
    pub(super) layout: ui::FlowFieldLayout,
}

impl TagEntryFieldProjection {
    pub(super) fn new(
        tag_draft: &str,
        tag_tokens: &[String],
        pending_category_tag: Option<&str>,
        input_placeholder: &str,
        completion_suffix: Option<&str>,
        tags: &[String],
        tag_display_categories: &[MetadataTagDisplayCategory],
        content_width: f32,
    ) -> Self {
        let visible_tags = visible_metadata_tags(tags, tag_tokens, tag_display_categories);
        let input_width = if pending_category_tag.is_some() {
            tag_input_width_with_completion(tag_draft, completion_suffix)
        } else {
            tag_input_width_with_completion_or_placeholder(
                tag_draft,
                completion_suffix,
                input_placeholder,
            )
        };
        let rows = tag_field_rows(
            &visible_tags,
            tag_display_categories,
            pending_category_tag,
            input_width,
            content_width,
        );
        let layout = tag_field_layout(rows.len(), content_width);
        Self { rows, layout }
    }
}

impl ui::FlowItemWidth for TagEntryRowItem {
    fn flow_width(&self) -> f32 {
        tag_entry_row_item_width(self)
    }
}

fn visible_metadata_tags(
    tags: &[String],
    tag_tokens: &[String],
    tag_display_categories: &[MetadataTagDisplayCategory],
) -> Vec<String> {
    let mut visible_tags = tags.to_vec();
    for token in tag_tokens {
        if !visible_tags.iter().any(|tag| tag == token) {
            visible_tags.push(token.clone());
        }
    }
    order_metadata_tags_for_display(&mut visible_tags, tag_display_categories);
    visible_tags
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
    let label = format!("{pending_category_tag} ->");
    ui::pack_flow_rows_with_trailing_group(
        tag_entry_flow_items(tags),
        [
            ui::FlowItem::new(
                TagEntryRowItem::PendingCategory(label.clone()),
                tag_pill_width(&label),
            ),
            ui::FlowItem::new(TagEntryRowItem::Input(input_width), input_width),
        ],
        content_width,
        tag_field_flow_metrics(),
    )
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

pub(super) fn tag_field_layout(row_count: usize, content_width: f32) -> ui::FlowFieldLayout {
    tag_field_metrics().layout_for_content_width(content_width, row_count)
}

pub(in crate::native_app) fn tag_field_content_width(sidebar_width: f32) -> f32 {
    tag_field_metrics().content_width(sidebar_width)
}

fn tag_field_flow_metrics() -> ui::FlowLayoutMetrics {
    ui::FlowLayoutMetrics::new(
        TAG_FIELD_ITEM_GAP,
        TAG_FIELD_LINE_GAP,
        TAG_FIELD_CONTROL_HEIGHT,
    )
}

fn tag_field_metrics() -> ui::FlowFieldMetrics {
    ui::FlowFieldMetrics::new(
        tag_field_flow_metrics(),
        TAG_FIELD_HORIZONTAL_CHROME,
        TAG_FIELD_VERTICAL_CHROME,
        TAG_FIELD_MIN_CONTENT_WIDTH,
        MAX_TAG_FIELD_ROWS,
    )
}

pub(super) fn tag_input_width_with_completion(
    tag_draft: &str,
    completion_suffix: Option<&str>,
) -> f32 {
    tag_input_width_policy().width_for_value_and_completion_suffix(tag_draft, completion_suffix)
}

pub(super) fn tag_input_width_with_completion_or_placeholder(
    tag_draft: &str,
    completion_suffix: Option<&str>,
    placeholder: &str,
) -> f32 {
    tag_input_width_policy().width_for_value_completion_or_placeholder(
        tag_draft,
        completion_suffix,
        placeholder,
    )
}

fn tag_input_width_policy() -> ui::TextInputWidthPolicy {
    metadata_tag_input_width_policy(TAG_INPUT_MIN_WIDTH, TAG_INPUT_MAX_WIDTH)
        .with_min_visible_chars(TAG_INPUT_MIN_VISIBLE_CHARS)
}

pub(super) fn tag_pill_width(tag: &str) -> f32 {
    metadata_tag_pill_width(tag)
}

#[cfg(test)]
mod tests {
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

        let projection = TagEntryFieldProjection::new(
            "",
            &tokens,
            None,
            "add tag",
            None,
            &tags,
            &categories,
            420.0,
        );

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
}
