use crate::native_app::{
    app::{GuiMessage, MetadataMessage, NativeAppState},
    metadata::metadata_tag_pill_width,
    metadata::{
        MetadataTagCategoryGroup, MetadataTagSelectionState, metadata_tag_pill_selection_style,
    },
};
use radiant::prelude as ui;

const TAG_LIBRARY_PILL_HEIGHT: f32 = 18.0;
const TAG_LIBRARY_PILL_GAP: f32 = 3.0;

struct TagRowContext<'a> {
    category_id: &'a str,
    locked: bool,
    selection_state: MetadataTagSelectionState,
    drag_active: bool,
    drag_source: bool,
    active_drop_target: bool,
}

pub(in crate::native_app) fn panel(state: &NativeAppState) -> ui::View<GuiMessage> {
    let drag_active = state.metadata_tag_drag_active();
    let drop_hover = state.metadata_tag_drop_hover();
    let dragged_tag = state.dragged_metadata_tag();
    let groups = state
        .categorized_metadata_tags()
        .into_iter()
        .map(|group| category_group(state, group, drag_active, drop_hover, dragged_tag))
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
    .key("metadata-tag-library-panel")
    .width(220.0)
    .fill_height()
}

fn category_group(
    state: &NativeAppState,
    group: MetadataTagCategoryGroup,
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
                let selection_state = state.metadata_tag_selection_state(&tag);
                tag_row(
                    tag,
                    TagRowContext {
                        category_id: category_id.as_str(),
                        locked,
                        selection_state,
                        drag_active,
                        drag_source,
                        active_drop_target: category_hovered,
                    },
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
            ui::row_actions()
                .drop_target_key(
                    category_for_input.clone(),
                    drop_metadata_tag_on_category,
                    hover_metadata_tag_drop_category,
                )
                .primary_key(category_for_input, toggle_metadata_tag_category),
        )
        .key(format!("metadata-tag-category-{}", category_id))
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

fn toggle_metadata_tag(tag: String) -> GuiMessage {
    GuiMessage::Metadata(MetadataMessage::ToggleMetadataTag(tag))
}

fn toggle_metadata_tag_category(category_id: String) -> GuiMessage {
    GuiMessage::Metadata(MetadataMessage::ToggleMetadataTagCategory(category_id))
}

fn tag_row(tag: String, context: TagRowContext<'_>) -> ui::View<GuiMessage> {
    let style = metadata_tag_pill_selection_style(context.category_id, context.selection_state);
    let width = metadata_tag_pill_width(&tag);
    let tag_for_input = tag.clone();
    let category_for_input = context.category_id.to_string();
    let mut badge = ui::interactive_badge(tag.clone())
        .style(style)
        .active(context.selection_state.is_all());

    if !context.locked {
        badge = badge
            .tracked_drag_source_with_motion(context.drag_active, context.drag_source)
            .tracked_drop_target(context.drag_active, context.active_drop_target);
    }
    badge
        .actions(
            ui::row_actions()
                .secondary_key(tag_for_input.clone(), open_metadata_tag_context_menu)
                .drag_key(tag_for_input.clone(), drag_metadata_tag)
                .drop_target_key(
                    category_for_input,
                    drop_metadata_tag_on_category,
                    hover_metadata_tag_drop_category,
                )
                .primary_key(tag_for_input, toggle_metadata_tag),
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
        .actions(ui::row_actions().drop_target_key(
            category_for_input,
            drop_metadata_tag_on_category,
            hover_metadata_tag_drop_category,
        ))
        .key(format!("metadata-tag-empty-category-{category_id}"))
        .fill_width()
        .height(20.0)
}
