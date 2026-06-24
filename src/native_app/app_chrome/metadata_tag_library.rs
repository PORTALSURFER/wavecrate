use crate::native_app::app::{GuiMessage, MetadataMessage, NativeAppState};
use radiant::prelude as ui;

mod identity;
mod projection;

use projection::{
    MetadataTagCategoryBodyProjection, MetadataTagCategoryProjection,
    MetadataTagEmptyCategoryProjection, MetadataTagLibraryProjection,
    MetadataTagPillGroupProjection, MetadataTagProjection,
};

const TAG_LIBRARY_PILL_HEIGHT: f32 = 18.0;
const TAG_LIBRARY_PILL_GAP: f32 = 3.0;

pub(in crate::native_app) fn panel(state: &NativeAppState) -> ui::View<GuiMessage> {
    panel_from_projection(MetadataTagLibraryProjection::from_state(state))
}

fn panel_from_projection(projection: MetadataTagLibraryProjection) -> ui::View<GuiMessage> {
    let groups = projection
        .categories
        .into_iter()
        .map(category_group)
        .collect::<Vec<_>>();
    ui::closeable_panel_section_from_parts(
        ui::PanelSectionParts::new(
            "Tag Editor",
            ui::scroll(ui::column(groups).spacing(3.0).fill_width())
                .fill_width()
                .fill_height(),
        )
        .title_height(24.0),
        GuiMessage::Metadata(MetadataMessage::ToggleMetadataTagLibrary),
    )
    .key(identity::PANEL_KEY)
    .width(220.0)
    .fill_height()
}

fn category_group(category: MetadataTagCategoryProjection) -> ui::View<GuiMessage> {
    let mut children = vec![category_header(&category)];
    if category.drop_target {
        children.push(
            ui::row(Vec::<ui::View<GuiMessage>>::new())
                .key(identity::category_drop_indicator_key(category.id))
                .style(ui::WidgetStyle::strong(ui::WidgetTone::Warning))
                .fill_width()
                .height(4.0),
        );
    }
    if let Some(body) = category_body(category.body) {
        children.push(body);
    }
    ui::column(children)
        .key(identity::category_group_key(category.id))
        .spacing(2.0)
        .fill_width()
}

fn category_body(body: MetadataTagCategoryBodyProjection) -> Option<ui::View<GuiMessage>> {
    match body {
        MetadataTagCategoryBodyProjection::Collapsed => None,
        MetadataTagCategoryBodyProjection::Empty(empty) => Some(empty_category_target(empty)),
        MetadataTagCategoryBodyProjection::Tags(group) => Some(category_tag_pills(group)),
    }
}

fn category_tag_pills(group: MetadataTagPillGroupProjection) -> ui::View<GuiMessage> {
    let category_id = group.category_id;
    let pills = group.tags.into_iter().map(tag_row);
    ui::wrap(pills, TAG_LIBRARY_PILL_GAP, TAG_LIBRARY_PILL_GAP)
        .key(identity::category_pills_key(category_id))
        .fill_width()
}

fn category_header(category: &MetadataTagCategoryProjection) -> ui::View<GuiMessage> {
    let category_for_input = category.id.to_string();
    let style = if category.drop_target {
        ui::WidgetStyle::strong(ui::WidgetTone::Warning)
    } else {
        ui::WidgetStyle::subtle(ui::WidgetTone::Neutral)
    };
    let visual = ui::row([
        ui::disclosure_button(category.expanded)
            .passive()
            .size(20.0, 18.0),
        ui::text_line(category.header_label.clone(), 22.0),
    ])
    .style(style)
    .padding_x(4.0)
    .spacing(4.0)
    .fill_width()
    .height(22.0);
    ui::interactive_row_underlay(visual)
        .tracked_drop_candidate(
            category.drop_tracking_active(),
            category.drop_target,
            category.drop_candidate,
            category.drop_target_active,
        )
        .style(style)
        .input_key(identity::category_input_key(category.id))
        .actions(
            ui::row_actions()
                .tracked_drop_candidate_key(
                    category_for_input.clone(),
                    drop_metadata_tag_on_category,
                    hover_metadata_tag_drop_category,
                    clear_metadata_tag_drop_category_unless,
                )
                .primary_key(category_for_input, toggle_metadata_tag_category),
        )
        .fill_width()
        .height(22.0)
}

