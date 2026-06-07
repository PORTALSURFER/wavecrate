use crate::native_app::app::{GuiMessage, NativeAppState};
use crate::native_app::app_chrome::library_browser::folder_sidebar::{
    self, FolderSidebarViewModel,
};
use crate::native_app::app_chrome::library_browser::sample_browser_view::{
    SampleBrowserViewModel, sample_browser,
};
use crate::native_app::app_chrome::status_bar;
use crate::native_app::app_chrome::toolbar::main_toolbar;
use crate::native_app::app_chrome::waveform_panel::waveform_panel;
use crate::native_app::audio::audio_settings::top_status_bar;
use crate::native_app::library_browser::context_menu as browser_context_menu;
use crate::native_app::library_browser::folder_browser::FileColumnDragFeedback;
use crate::native_app::transaction_history::TRANSACTION_LIST_MODAL_ID;
use crate::native_app::{
    app::FileMoveConflictResolution,
    metadata::metadata_tag_pill_width,
    metadata::{MetadataTagCategoryGroup, metadata_tag_category_tone},
};
use radiant::prelude as ui;

const TAG_LIBRARY_PILL_HEIGHT: f32 = 18.0;
const TAG_LIBRARY_PILL_GAP: f32 = 3.0;
const CENTER_PANEL_PADDING: f32 = 6.0;
const FOLDER_SIDEBAR_PADDING: f32 = 4.0;
const METADATA_PANEL_PADDING: f32 = 6.0;
const BOTTOM_STATUS_BAR_HEIGHT: f32 = 30.0;
const FOLDER_SPLITTER_HIT_WIDTH: f32 = 5.0;
const FOLDER_SPLITTER_INSET: f32 = 1.0;

pub(in crate::native_app) fn view(state: &mut NativeAppState) -> ui::View<GuiMessage> {
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
    if state.transaction_list_open {
        layers.push(transaction_list_modal(state));
    }
    if state
        .folder_browser
        .pending_file_move_conflict_view()
        .is_some()
    {
        layers.push(file_move_conflict_modal(state));
    }
    if let Some(overlay) = tag_completion_overlay {
        layers.push(overlay);
    }
    if let Some(menu) = state.context_menu.as_ref() {
        layers.push(browser_context_menu::overlay(menu));
    }
    if let Some(feedback) = state.folder_browser.file_column_drag_feedback() {
        layers.push(sample_column_drag_preview(&feedback));
    }
    ui::stack_layers(layers).fill()
}

fn sample_column_drag_preview(feedback: &FileColumnDragFeedback) -> ui::View<GuiMessage> {
    let size = ui::Vector2::new(feedback.width.clamp(64.0, 180.0), 22.0);
    ui::drag_preview_sized(feedback.label.clone(), feedback.pointer, size)
        .key("sample-column-drag-preview")
}

