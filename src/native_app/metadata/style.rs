use radiant::prelude as ui;

use super::MetadataTagSelectionState;

/// Product styling for Wavecrate's fixed metadata tag categories.
pub(in crate::native_app) fn metadata_tag_category_tone(category_id: &str) -> ui::WidgetTone {
    match category_id {
        "playback-type" => ui::WidgetTone::Warning,
        "sound-type" => ui::WidgetTone::Accent,
        "character" => ui::WidgetTone::Success,
        "prefix" => ui::WidgetTone::Danger,
        "tuning-scale" => ui::WidgetTone::Neutral,
        _ => ui::WidgetTone::Neutral,
    }
}

pub(in crate::native_app) fn metadata_tag_pill_style(
    category_id: &str,
    active: bool,
) -> ui::WidgetStyle {
    let tone = metadata_tag_pill_tone(category_id, active);
    if active {
        ui::WidgetStyle::strong(tone)
    } else {
        ui::WidgetStyle::subtle(tone)
    }
}

pub(in crate::native_app) fn metadata_tag_pill_selection_style(
    category_id: &str,
    state: MetadataTagSelectionState,
) -> ui::WidgetStyle {
    match state {
        MetadataTagSelectionState::None => {
            ui::WidgetStyle::subtle(metadata_tag_pill_tone(category_id, false))
        }
        MetadataTagSelectionState::Mixed => {
            ui::WidgetStyle::normal(metadata_tag_pill_tone(category_id, true))
        }
        MetadataTagSelectionState::All => {
            ui::WidgetStyle::strong(metadata_tag_pill_tone(category_id, true))
        }
    }
}

pub(in crate::native_app) fn metadata_tag_category_is_pinned(category_id: &str) -> bool {
    category_id == "playback-type"
}

fn metadata_tag_pill_tone(category_id: &str, active: bool) -> ui::WidgetTone {
    if category_id == "playback-type" && !active {
        ui::WidgetTone::Neutral
    } else {
        metadata_tag_category_tone(category_id)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn metadata_tag_pill_style_keeps_fixed_vocabulary_visual_policy() {
        assert_eq!(
            metadata_tag_category_tone("sound-type"),
            ui::WidgetTone::Accent
        );
        assert_eq!(
            metadata_tag_category_tone("character"),
            ui::WidgetTone::Success
        );
        assert_eq!(
            metadata_tag_pill_style("playback-type", false).prominence,
            ui::WidgetProminence::Subtle
        );
        assert_eq!(
            metadata_tag_pill_style("playback-type", false).tone,
            ui::WidgetTone::Neutral
        );
        assert_eq!(
            metadata_tag_pill_style("playback-type", true).prominence,
            ui::WidgetProminence::Strong
        );
        assert_eq!(
            metadata_tag_pill_style("playback-type", true).tone,
            ui::WidgetTone::Warning
        );
        assert_eq!(
            metadata_tag_pill_selection_style("playback-type", MetadataTagSelectionState::None)
                .tone,
            ui::WidgetTone::Neutral
        );
        assert_eq!(
            metadata_tag_pill_selection_style("playback-type", MetadataTagSelectionState::All).tone,
            ui::WidgetTone::Warning
        );
        assert_eq!(
            metadata_tag_pill_style("character", false).prominence,
            ui::WidgetProminence::Subtle
        );
        assert_eq!(
            metadata_tag_pill_style("character", true).prominence,
            ui::WidgetProminence::Strong
        );
        assert_eq!(
            metadata_tag_pill_selection_style("character", MetadataTagSelectionState::None)
                .prominence,
            ui::WidgetProminence::Subtle
        );
        assert_eq!(
            metadata_tag_pill_selection_style("character", MetadataTagSelectionState::Mixed)
                .prominence,
            ui::WidgetProminence::Normal
        );
        assert_eq!(
            metadata_tag_pill_selection_style("character", MetadataTagSelectionState::All)
                .prominence,
            ui::WidgetProminence::Strong
        );
    }
}
