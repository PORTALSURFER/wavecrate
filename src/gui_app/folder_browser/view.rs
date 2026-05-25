use radiant::{
    prelude as ui,
    widgets::{ButtonMessage, WidgetStyle, WidgetTone},
};

use super::{
    FolderBrowserMessage, FolderBrowserState, GuiMessage, SourceEntry, TREE_DEPTH_INDENT,
    TREE_ROW_HEIGHT, VisibleFolder, plural,
    tree_hit_target::{FolderTreeHitMessage, FolderTreeHitTarget},
    tree_widgets::FolderDropClearTarget,
};

pub(in crate::gui_app) fn folder_browser_view(state: &FolderBrowserState) -> ui::View<GuiMessage> {
    ui::column([
        source_selector(state),
        ui::text("Folders").height(22.0).fill_width(),
        ui::scroll(folder_tree_view(state)).fill(),
        selected_folder_status(state),
        filter_section(),
        metadata_section(),
    ])
    .spacing(3.0)
    .padding(4.0)
    .style(WidgetStyle::default())
    .fill_height()
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

fn metadata_section() -> ui::View<GuiMessage> {
    sidebar_section(
        "Metadata",
        ui::column([
            ui::row([ui::text("Tagging")
                .key("metadata-tagging-tab")
                .style(WidgetStyle {
                    tone: WidgetTone::Accent,
                    prominence: ui::WidgetProminence::Subtle,
                })
                .padding(4.0)
                .height(22.0)
                .fill_width()])
            .fill_width()
            .height(24.0),
            ui::row([
                ui::text("Tags").height(20.0).width(48.0),
                ui::text("None").height(20.0).fill_width(),
            ])
            .fill_width()
            .height(20.0)
            .spacing(6.0),
        ])
        .fill_width()
        .spacing(3.0),
        82.0,
    )
}

fn sidebar_section(
    title: &'static str,
    content: ui::View<GuiMessage>,
    height: f32,
) -> ui::View<GuiMessage> {
    ui::column([ui::text(title).height(20.0).fill_width(), content])
        .style(WidgetStyle {
            tone: WidgetTone::Neutral,
            prominence: ui::WidgetProminence::Subtle,
        })
        .padding(6.0)
        .spacing(4.0)
        .fill_width()
        .height(height)
}