fn center_panel(state: &mut NativeAppState) -> ui::View<GuiMessage> {
    let mut children = vec![folder_sidebar_panel(state)];
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

fn metadata_tag_completion_overlay(state: &NativeAppState) -> Option<ui::View<GuiMessage>> {
    state.folder_browser.selected_file_id()?;
    let completion_options = state.metadata_tag_completion_options();
    if completion_options.is_empty() {
        return None;
    }
    let tag_field_content_width =
        folder_sidebar::tag_field_content_width(state.folder_panel.size());
    let inset_x = CENTER_PANEL_PADDING + FOLDER_SIDEBAR_PADDING + METADATA_PANEL_PADDING;
    let inset_y = BOTTOM_STATUS_BAR_HEIGHT
        + CENTER_PANEL_PADDING
        + FOLDER_SIDEBAR_PADDING
        + folder_sidebar::metadata_tag_completion_bottom_inset(
            state.folder_browser.metadata_panel_height(),
        )
        + folder_sidebar::TAG_COMPLETION_POPUP_GAP;
    Some(folder_sidebar::tag_completion_overlay(
        completion_options.as_slice(),
        tag_field_content_width,
        inset_x,
        inset_y,
    ))
}

fn folder_sidebar_panel(state: &mut NativeAppState) -> ui::View<GuiMessage> {
    folder_sidebar::folder_sidebar(FolderSidebarViewModel::from_app_state(state))
}

fn metadata_tag_library_panel(state: &NativeAppState) -> ui::View<GuiMessage> {
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

fn metadata_tag_empty_category_target(
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

fn main_area(state: &mut NativeAppState) -> ui::View<GuiMessage> {
    let toolbar = main_toolbar(state);
    let waveform = waveform_panel(state);
    let suppress_sample_hover = state.folder_panel.is_resizing();
    let sample_browser_model = SampleBrowserViewModel::from_app_state(state, suppress_sample_hover);
    ui::column([toolbar, waveform, sample_browser(sample_browser_model)])
        .padding(4.0)
        .fill()
}

fn transaction_list_modal(state: &NativeAppState) -> ui::View<GuiMessage> {
    let items = state.transaction_history.list_items();
    let summary = transaction_list_summary(state);
    let list = if items.is_empty() {
        ui::column([
            ui::text_line("No transactions registered", 24.0).fill_width(),
            ui::text_line("Undoable actions will appear here.", 22.0).fill_width(),
        ])
        .spacing(4.0)
        .fill_width()
    } else {
        ui::scroll(
            ui::column(
                items
                    .into_iter()
                    .map(transaction_list_row)
                    .collect::<Vec<_>>(),
            )
            .spacing(4.0)
            .fill_width(),
        )
        .fill_width()
        .fill_height()
    };
    let content = ui::column([summary, list])
        .spacing(6.0)
        .fill_width()
        .fill_height();

    ui::closeable_panel_section_layer_from_parts(
        ui::PanelSectionLayerParts::new(
            ui::PanelSectionParts::new("Transactions", content)
                .style(ui::WidgetStyle::strong(ui::WidgetTone::Neutral))
                .padding(8.0)
                .spacing(6.0)
                .title_height(24.0),
            ui::Vector2::new(420.0, 300.0),
        )
        .horizontal(ui::LayerHorizontalAnchor::Center)
        .vertical(ui::LayerVerticalAnchor::Center),
        GuiMessage::CloseTransactionList,
    )
    .key("transaction-list-modal")
    .id(TRANSACTION_LIST_MODAL_ID)
}

fn file_move_conflict_modal(state: &NativeAppState) -> ui::View<GuiMessage> {
    let conflict = state
        .folder_browser
        .pending_file_move_conflict_view()
        .expect("file move conflict modal requires pending conflict state");
    let summary = format!(
        "Conflict {} of {}",
        conflict.current_number, conflict.total_count
    );
    let content = ui::column([
        ui::text_line(summary, 22.0).fill_width(),
        ui::text_line(conflict.file_name, 24.0).fill_width(),
        ui::text_line(
            format!("Destination: {}", conflict.destination_folder),
            20.0,
        )
        .fill_width(),
        ui::row([
            ui::button("Overwrite")
                .danger()
                .message(GuiMessage::ResolveFileMoveConflict(
                    FileMoveConflictResolution::Overwrite,
                ))
                .width(92.0)
                .height(24.0),
            ui::button("Rename")
                .primary()
                .message(GuiMessage::ResolveFileMoveConflict(
                    FileMoveConflictResolution::Rename,
                ))
                .width(78.0)
                .height(24.0),
            ui::button("Skip")
                .message(GuiMessage::ResolveFileMoveConflict(
                    FileMoveConflictResolution::Skip,
                ))
                .width(64.0)
                .height(24.0),
        ])
        .spacing(6.0)
        .fill_width()
        .height(26.0),
    ])
    .spacing(6.0)
    .fill_width()
    .fill_height();

    ui::closeable_panel_section_layer_from_parts(
        ui::PanelSectionLayerParts::new(
            ui::PanelSectionParts::new("File Move Conflict", content)
                .style(ui::WidgetStyle::strong(ui::WidgetTone::Warning))
                .padding(8.0)
                .spacing(6.0)
                .title_height(24.0),
            ui::Vector2::new(430.0, 180.0),
        )
        .horizontal(ui::LayerHorizontalAnchor::Center)
        .vertical(ui::LayerVerticalAnchor::Center),
        GuiMessage::CancelFileMoveConflicts,
    )
    .key("file-move-conflict-modal")
}

fn transaction_list_summary(state: &NativeAppState) -> ui::View<GuiMessage> {
    let undo = if state.transaction_history.can_undo() {
        "undo ready"
    } else {
        "no undo"
    };
    let redo = if state.transaction_history.can_redo() {
        "redo ready"
    } else {
        "no redo"
    };
    let active = if state.transaction_history.is_transaction_open() {
        "open transaction"
    } else {
        "closed"
    };
    ui::text_line(format!("{undo} | {redo} | {active}"), 20.0)
        .key("transaction-list-summary")
        .fill_width()
}

fn transaction_list_row(
    item: crate::native_app::transaction_history::TransactionListItem,
) -> ui::View<GuiMessage> {
    let action_label = match item.action_count {
        1 => String::from("1 action"),
        count => format!("{count} actions"),
    };
    let action_summary = if item.action_labels.is_empty() {
        action_label
    } else {
        format!("{}: {}", action_label, item.action_labels.join(", "))
    };
    ui::row([
        ui::passive_badge(item.state.label().to_string())
            .style(transaction_list_state_style(item.state))
            .size(58.0, 20.0),
        ui::text_line(item.label, 22.0).fill_width(),
        ui::text_line(action_summary, 22.0).width(150.0),
    ])
    .key(format!("transaction-list-row-{}", item.id))
    .style(ui::WidgetStyle::subtle(ui::WidgetTone::Neutral))
    .padding_x(6.0)
    .spacing(6.0)
    .fill_width()
    .height(26.0)
}

fn transaction_list_state_style(
    state: crate::native_app::transaction_history::TransactionListState,
) -> ui::WidgetStyle {
    match state {
        crate::native_app::transaction_history::TransactionListState::Active => {
            ui::WidgetStyle::strong(ui::WidgetTone::Warning)
        }
        crate::native_app::transaction_history::TransactionListState::Undoable => {
            ui::WidgetStyle::strong(ui::WidgetTone::Accent)
        }
        crate::native_app::transaction_history::TransactionListState::Redoable => {
            ui::WidgetStyle::subtle(ui::WidgetTone::Neutral)
        }
    }
}
