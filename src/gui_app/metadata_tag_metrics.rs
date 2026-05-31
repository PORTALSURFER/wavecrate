use radiant::prelude as ui;

const METADATA_TAG_PILL_FONT_SIZE: f32 = 12.5;
const METADATA_TAG_PILL_PADDING_X: f32 = 11.0;
const METADATA_TAG_PILL_PADDING_Y: f32 = 0.0;
const METADATA_TAG_PILL_GAP: f32 = 3.0;
const METADATA_TAG_PILL_CLUSTER_GAP: f32 = 0.0;
const METADATA_TAG_PILL_MIN_HEIGHT: f32 = 18.0;
const METADATA_TAG_PILL_MIN_WIDTH: f32 = 38.0;
const METADATA_TAG_PILL_MAX_WIDTH: f32 = 180.0;

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
