use radiant::prelude as ui;

use crate::native_app::app::{GuiMessage, MetadataMessage};
use crate::native_app::metadata::{
    MetadataTagSelectionState, metadata_tag_pill_selection_style, metadata_tag_pill_style,
};

use super::identity;
use super::projection::{
    TagEntryItemProjection, TagEntryRowProjection, TagInputProjection, TagLibraryToggleProjection,
    TagTokenProjection,
};
use crate::native_app::app_chrome::library_browser::library_sidebar::tag_entry_layout::{
    TAG_FIELD_CONTROL_HEIGHT, TAG_FIELD_ITEM_GAP, accepted_tag_pill_width, tag_pill_width,
};

const TAG_REMOVE_BUTTON_WIDTH: f32 = 18.0;
const TAG_REMOVE_BUTTON_INSET: f32 = 1.0;

pub(super) fn tag_entry_row(row: TagEntryRowProjection, row_index: usize) -> ui::View<GuiMessage> {
    ui::row(
        row.items
            .into_iter()
            .map(|item| match item {
                TagEntryItemProjection::Accepted(tag) => accepted_tag_token(tag),
                TagEntryItemProjection::PendingCategory(tag) => {
                    pending_category_tag_token(tag.label.as_str())
                }
                TagEntryItemProjection::Input(input) => tag_text_input(input),
                TagEntryItemProjection::LibraryToggle(toggle) => {
                    metadata_tag_library_toggle(toggle)
                }
            })
            .collect::<Vec<_>>(),
    )
    .key(identity::tag_row_key(row_index))
    .height(TAG_FIELD_CONTROL_HEIGHT)
    .fill_width()
    .spacing(TAG_FIELD_ITEM_GAP)
}

fn tag_text_input(projection: TagInputProjection) -> ui::View<GuiMessage> {
    let mut input = ui::text_input(projection.draft)
        .placeholder(projection.placeholder.to_ascii_uppercase())
        .underline();

    if let Some(suffix) = projection
        .completion_suffix
        .filter(|suffix| !suffix.is_empty())
    {
        input = input.completion_suffix(suffix);
    }

    input
        .message_event(|message| GuiMessage::Metadata(MetadataMessage::MetadataTagInput(message)))
        .id(identity::metadata_tag_input_id())
        .size(projection.width, TAG_FIELD_CONTROL_HEIGHT)
}

fn accepted_tag_token(tag: TagTokenProjection) -> ui::View<GuiMessage> {
    let style = if tag.active {
        metadata_tag_pill_style(tag.category_id.as_str(), true)
    } else if tag.mixed {
        metadata_tag_pill_selection_style(
            tag.category_id.as_str(),
            MetadataTagSelectionState::Mixed,
        )
    } else {
        metadata_tag_pill_style(tag.category_id.as_str(), false)
    };
    let style = playback_type_tag_style(tag.label.as_str(), style);
    let display_label = format!("{} ×", tag.label.to_ascii_uppercase());
    let tag_for_input = tag.label.clone();
    let tag_width = accepted_tag_pill_width(&tag.label);
    let badge = ui::interactive_badge(display_label)
        .style(style)
        .active(tag.active)
        .outline()
        .actions(ui::row_actions().primary_secondary_key(
            tag_for_input,
            |tag| GuiMessage::Metadata(MetadataMessage::SelectMetadataTag(tag)),
            |tag, position| {
                GuiMessage::Metadata(MetadataMessage::OpenMetadataTagContextMenu { tag, position })
            },
        ))
        .key(identity::accepted_tag_key(&tag.label))
        .size(tag_width, TAG_FIELD_CONTROL_HEIGHT);
    let remove = ui::button("×")
        .style(style)
        .hover_chrome_only()
        .message(GuiMessage::Metadata(MetadataMessage::RemoveMetadataTag(
            tag.label.clone(),
        )))
        .key(identity::accepted_tag_remove_key(&tag.label));
    let remove_layer = ui::anchored_layer(
        remove,
        ui::Vector2::new(TAG_REMOVE_BUTTON_WIDTH, TAG_FIELD_CONTROL_HEIGHT),
        ui::LayerHorizontalAnchor::End,
        ui::LayerVerticalAnchor::Center,
        TAG_REMOVE_BUTTON_INSET,
        0.0,
    );

    ui::stack([badge, remove_layer]).size(tag_width, TAG_FIELD_CONTROL_HEIGHT)
}

