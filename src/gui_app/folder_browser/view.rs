use radiant::{
    prelude as ui,
    widgets::{ButtonMessage, WidgetStyle, WidgetTone},
};

use crate::gui_app::metadata_tags::{MetadataTagCompletionOption, MetadataTagDisplayCategory};

use super::tag_editor::{metadata_section, tag_field_content_width, tag_field_height};
use super::{
    CollectionHitMessage, CollectionHitTarget, FolderBrowserMessage, FolderBrowserState,
    GuiMessage, SourceEntry, TREE_DEPTH_INDENT, TREE_ROW_HEIGHT, VisibleFolder,
    collections::COLLECTION_ROW_HEIGHT,
    plural,
    tree_hit_target::{FolderTreeHitMessage, FolderTreeHitTarget},
    tree_widgets::FolderDropClearTarget,
};

pub(in crate::gui_app) fn folder_browser_view(
    state: &FolderBrowserState,
    sidebar_width: f32,
    has_selected_file: bool,
    metadata_tag_draft: &str,
    metadata_tag_tokens: &[String],
    metadata_tag_pending_category_tag: Option<&str>,
    metadata_tag_input_placeholder: &str,
    metadata_tag_completion_suffix: Option<&str>,
    metadata_tag_completion_options: &[MetadataTagCompletionOption],
    metadata_tags: &[String],
    metadata_tag_display_categories: &[MetadataTagDisplayCategory],
    selected_metadata_tag: Option<&str>,
) -> ui::View<GuiMessage> {
    let tag_field_content_width = tag_field_content_width(sidebar_width);
    let tag_field_height = tag_field_height(
        metadata_tag_draft,
        metadata_tag_tokens,
        metadata_tag_pending_category_tag,
        metadata_tag_completion_suffix,
        metadata_tags,
        metadata_tag_display_categories,
        tag_field_content_width,
    );
    ui::column([
        source_selector(state),
        ui::text("Folders").height(22.0).fill_width(),
        ui::scroll(folder_tree_view(state)).fill(),
        selected_folder_status(state),
        collections_section(state),
        filter_section(),
        metadata_section(
            metadata_tag_draft,
            metadata_tag_tokens,
            metadata_tag_pending_category_tag,
            metadata_tag_input_placeholder,
            metadata_tag_completion_suffix,
            metadata_tag_completion_options,
            metadata_tags,
            metadata_tag_display_categories,
            selected_metadata_tag,
            tag_field_content_width,
            tag_field_height,
            has_selected_file,
        ),
    ])
    .spacing(3.0)
    .padding(4.0)
    .style(WidgetStyle::default())
    .fill_height()
}

fn collections_section(state: &FolderBrowserState) -> ui::View<GuiMessage> {
    let rows = state
        .visible_collections()
        .into_iter()
        .map(|collection| collection_row(state, collection))
        .collect::<Vec<_>>();
    sidebar_panel(
        ui::column([
            ui::row([
                ui::text("Collections").height(20.0).fill_width(),
                ui::drag_handle_mapped(|message| {
                    GuiMessage::FolderBrowser(FolderBrowserMessage::ResizeCollectionsPanel(message))
                })
                .key("collections-resize-handle")
                .size(26.0, 18.0),
            ])
            .spacing(4.0)
            .height(20.0)
            .fill_width(),
            ui::scroll(ui::column(rows).spacing(1.0).fill_width().height(
                COLLECTION_ROW_HEIGHT * wavecrate::sample_sources::SampleCollection::COUNT as f32,
            ))
            .style(WidgetStyle {
                tone: WidgetTone::Neutral,
                prominence: ui::WidgetProminence::Subtle,
            })
            .fill_width()
            .fill_height(),
        ])
        .spacing(4.0)
        .fill_width()
        .fill_height(),
        state.collections_panel_height(),
    )
}

