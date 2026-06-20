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
    let tone = metadata_tag_category_tone(category_id);
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
    let tone = metadata_tag_category_tone(category_id);
    match state {
        MetadataTagSelectionState::None => ui::WidgetStyle::subtle(tone),
        MetadataTagSelectionState::Mixed => ui::WidgetStyle::normal(tone),
        MetadataTagSelectionState::All => ui::WidgetStyle::strong(tone),
    }
}

pub(in crate::native_app) fn metadata_tag_category_is_pinned(category_id: &str) -> bool {
    category_id == "playback-type"
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
            metadata_tag_pill_style("playback-type", true).prominence,
            ui::WidgetProminence::Strong
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
