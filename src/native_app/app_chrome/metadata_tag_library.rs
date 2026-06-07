use crate::native_app::{
    app::{GuiMessage, NativeAppState},
    metadata::metadata_tag_pill_width,
    metadata::{MetadataTagCategoryGroup, metadata_tag_category_tone},
};
use radiant::prelude as ui;

const TAG_LIBRARY_PILL_HEIGHT: f32 = 18.0;
const TAG_LIBRARY_PILL_GAP: f32 = 3.0;

pub(in crate::native_app) fn panel(state: &NativeAppState) -> ui::View<GuiMessage> {
    let selected_tags = state.selected_metadata_tags();
    let drag_active = state.metadata_tag_drag_active();
    let drop_hover = state.metadata_tag_drop_hover();
    let dragged_tag = state.dragged_metadata_tag();
    let groups = state
        .categorized_metadata_tags()
        .into_iter()
        .map(|group| category_group(group, selected_tags, drag_active, drop_hover, dragged_tag))
        .collect::<Vec<_>>();
    ui::panel_section_from_parts(
        ui::PanelSectionParts::new(
            "Tag Editor",
            ui::scroll(ui::column(groups).spacing(3.0).fill_width())
                .fill_width()
                .fill_height(),
        )
        .trailing(
            ui::close_button()
                .message(GuiMessage::ToggleMetadataTagLibrary)
                .key("metadata-tag-library-close")
                .size(22.0, 20.0),
        )
        .title_height(24.0),
    )
    .key("metadata-tag-library-panel")
    .width(220.0)
    .fill_height()
}

fn category_group(
    group: MetadataTagCategoryGroup,
    selected_tags: &[String],
    drag_active: bool,
    drop_hover: Option<&str>,
    dragged_tag: Option<&str>,
) -> ui::View<GuiMessage> {
    let count_label = if group.tags.is_empty() {
        String::new()
    } else {
        format!(" ({})", group.tags.len())
    };
    let locked = group.locked;
    let category_id = group.id.to_string();
    let category_hovered = drop_hover == Some(group.id);
    let mut children = vec![category_header(
        category_id.clone(),
        group.collapsed,
        format!(
            "{}{count_label}{}",
            group.label,
            if locked { " [locked]" } else { "" }
        ),
        locked,
        drag_active,
        category_hovered,
    )];
    if category_hovered {
        children.push(
            ui::row(Vec::<ui::View<GuiMessage>>::new())
                .key(format!("metadata-tag-category-drop-indicator-{}", group.id))
                .style(ui::WidgetStyle::strong(ui::WidgetTone::Warning))
                .fill_width()
                .height(4.0),
        );
    }
    if !group.collapsed {
        if group.tags.is_empty() {
            children.push(empty_category_target(
                category_id.as_str(),
                locked,
                drag_active,
                category_hovered,
            ));
        } else {
            let pills = group.tags.into_iter().map(|tag| {
                let drag_source = dragged_tag == Some(tag.as_str());
                tag_row(
                    tag,
                    category_id.as_str(),
                    locked,
                    selected_tags,
                    drag_active,
                    drag_source,
                    category_hovered,
                )
            });
            children.push(
                ui::wrap(pills, TAG_LIBRARY_PILL_GAP, TAG_LIBRARY_PILL_GAP)
                    .key(format!("metadata-tag-category-pills-{}", group.id))
                    .fill_width(),
            );
        }
    }
    ui::column(children)
        .key(format!("metadata-tag-category-group-{}", group.id))
        .spacing(2.0)
        .fill_width()
}

fn category_header(
    category_id: String,
    collapsed: bool,
    label: String,
    locked: bool,
    drag_active: bool,
    drop_hover: bool,
) -> ui::View<GuiMessage> {
    let category_for_input = category_id.clone();
    let style = if drop_hover {
        ui::WidgetStyle::strong(ui::WidgetTone::Warning)
    } else {
        ui::WidgetStyle::subtle(ui::WidgetTone::Neutral)
    };
    let visual = ui::row([
        ui::disclosure_button(!collapsed)
            .passive()
            .key(format!("metadata-tag-category-disclosure-{category_id}"))
            .size(20.0, 18.0),
        ui::text_line(label, 22.0).key(format!("metadata-tag-category-label-{category_id}")),
    ])
    .style(style)
    .padding_x(4.0)
    .spacing(4.0)
    .fill_width()
    .height(22.0);
    ui::interactive_row_underlay(visual)
        .tracked_drop_target(drag_active && !locked, drop_hover)
        .style(style)
        .actions(
            ui::InteractiveRowActions::new()
                .drop_target_key(
                    category_for_input.clone(),
                    |category_id| GuiMessage::DropMetadataTagOnCategory { category_id },
                    |category_id, _| GuiMessage::HoverMetadataTagDropCategory { category_id },
                )
                .activate_key(category_for_input, GuiMessage::ToggleMetadataTagCategory),
        )
        .key(format!("metadata-tag-category-{}", category_id))
        .fill_width()
        .height(22.0)
}

fn tag_row(
    tag: String,
    category_id: &str,
    locked: bool,
    selected_tags: &[String],
    drag_active: bool,
    drag_source: bool,
    active_drop_target: bool,
) -> ui::View<GuiMessage> {
    let selected = selected_tags.iter().any(|selected| selected == &tag);
    let tone = metadata_tag_category_tone(category_id);
    let style = if selected || locked {
        ui::WidgetStyle::strong(tone)
    } else {
        ui::WidgetStyle::subtle(tone)
    };
    let width = metadata_tag_pill_width(&tag);
    let tag_for_input = tag.clone();
    let category_for_input = category_id.to_string();
    let mut badge = ui::interactive_badge(tag.clone())
        .style(style)
        .active(selected || locked);
    if !selected && !locked {
        badge = badge.subtle();
    }

    if !locked {
        badge = badge
            .tracked_drag_source_with_motion(drag_active, drag_source)
            .tracked_drop_target(drag_active, active_drop_target);
    }
    badge
        .actions(
            ui::InteractiveRowActions::new()
                .secondary_key(tag_for_input.clone(), |tag, position| {
                    GuiMessage::OpenMetadataTagContextMenu { tag, position }
                })
                .drag_key(tag_for_input.clone(), |tag, drag| {
                    GuiMessage::DragMetadataTag { tag, drag }
                })
                .drop_target_key(
                    category_for_input,
                    |category_id| GuiMessage::DropMetadataTagOnCategory { category_id },
                    |category_id, _| GuiMessage::HoverMetadataTagDropCategory { category_id },
                )
                .activate_key(tag_for_input, GuiMessage::ToggleMetadataTag),
        )
        .key(format!("metadata-tag-library-row-{tag}"))
        .width(width)
        .height(TAG_LIBRARY_PILL_HEIGHT)
}

fn empty_category_target(
    category_id: &str,
    locked: bool,
    drag_active: bool,
    active_drop_target: bool,
) -> ui::View<GuiMessage> {
    let category_for_input = category_id.to_string();
    let visual = ui::text_line("No tags yet", 20.0).padding(4.0);
    ui::interactive_row_underlay(visual)
        .tracked_drop_target(drag_active && !locked, active_drop_target)
        .actions(ui::InteractiveRowActions::new().drop_target_key(
            category_for_input,
            |category_id| GuiMessage::DropMetadataTagOnCategory { category_id },
            |category_id, _| GuiMessage::HoverMetadataTagDropCategory { category_id },
        ))
        .key(format!("metadata-tag-empty-category-{category_id}"))
        .fill_width()
        .height(20.0)
}