fn collection_row(
    state: &FolderBrowserState,
    collection: super::SampleCollectionView,
) -> ui::View<GuiMessage> {
    let collection_id = collection.collection;
    if let Some(rename) = state.collection_rename_view(collection_id) {
        let caret = rename.draft.chars().count();
        return ui::row([
            ui::custom_widget(CollectionHitTarget::new(&collection), |_| None)
                .key(format!(
                    "collection-rename-swatch-{}",
                    collection.collection.index()
                ))
                .width(34.0)
                .height(COLLECTION_ROW_HEIGHT),
            ui::text_input(rename.draft)
                .selection(0, caret)
                .message_event(|message| {
                    GuiMessage::FolderBrowser(FolderBrowserMessage::RenameInput(message))
                })
                .id(rename.input_id)
                .key(format!(
                    "collection-rename-input-{}",
                    collection.collection.index()
                ))
                .fill_width()
                .height(COLLECTION_ROW_HEIGHT),
        ])
        .key(format!(
            "collection-rename-row-{}",
            collection.collection.index()
        ))
        .fill_width()
        .height(COLLECTION_ROW_HEIGHT)
        .spacing(2.0);
    }
    ui::custom_widget_mapped(
        CollectionHitTarget::new(&collection),
        move |message| match message {
            CollectionHitMessage::Activate => {
                GuiMessage::FolderBrowser(FolderBrowserMessage::ActivateCollection(collection_id))
            }
            CollectionHitMessage::Rename => {
                GuiMessage::FolderBrowser(FolderBrowserMessage::RenameCollection(collection_id))
            }
            CollectionHitMessage::Drop => {
                GuiMessage::FolderBrowser(FolderBrowserMessage::DropOnCollection(collection_id))
            }
            CollectionHitMessage::HoverDropTarget(position) => GuiMessage::FolderBrowser(
                FolderBrowserMessage::HoverCollectionDropTarget(collection_id, position),
            ),
        },
    )
    .key(format!("collection-row-{}", collection.collection.index()))
    .fill_width()
    .height(COLLECTION_ROW_HEIGHT)
}

fn folder_tree_view(state: &FolderBrowserState) -> ui::View<GuiMessage> {
    ui::stack([
        ui::custom_widget_mapped(
            FolderDropClearTarget::new(state.drop_target_folder.is_some()),
            GuiMessage::FolderBrowser,
        )
        .key("folder-drop-clear-target")
        .input_only()
        .fill(),
        ui::column(
            state
                .visible_folders()
                .into_iter()
                .map(folder_row)
                .collect::<Vec<_>>(),
        )
        .fill_width()
        .spacing(1.0),
    ])
    .fill()
}

fn source_selector(state: &FolderBrowserState) -> ui::View<GuiMessage> {
    ui::column([
        ui::row([
            ui::text("Sources").height(20.0).fill_width(),
            ui::button("+")
                .primary()
                .message(GuiMessage::FolderBrowser(FolderBrowserMessage::AddSource))
                .key("source-add-button")
                .size(28.0, 22.0),
        ])
        .spacing(3.0)
        .fill_width()
        .height(24.0),
        ui::column(
            state
                .sources
                .iter()
                .map(|source| source_row(state, source))
                .collect::<Vec<_>>(),
        )
        .spacing(2.0)
        .fill_width(),
    ])
    .spacing(3.0)
    .fill_width()
}

fn source_row(state: &FolderBrowserState, source: &SourceEntry) -> ui::View<GuiMessage> {
    let id = source.id.clone();
    let row_key = source.id.clone();
    let menu_id = source.id.clone();
    let selected = state.selected_source == source.id;
    let label = if source.loading_task.is_some() {
        format!("{} (scanning)", source.label)
    } else {
        source.label.clone()
    };
    let mut row = ui::button(label)
        .secondary_clicks()
        .mapped(move |message| match message {
            ButtonMessage::SecondaryActivate { position } => GuiMessage::FolderBrowser(
                FolderBrowserMessage::OpenSourceContextMenu(menu_id.clone(), position),
            ),
            _ => GuiMessage::FolderBrowser(FolderBrowserMessage::SelectSource(id.clone())),
        })
        .key(format!("source-row-{row_key}"))
        .fill_width()
        .height(24.0);
    if selected {
        row = row.primary();
    } else {
        row = row.subtle();
    }
    row.style(if selected {
        WidgetStyle {
            tone: WidgetTone::Accent,
            prominence: ui::WidgetProminence::Subtle,
        }
    } else {
        WidgetStyle::default()
    })
    .fill_width()
}

