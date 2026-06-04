use radiant::prelude as ui;

const METADATA_TAG_PILL_FONT_SIZE: f32 = 12.5;
const METADATA_TAG_PILL_PADDING_X: f32 = 11.0;
const METADATA_TAG_PILL_PADDING_Y: f32 = 0.0;
const METADATA_TAG_PILL_GAP: f32 = 3.0;
const METADATA_TAG_PILL_CLUSTER_GAP: f32 = 0.0;
const METADATA_TAG_PILL_MIN_HEIGHT: f32 = 18.0;
const METADATA_TAG_PILL_MIN_WIDTH: f32 = 38.0;
const METADATA_TAG_PILL_MAX_WIDTH: f32 = 180.0;
const METADATA_TAG_INPUT_CHARACTER_ADVANCE: f32 = 7.0;
const METADATA_TAG_INPUT_PADDING: f32 = 12.0;

pub(super) fn metadata_tag_pill_width(tag: &str) -> f32 {
    ui::inline_badge_width_in_range(
        tag,
        metadata_tag_pill_metrics(),
        METADATA_TAG_PILL_MIN_WIDTH,
        METADATA_TAG_PILL_MAX_WIDTH,
    )
}

fn metadata_tag_pill_metrics() -> ui::InlineBadgeMetrics {
    ui::InlineBadgeMetrics::new(
        METADATA_TAG_PILL_FONT_SIZE,
        METADATA_TAG_PILL_PADDING_X,
        METADATA_TAG_PILL_PADDING_Y,
        METADATA_TAG_PILL_GAP,
        METADATA_TAG_PILL_CLUSTER_GAP,
        METADATA_TAG_PILL_MIN_HEIGHT,
    )
}

pub(super) fn metadata_tag_input_width_for_char_count(
    char_count: usize,
    min_width: f32,
    max_width: f32,
) -> f32 {
    ui::estimated_text_width_for_char_count_in_range(
        char_count,
        metadata_tag_input_width_estimate(),
        min_width,
        max_width,
    )
}

pub(super) fn metadata_tag_input_width_for_segments<'a>(
    segments: impl IntoIterator<Item = &'a str>,
    min_width: f32,
    max_width: f32,
) -> f32 {
    ui::estimated_text_width_for_segments_in_range(
        segments,
        metadata_tag_input_width_estimate(),
        min_width,
        max_width,
    )
}

fn metadata_tag_input_width_estimate() -> ui::TextWidthEstimate {
    ui::TextWidthEstimate::new(
        METADATA_TAG_INPUT_CHARACTER_ADVANCE,
        METADATA_TAG_INPUT_PADDING,
    )
}
