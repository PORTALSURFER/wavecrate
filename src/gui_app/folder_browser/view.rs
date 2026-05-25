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

pub(in crate::gui_app) fn folder_browser_view(
    state: &FolderBrowserState,
    metadata_tag_draft: &str,
    metadata_tag_tokens: &[String],
    metadata_tag_suggestion: Option<&str>,
    metadata_tags: &[String],
    metadata_tags_expanded: bool,
) -> ui::View<GuiMessage> {
    ui::column([
        source_selector(state),
        ui::text("Folders").height(22.0).fill_width(),
        ui::scroll(folder_tree_view(state)).fill(),
        selected_folder_status(state),
        filter_section(),
        metadata_section(
            metadata_tag_draft,
            metadata_tag_tokens,
            metadata_tag_suggestion,
            metadata_tags,
            metadata_tags_expanded,
        ),
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

fn metadata_section(
    tag_draft: &str,
    tag_tokens: &[String],
    tag_suggestion: Option<&str>,
    tags: &[String],
    expanded: bool,
) -> ui::View<GuiMessage> {
    let tag_field_height = if expanded { 130.0 } else { 82.0 };
    let section_height = if expanded { 220.0 } else { 172.0 };
    let toggle_label = if expanded { "Less" } else { "More" };
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
                ui::text(format!("Tags ({})", tags.len()))
                    .height(20.0)
                    .fill_width(),
                ui::button(toggle_label)
                    .message(GuiMessage::ToggleMetadataTagsExpanded)
                    .key("metadata-tags-expand-toggle")
                    .height(20.0)
                    .width(54.0),
            ])
            .fill_width()
            .height(22.0)
            .spacing(4.0),
            tag_entry_field(
                tag_draft,
                tag_tokens,
                tag_suggestion,
                tags,
                tag_field_height,
            )
            .key("metadata-tag-entry-field")
            .fill_width()
            .height(tag_field_height),
        ])
        .fill_width()
        .spacing(4.0),
        section_height,
    )
}

fn tag_entry_field(
    tag_draft: &str,
    tag_tokens: &[String],
    tag_suggestion: Option<&str>,
    tags: &[String],
    height: f32,
) -> ui::View<GuiMessage> {
    let mut visible_tags = tags.to_vec();
    for token in tag_tokens {
        if !visible_tags.iter().any(|tag| tag == token) {
            visible_tags.push(token.clone());
        }
    }

    let mut children = visible_tags
        .iter()
        .map(|tag| accepted_tag_token(tag.as_str()))
        .collect::<Vec<_>>();
    children.push(
        ui::text_input(tag_draft.to_string())
            .placeholder("add tag")
            .message_event(GuiMessage::MetadataTagInput)
            .key("metadata-tag-input")
            .height(24.0)
            .width(tag_input_width(tag_draft)),
    );
    if let Some(suggestion) = tag_suggestion {
        children.push(tag_completion_token(suggestion));
    }

    ui::scroll(ui::wrap(children, 4.0, 4.0).fill_width())
        .style(WidgetStyle {
            tone: WidgetTone::Neutral,
            prominence: ui::WidgetProminence::Subtle,
        })
        .padding(3.0)
        .fill_width()
        .height(height)
}

fn tag_input_width(value: &str) -> f32 {
    let char_width = value.chars().count().max(7) as f32;
    (char_width * 7.0 + 42.0).clamp(92.0, 180.0)
}

fn tag_pill_width(tag: &str) -> f32 {
    (tag.chars().count() as f32 * 7.0 + 22.0).clamp(38.0, 180.0)
}

fn accepted_tag_token(tag: &str) -> ui::View<GuiMessage> {
    ui::badge(tag.to_string())
        .subtle()
        .message(GuiMessage::Noop)
        .key(format!("metadata-tag-accepted-{tag}"))
        .style(WidgetStyle {
            tone: WidgetTone::Accent,
            prominence: ui::WidgetProminence::Subtle,
        })
        .height(22.0)
        .width(tag_pill_width(tag))
}

fn tag_completion_token(tag: &str) -> ui::View<GuiMessage> {
    ui::badge(format!("Tab {tag}"))
        .subtle()
        .message(GuiMessage::Noop)
        .key(format!("metadata-tag-completion-{tag}"))
        .style(WidgetStyle {
            tone: WidgetTone::Accent,
            prominence: ui::WidgetProminence::Subtle,
        })
        .height(22.0)
        .width((tag.chars().count() as f32 * 7.0 + 44.0).clamp(58.0, 180.0))
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