fn folder_row(folder: VisibleFolder) -> ui::View<GuiMessage> {
    let id = folder.id.clone();
    if let (Some(draft), Some(input_id)) = (folder.rename_draft.clone(), folder.rename_input_id) {
        let caret = draft.chars().count();
        let indent = (folder.depth as f32) * TREE_DEPTH_INDENT;
        return ui::row([
            ui::spacer().width(indent).height(22.0),
            ui::text_input(draft)
                .selection(0, caret)
                .message_event(|message| {
                    GuiMessage::FolderBrowser(FolderBrowserMessage::RenameInput(message))
                })
                .id(input_id)
                .key(format!("folder-rename-input-{id}"))
                .fill_width()
                .height(22.0),
        ])
        .key(format!("folder-row-{id}"))
        .style(WidgetStyle {
            tone: WidgetTone::Accent,
            prominence: ui::WidgetProminence::Subtle,
        })
        .fill_width()
        .height(TREE_ROW_HEIGHT)
        .spacing(1.0)
        .hoverable();
    }

    let expander = if folder.expanded { "[-]" } else { "[+]" };
    let indent = (folder.depth as f32) * TREE_DEPTH_INDENT;
    let label_text = if folder.has_children {
        format!("{expander} {}", folder.name)
    } else {
        format!("    {}", folder.name)
    };
    let hit_id = id.clone();
    let hit_target = ui::custom_widget_mapped(
        FolderTreeHitTarget::new(
            label_text,
            folder.selected,
            folder.drop_target,
            folder.drag_active,
            folder.drag_source,
            folder.drop_candidate,
            folder.drop_target_active,
        ),
        move |message| match message {
            FolderTreeHitMessage::Activate => {
                GuiMessage::FolderBrowser(FolderBrowserMessage::ActivateFolder(hit_id.clone()))
            }
            FolderTreeHitMessage::ContextMenu(position) => GuiMessage::FolderBrowser(
                FolderBrowserMessage::OpenFolderContextMenu(hit_id.clone(), position),
            ),
            FolderTreeHitMessage::Drag(drag) => {
                GuiMessage::FolderBrowser(FolderBrowserMessage::DragFolder(hit_id.clone(), drag))
            }
            FolderTreeHitMessage::Drop => {
                GuiMessage::FolderBrowser(FolderBrowserMessage::DropOnFolder(hit_id.clone()))
            }
            FolderTreeHitMessage::HoverDropTarget(position) => GuiMessage::FolderBrowser(
                FolderBrowserMessage::HoverDropTarget(hit_id.clone(), position),
            ),
        },
    )
    .key(format!("folder-row-hit-{id}"))
    .fill_width()
    .height(22.0);

    ui::row([
        ui::spacer().width(indent).height(22.0),
        hit_target.fill_width().height(22.0),
    ])
    .key(format!("folder-row-{id}"))
    .style(if folder.selected || folder.drop_target {
        WidgetStyle {
            tone: WidgetTone::Accent,
            prominence: ui::WidgetProminence::Subtle,
        }
    } else {
        WidgetStyle::default()
    })
    .fill_width()
    .height(TREE_ROW_HEIGHT)
    .spacing(1.0)
}

fn selected_folder_status(state: &FolderBrowserState) -> ui::View<GuiMessage> {
    let file_count = state.selected_files().len();
    let audio_count = state.selected_audio_files().len();
    let label = state
        .selected_folder()
        .map(|folder| {
            format!(
                "{} | {audio_count} audio | {file_count} item{}",
                folder.name,
                plural(file_count)
            )
        })
        .unwrap_or_else(|| String::from("No folder selected"));
    ui::text(label).height(20.0).fill_width().truncate()
}

fn filter_section() -> ui::View<GuiMessage> {
    sidebar_section(
        "Filter",
        ui::column([
            ui::row([
                ui::text("Name").height(20.0).width(48.0),
                ui::text("Any").height(20.0).fill_width(),
            ])
            .fill_width()
            .height(20.0)
            .spacing(6.0),
            ui::row([
                ui::text("Type").height(20.0).width(48.0),
                ui::text("Audio").height(20.0).fill_width(),
            ])
            .fill_width()
            .height(20.0)
            .spacing(6.0),
        ])
        .fill_width()
        .spacing(2.0),
        76.0,
    )
}

fn sidebar_section(
    title: &'static str,
    content: ui::View<GuiMessage>,
    height: f32,
) -> ui::View<GuiMessage> {
    sidebar_panel(
        ui::column([ui::text(title).height(20.0).fill_width(), content])
            .spacing(4.0)
            .fill_width(),
        height,
    )
}

fn sidebar_panel(content: ui::View<GuiMessage>, height: f32) -> ui::View<GuiMessage> {
    content
        .style(WidgetStyle {
            tone: WidgetTone::Neutral,
            prominence: ui::WidgetProminence::Subtle,
        })
        .padding(6.0)
        .fill_width()
        .height(height)
}