fn open_metadata_tag_context_menu(tag: String, position: ui::Point) -> GuiMessage {
    GuiMessage::Metadata(MetadataMessage::OpenMetadataTagContextMenu { tag, position })
}

fn drag_metadata_tag(tag: String, drag: ui::DragHandleMessage) -> GuiMessage {
    GuiMessage::Metadata(MetadataMessage::DragMetadataTag { tag, drag })
}

fn drop_metadata_tag_on_category(category_id: String) -> GuiMessage {
    GuiMessage::Metadata(MetadataMessage::DropMetadataTagOnCategory { category_id })
}

fn hover_metadata_tag_drop_category(category_id: String, _: ui::Point) -> GuiMessage {
    GuiMessage::Metadata(MetadataMessage::HoverMetadataTagDropCategory { category_id })
}

fn clear_metadata_tag_drop_category_unless(category_id: String, _: ui::Point) -> GuiMessage {
    GuiMessage::Metadata(MetadataMessage::ClearMetadataTagDropCategoryUnless { category_id })
}

fn toggle_metadata_tag(tag: String) -> GuiMessage {
    GuiMessage::Metadata(MetadataMessage::ToggleMetadataTag(tag))
}

fn toggle_metadata_tag_category(category_id: String) -> GuiMessage {
    GuiMessage::Metadata(MetadataMessage::ToggleMetadataTagCategory(category_id))
}

fn tag_row(tag: MetadataTagProjection) -> ui::View<GuiMessage> {
    let tag_for_input = tag.label.clone();
    let category_for_input = tag.category_id.to_string();
    let mut badge = ui::interactive_badge(tag.label.clone())
        .style(tag.style)
        .active(tag.active);

    if tag.draggable {
        badge = badge.tracked_drag_source_with_motion(tag.drag_active, tag.drag_source);
    }
    if tag.drop_tracking_active() {
        badge = badge.tracked_drop_candidate(
            true,
            tag.drop_target,
            tag.drop_candidate,
            tag.drop_target_active,
        );
    }
    badge
        .actions(
            ui::row_actions()
                .secondary_key(tag_for_input.clone(), open_metadata_tag_context_menu)
                .drag_key(tag_for_input.clone(), drag_metadata_tag)
                .tracked_drop_candidate_key(
                    category_for_input,
                    drop_metadata_tag_on_category,
                    hover_metadata_tag_drop_category,
                    clear_metadata_tag_drop_category_unless,
                )
                .double_key(tag_for_input.clone(), toggle_metadata_tag)
                .primary_key(tag_for_input, toggle_metadata_tag),
        )
        .key(identity::tag_row_key(&tag.label))
        .width(tag.width)
        .height(TAG_LIBRARY_PILL_HEIGHT)
}

fn empty_category_target(category: MetadataTagEmptyCategoryProjection) -> ui::View<GuiMessage> {
    let category_for_input = category.category_id.to_string();
    let visual = ui::text_line("No tags yet", 20.0).padding(4.0);
    ui::interactive_row_underlay(visual)
        .tracked_drop_candidate(
            category.drop_tracking_active(),
            category.drop_target,
            category.drop_candidate,
            category.drop_target_active,
        )
        .input_key(identity::empty_category_input_key(category.category_id))
        .actions(ui::row_actions().tracked_drop_candidate_key(
            category_for_input,
            drop_metadata_tag_on_category,
            hover_metadata_tag_drop_category,
            clear_metadata_tag_drop_category_unless,
        ))
        .fill_width()
        .height(20.0)
}
