use super::{GuiAppState, GuiMessage};
use crate::gui_app::{
    audio_settings::top_status_bar,
    context_menu,
    folder_browser::{self, FileColumnDragFeedback},
    metadata_tag_metrics::metadata_tag_pill_width,
    metadata_tags::{MetadataTagCategoryGroup, metadata_tag_category_tone},
    sample_browser_view::sample_browser,
    status_bar,
    toolbar::main_toolbar,
    waveform_panel::waveform_panel,
};
use radiant::{gui::types::Point, prelude as ui};

const TAG_LIBRARY_PILL_HEIGHT: f32 = 18.0;
const TAG_LIBRARY_PILL_GAP: f32 = 3.0;
const CENTER_PANEL_PADDING: f32 = 6.0;
const FOLDER_SIDEBAR_PADDING: f32 = 4.0;
const METADATA_PANEL_PADDING: f32 = 6.0;
const BOTTOM_STATUS_BAR_HEIGHT: f32 = 30.0;
const FOLDER_SPLITTER_HIT_WIDTH: f32 = 5.0;
const FOLDER_SPLITTER_INSET: f32 = 1.0;

pub(super) fn view(state: &mut GuiAppState) -> ui::View<GuiMessage> {
    let tag_completion_overlay = metadata_tag_completion_overlay(state);
    let content = ui::column([
        top_status_bar(state),
        center_panel(state),
        status_bar::bottom_status_bar(state),
    ])
    .spacing(0.0)
    .fill();
    let mut layers = vec![content];
    if state.job_details_open
        && let Some(progress) = state.folder_progress.as_ref()
    {
        layers.push(status_bar::job_details_popover(progress));
    }
    if let Some(overlay) = tag_completion_overlay {
        layers.push(overlay);
    }
    if let Some(menu) = state.context_menu.as_ref() {
        layers.push(context_menu::overlay(menu));
    }
    if let Some(feedback) = state.folder_browser.file_column_drag_feedback() {
        layers.push(sample_column_drag_preview(&feedback));
    }
    ui::stack_layers(layers).fill()
}

fn sample_column_drag_preview(feedback: &FileColumnDragFeedback) -> ui::View<GuiMessage> {
    const OFFSET_X: f32 = 12.0;
    const OFFSET_Y: f32 = 14.0;
    let size = ui::Vector2::new(feedback.width.clamp(64.0, 180.0), 22.0);
    ui::floating_layer(
        Point::new(feedback.pointer.x + OFFSET_X, feedback.pointer.y + OFFSET_Y),
        size,
        ui::row([ui::text(feedback.label.clone())
            .align_text(ui::TextAlign::Left)
            .truncate()
            .fill_width()
            .height(20.0)])
        .padding_x(8.0)
        .style(ui::WidgetStyle::subtle(ui::WidgetTone::Accent))
        .fill(),
    )
    .key("sample-column-drag-preview")
}

fn center_panel(state: &mut GuiAppState) -> ui::View<GuiMessage> {
    let mut children = vec![folder_sidebar(state)];
    if state.metadata_tag_library_open && state.folder_browser.selected_file_id().is_some() {
        children.push(metadata_tag_library_panel(state));
    }
    children.push(folder_splitter());
    children.push(main_area(state));
    ui::column([
        ui::spacer().height(CENTER_PANEL_PADDING).fill_width(),
        ui::row(children).padding_x(CENTER_PANEL_PADDING).fill(),
    ])
    .spacing(0.0)
    .fill()
}

fn metadata_tag_completion_overlay(state: &GuiAppState) -> Option<ui::View<GuiMessage>> {
    if state.folder_browser.selected_file_id().is_none() {
        return None;
    }
    let completion_options = state.metadata_tag_completion_options();
    if completion_options.is_empty() {
        return None;
    }
    let pending_category_tag = state
        .pending_metadata_tag_category_tag()
        .map(str::to_string);
    let completion_suffix = state.metadata_tag_completion_suffix();
    let selected_metadata_tags = state.selected_metadata_tags().to_vec();
    let display_categories = state.selected_metadata_tag_display_categories();
    let tag_field_content_width = folder_browser::tag_field_content_width(state.folder_width);
    let tag_field_height = folder_browser::tag_field_height(
        state.metadata_tag_draft.as_str(),
        state.metadata_tag_tokens.as_slice(),
        pending_category_tag.as_deref(),
        completion_suffix.as_deref(),
        selected_metadata_tags.as_slice(),
        display_categories.as_slice(),
        tag_field_content_width,
    );
    let inset_x = CENTER_PANEL_PADDING + FOLDER_SIDEBAR_PADDING + METADATA_PANEL_PADDING;
    let inset_y = BOTTOM_STATUS_BAR_HEIGHT
        + CENTER_PANEL_PADDING
        + FOLDER_SIDEBAR_PADDING
        + METADATA_PANEL_PADDING
        + tag_field_height
        + folder_browser::TAG_COMPLETION_POPUP_GAP;
    Some(folder_browser::tag_completion_overlay(
        completion_options.as_slice(),
        tag_field_content_width,
        inset_x,
        inset_y,
    ))
}