fn playback_type_tag_style(label: &str, mut style: ui::WidgetStyle) -> ui::WidgetStyle {
    if label.eq_ignore_ascii_case("one-shot") || label.eq_ignore_ascii_case("oneshot") {
        style.tone = ui::WidgetTone::Warning;
    } else if label.eq_ignore_ascii_case("loop") {
        style.tone = ui::WidgetTone::Accent;
    }
    style
}

fn pending_category_tag_token(tag: &str) -> ui::View<GuiMessage> {
    ui::badge(tag.to_ascii_uppercase())
        .subtle()
        .outline()
        .passive()
        .key(identity::pending_category_tag_key(tag))
        .style(ui::WidgetStyle::subtle(ui::WidgetTone::Accent))
        .size(tag_pill_width(tag), TAG_FIELD_CONTROL_HEIGHT)
}

fn metadata_tag_library_toggle(projection: TagLibraryToggleProjection) -> ui::View<GuiMessage> {
    let tooltip = if projection.open {
        "Hide tag library"
    } else {
        "Show tag library"
    };
    let toggle = ui::disclosure_button(projection.open)
        .message(GuiMessage::Metadata(
            MetadataMessage::ToggleMetadataTagLibrary,
        ))
        .key(identity::TAG_LIBRARY_TOGGLE_KEY)
        .size(projection.width, TAG_FIELD_CONTROL_HEIGHT)
        .tooltip_if(projection.help_tooltips_enabled, tooltip);
    #[cfg(test)]
    {
        toggle.id(identity::metadata_tag_library_toggle_id())
    }
    #[cfg(not(test))]
    {
        toggle
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use radiant::prelude::IntoView;
    use radiant::runtime::{DeclarativeOwnedRuntimeBridge, SurfaceRuntime};

    #[test]
    fn accepted_tag_remove_affordance_lights_on_hover_and_removes_on_click() {
        let tag = TagTokenProjection {
            label: String::from("kick"),
            category_id: String::from("sound-type"),
            mixed: false,
            active: true,
        };
        let width = accepted_tag_pill_width(&tag.label);
        let bridge = DeclarativeOwnedRuntimeBridge::new(
            Vec::<GuiMessage>::new(),
            move |_| accepted_tag_token(tag.clone()).into_surface(),
            |messages: &mut Vec<GuiMessage>, message| messages.push(message),
        );
        let mut runtime =
            SurfaceRuntime::new(bridge, ui::Vector2::new(width, TAG_FIELD_CONTROL_HEIGHT));
        let remove_position = ui::Point::new(width - 8.0, TAG_FIELD_CONTROL_HEIGHT * 0.5);

        let idle = runtime.frame_with_default_theme();
        assert!(idle.paint_plan.contains_text("KICK ×"));
        assert!(!idle.paint_plan.contains_text("×"));

        runtime.dispatch_input_at(
            remove_position,
            ui::WidgetInput::pointer_move(remove_position),
        );
        assert!(
            runtime
                .frame_with_default_theme()
                .paint_plan
                .contains_text("×")
        );

        runtime.dispatch_input_at(
            remove_position,
            ui::WidgetInput::primary_press(remove_position),
        );
        runtime.dispatch_input_at(
            remove_position,
            ui::WidgetInput::primary_release(remove_position),
        );

        assert_eq!(
            runtime.bridge().state(),
            &[GuiMessage::Metadata(MetadataMessage::RemoveMetadataTag(
                String::from("kick")
            ))]
        );
    }

    #[test]
    fn accepted_tag_label_keeps_selection_behavior_away_from_remove_button() {
        let tag = TagTokenProjection {
            label: String::from("kick"),
            category_id: String::from("sound-type"),
            mixed: false,
            active: true,
        };
        let width = accepted_tag_pill_width(&tag.label);
        let bridge = DeclarativeOwnedRuntimeBridge::new(
            Vec::<GuiMessage>::new(),
            move |_| accepted_tag_token(tag.clone()).into_surface(),
            |messages: &mut Vec<GuiMessage>, message| messages.push(message),
        );
        let mut runtime =
            SurfaceRuntime::new(bridge, ui::Vector2::new(width, TAG_FIELD_CONTROL_HEIGHT));
        let label_position = ui::Point::new(8.0, TAG_FIELD_CONTROL_HEIGHT * 0.5);

        runtime.dispatch_input_at(
            label_position,
            ui::WidgetInput::primary_press(label_position),
        );
        runtime.dispatch_input_at(
            label_position,
            ui::WidgetInput::primary_release(label_position),
        );

        assert_eq!(
            runtime.bridge().state(),
            &[GuiMessage::Metadata(MetadataMessage::SelectMetadataTag(
                String::from("kick")
            ))]
        );
    }
}