fn folder_sidebar(state: &mut GuiAppState) -> ui::View<GuiMessage> {
    let folder_width = state.folder_width;
    let has_selected_file = state.folder_browser.selected_file_id().is_some();
    let pending_category_tag = state
        .pending_metadata_tag_category_tag()
        .map(str::to_string);
    let completion_suffix = state.metadata_tag_completion_suffix();
    let completion_options = state.metadata_tag_completion_options();
    let selected_metadata_tags = state.selected_metadata_tags().to_vec();
    let display_categories = state.selected_metadata_tag_display_categories();
    let selected_metadata_tag = state.selected_metadata_tag.clone();
    let input_placeholder = state.metadata_tag_input_placeholder();
    folder_browser::folder_browser_view_mut(
        &mut state.folder_browser,
        folder_width,
        has_selected_file,
        state.metadata_tag_draft.as_str(),
        state.metadata_tag_tokens.as_slice(),
        pending_category_tag.as_deref(),
        input_placeholder,
        completion_suffix.as_deref(),
        completion_options.as_slice(),
        selected_metadata_tags.as_slice(),
        display_categories.as_slice(),
        selected_metadata_tag.as_deref(),
    )
    .width(folder_width)
    .fill_height()
}

fn metadata_tag_library_panel(state: &GuiAppState) -> ui::View<GuiMessage> {
    let selected_tags = state.selected_metadata_tags();
    let drag_active = state.metadata_tag_drag_active();
    let drop_hover = state.metadata_tag_drop_hover();
    let dragged_tag = state.dragged_metadata_tag();
    let groups = state
        .categorized_metadata_tags()
        .into_iter()
        .map(|group| {
            metadata_tag_category_group(group, selected_tags, drag_active, drop_hover, dragged_tag)
        })
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

fn metadata_tag_category_group(
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
    let mut children = vec![metadata_tag_category_header(
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
            children.push(metadata_tag_empty_category_target(
                category_id.as_str(),
                locked,
                drag_active,
                category_hovered,
            ));
        } else {
            let pills = group.tags.into_iter().map(|tag| {
                let drag_source = dragged_tag == Some(tag.as_str());
                metadata_tag_library_row(
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

fn metadata_tag_category_header(
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
        .row(|row| row.tracked_drop_target(drag_active && !locked, drop_hover))
        .style(style)
        .actions(
            ui::InteractiveRowActions::new()
                .drop({
                    let category_id = category_for_input.clone();
                    move || GuiMessage::DropMetadataTagOnCategory {
                        category_id: category_id.clone(),
                    }
                })
                .hover_drop({
                    let category_id = category_for_input.clone();
                    move |_| GuiMessage::HoverMetadataTagDropCategory {
                        category_id: category_id.clone(),
                    }
                })
                .activate(move || {
                    GuiMessage::ToggleMetadataTagCategory(category_for_input.clone())
                }),
        )
        .key(format!("metadata-tag-category-{}", category_id))
        .fill_width()
        .height(22.0)
}

fn metadata_tag_library_row(
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
            .draggable()
            .drag_active(drag_active)
            .drag_source(drag_source)
            .drag_source_motion(true)
            .tracked_drop_target(drag_active, active_drop_target);
    }
    badge
        .actions(
            ui::InteractiveRowActions::new()
                .secondary({
                    let tag = tag_for_input.clone();
                    move |position| GuiMessage::OpenMetadataTagContextMenu {
                        tag: tag.clone(),
                        position,
                    }
                })
                .drag({
                    let tag = tag_for_input.clone();
                    move |drag| GuiMessage::DragMetadataTag {
                        tag: tag.clone(),
                        drag,
                    }
                })
                .drop({
                    let category_id = category_for_input.clone();
                    move || GuiMessage::DropMetadataTagOnCategory {
                        category_id: category_id.clone(),
                    }
                })
                .hover_drop({
                    let category_id = category_for_input;
                    move |_| GuiMessage::HoverMetadataTagDropCategory {
                        category_id: category_id.clone(),
                    }
                })
                .activate(move || GuiMessage::ToggleMetadataTag(tag_for_input.clone())),
        )
        .key(format!("metadata-tag-library-row-{tag}"))
        .width(width)
        .height(TAG_LIBRARY_PILL_HEIGHT)
}

fn metadata_tag_empty_category_target(
    category_id: &str,
    locked: bool,
    drag_active: bool,
    active_drop_target: bool,
) -> ui::View<GuiMessage> {
    let category_for_input = category_id.to_string();
    let visual = ui::text_line("No tags yet", 20.0).padding(4.0);
    ui::interactive_row_underlay(visual)
        .row(|row| row.tracked_drop_target(drag_active && !locked, active_drop_target))
        .actions(
            ui::InteractiveRowActions::new()
                .drop({
                    let category_id = category_for_input.clone();
                    move || GuiMessage::DropMetadataTagOnCategory {
                        category_id: category_id.clone(),
                    }
                })
                .hover_drop(move |_| GuiMessage::HoverMetadataTagDropCategory {
                    category_id: category_for_input.clone(),
                }),
        )
        .key(format!("metadata-tag-empty-category-{category_id}"))
        .fill_width()
        .height(20.0)
}

fn folder_splitter() -> ui::View<GuiMessage> {
    ui::drag_handle()
        .hover_chrome_only()
        .mapped(GuiMessage::ResizeFolder)
        .key("folder-browser-splitter-handle")
        .style(ui::WidgetStyle::subtle(ui::WidgetTone::Accent))
        .width(FOLDER_SPLITTER_HIT_WIDTH)
        .fill_height()
        .padding(FOLDER_SPLITTER_INSET)
}

fn main_area(state: &mut GuiAppState) -> ui::View<GuiMessage> {
    ui::column([
        main_toolbar(state),
        waveform_panel(state),
        sample_browser(state, state.folder_resize.is_some()),
    ])
    .padding(4.0)
    .fill()
}
