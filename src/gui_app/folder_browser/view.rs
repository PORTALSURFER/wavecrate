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

const TAG_FIELD_CONTROL_HEIGHT: f32 = 18.0;
const TAG_FIELD_ITEM_GAP: f32 = 3.0;
const TAG_FIELD_LINE_GAP: f32 = 3.0;
const TAG_FIELD_HORIZONTAL_CHROME: f32 = 26.0;
const TAG_FIELD_VERTICAL_CHROME: f32 = 6.0;
const MAX_TAG_FIELD_ROWS: usize = 6;
const MIN_TAG_INPUT_REMAINING_WIDTH: f32 = 180.0;
const METADATA_TAG_INPUT_ID: u64 = 0x5743_0000_0000_5447;

pub(in crate::gui_app) fn folder_browser_view(
    state: &FolderBrowserState,
    sidebar_width: f32,
    has_selected_file: bool,
    metadata_tag_draft: &str,
    metadata_tag_tokens: &[String],
    metadata_tag_suggestion: Option<&str>,
    metadata_tags: &[String],
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
            tag_field_content_width(sidebar_width),
            has_selected_file,
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
    tag_field_content_width: f32,
    has_selected_file: bool,
) -> ui::View<GuiMessage> {
    if !has_selected_file {
        return sidebar_section("Metadata", ui::spacer().height(0.0).fill_width(), 36.0);
    }

    let tag_field_height = tag_field_height(
        tag_draft,
        tag_tokens,
        tag_suggestion,
        tags,
        tag_field_content_width,
    );
    let section_height = 62.0 + tag_field_height;
    sidebar_section(
        "Metadata",
        ui::column([
            ui::text(format!("Tags ({})", tags.len()))
                .height(22.0)
                .fill_width(),
            tag_entry_field(
                tag_draft,
                tag_tokens,
                tag_suggestion,
                tags,
                tag_field_height,
                tag_field_content_width,
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
    content_width: f32,
) -> ui::View<GuiMessage> {
    let mut visible_tags = tags.to_vec();
    for token in tag_tokens {
        if !visible_tags.iter().any(|tag| tag == token) {
            visible_tags.push(token.clone());
        }
    }

    let input_width = tag_input_width(tag_draft);
    let rows = tag_field_rows(&visible_tags, input_width, tag_suggestion, content_width);
    let row_count = rows.len();
    let content = ui::column(
        rows.into_iter()
            .enumerate()
            .map(|(row_index, row)| tag_entry_row(row, tag_draft, row_index))
            .collect::<Vec<_>>(),
    )
    .fill_width()
    .height(rows_height(row_count))
    .spacing(TAG_FIELD_LINE_GAP);

    ui::scroll(content)
        .style(WidgetStyle {
            tone: WidgetTone::Neutral,
            prominence: ui::WidgetProminence::Subtle,
        })
        .padding(3.0)
        .fill_width()
        .height(height)
}

fn tag_field_content_width(sidebar_width: f32) -> f32 {
    (sidebar_width - TAG_FIELD_HORIZONTAL_CHROME).max(120.0)
}

fn tag_field_height(
    tag_draft: &str,
    tag_tokens: &[String],
    tag_suggestion: Option<&str>,
    tags: &[String],
    content_width: f32,
) -> f32 {
    let mut visible_tags = tags.to_vec();
    for token in tag_tokens {
        if !visible_tags.iter().any(|tag| tag == token) {
            visible_tags.push(token.clone());
        }
    }
    let input_width = tag_input_width(tag_draft);
    let rows = tag_field_rows(&visible_tags, input_width, tag_suggestion, content_width).len();
    rows_height(rows.min(MAX_TAG_FIELD_ROWS).max(1)) + TAG_FIELD_VERTICAL_CHROME
}

#[derive(Clone, Debug, PartialEq)]
enum TagEntryRowItem {
    Accepted(String),
    Input(f32),
    Completion(String),
}

fn tag_field_rows(
    tags: &[String],
    input_width: f32,
    tag_suggestion: Option<&str>,
    content_width: f32,
) -> Vec<Vec<TagEntryRowItem>> {
    let mut rows = pack_tag_rows(tags, content_width);
    if should_break_before_tag_input(tags, input_width, content_width) || rows.is_empty() {
        rows.push(Vec::new());
    }

    push_row_item(
        &mut rows,
        TagEntryRowItem::Input(input_width),
        input_width,
        content_width,
    );
    if let Some(suggestion) = tag_suggestion {
        push_row_item(
            &mut rows,
            TagEntryRowItem::Completion(suggestion.to_string()),
            tag_completion_width(suggestion),
            content_width,
        );
    }

    rows
}

fn pack_tag_rows(tags: &[String], content_width: f32) -> Vec<Vec<TagEntryRowItem>> {
    let mut rows = Vec::new();
    for tag in tags {
        push_row_item(
            &mut rows,
            TagEntryRowItem::Accepted(tag.clone()),
            tag_pill_width(tag),
            content_width,
        );
    }
    rows
}

fn push_row_item(
    rows: &mut Vec<Vec<TagEntryRowItem>>,
    item: TagEntryRowItem,
    width: f32,
    content_width: f32,
) {
    if rows.is_empty() {
        rows.push(Vec::new());
    }

    let current_width = row_width(rows.last().expect("row exists"));
    let proposed = if current_width <= 0.0 {
        width
    } else {
        current_width + TAG_FIELD_ITEM_GAP + width
    };
    if proposed > content_width && current_width > 0.0 {
        rows.push(Vec::new());
    }
    rows.last_mut().expect("row exists").push(item);
}

fn row_width(row: &[TagEntryRowItem]) -> f32 {
    row.iter()
        .map(tag_entry_row_item_width)
        .reduce(|total, width| total + TAG_FIELD_ITEM_GAP + width)
        .unwrap_or(0.0)
}

fn tag_entry_row_item_width(item: &TagEntryRowItem) -> f32 {
    match item {
        TagEntryRowItem::Accepted(tag) => tag_pill_width(tag),
        TagEntryRowItem::Input(width) => *width,
        TagEntryRowItem::Completion(tag) => tag_completion_width(tag),
    }
}

fn should_break_before_tag_input(tags: &[String], input_width: f32, content_width: f32) -> bool {
    let mut row_width = 0.0;
    for tag in tags {
        let width = tag_pill_width(tag);
        let proposed = if row_width <= 0.0 {
            width
        } else {
            row_width + TAG_FIELD_ITEM_GAP + width
        };
        if proposed > content_width && row_width > 0.0 {
            row_width = width;
        } else {
            row_width = proposed;
        }
    }

    row_width > 0.0
        && content_width - row_width - TAG_FIELD_ITEM_GAP
            < input_width.max(MIN_TAG_INPUT_REMAINING_WIDTH)
}

fn rows_height(row_count: usize) -> f32 {
    if row_count == 0 {
        return 0.0;
    }
    row_count as f32 * TAG_FIELD_CONTROL_HEIGHT
        + row_count.saturating_sub(1) as f32 * TAG_FIELD_LINE_GAP
}

fn tag_text_input(tag_draft: &str, width: f32) -> ui::View<GuiMessage> {
    ui::text_input(tag_draft.to_string())
        .placeholder("add tag")
        .underline()
        .message_event(GuiMessage::MetadataTagInput)
        .id(METADATA_TAG_INPUT_ID)
        .key("metadata-tag-input")
        .sizing(ui::WidgetSizing::fixed(ui::Vector2::new(
            width,
            TAG_FIELD_CONTROL_HEIGHT,
        )))
        .height(TAG_FIELD_CONTROL_HEIGHT)
        .width(width)
}

fn tag_entry_row(
    row: Vec<TagEntryRowItem>,
    tag_draft: &str,
    row_index: usize,
) -> ui::View<GuiMessage> {
    ui::row(
        row.into_iter()
            .map(|item| match item {
                TagEntryRowItem::Accepted(tag) => accepted_tag_token(tag.as_str()),
                TagEntryRowItem::Input(width) => tag_text_input(tag_draft, width),
                TagEntryRowItem::Completion(tag) => tag_completion_token(tag.as_str()),
            })
            .collect::<Vec<_>>(),
    )
    .key(format!("metadata-tag-row-{row_index}"))
    .height(TAG_FIELD_CONTROL_HEIGHT)
    .fill_width()
    .spacing(TAG_FIELD_ITEM_GAP)
}

fn tag_input_width(value: &str) -> f32 {
    let char_width = value.chars().count().max(7) as f32;
    (char_width * 7.0 + 12.0).clamp(61.0, 180.0)
}

fn tag_pill_width(tag: &str) -> f32 {
    (tag.chars().count() as f32 * 7.0 + 22.0).clamp(38.0, 180.0)
}

fn tag_completion_width(tag: &str) -> f32 {
    tag_pill_width(tag)
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
        .sizing(ui::WidgetSizing::fixed(ui::Vector2::new(
            tag_pill_width(tag),
            TAG_FIELD_CONTROL_HEIGHT,
        )))
        .height(TAG_FIELD_CONTROL_HEIGHT)
        .width(tag_pill_width(tag))
}

fn tag_completion_token(tag: &str) -> ui::View<GuiMessage> {
    let width = tag_completion_width(tag);
    ui::badge(tag.to_string())
        .subtle()
        .message(GuiMessage::Noop)
        .key(format!("metadata-tag-completion-{tag}"))
        .style(WidgetStyle {
            tone: WidgetTone::Neutral,
            prominence: ui::WidgetProminence::Subtle,
        })
        .sizing(ui::WidgetSizing::fixed(ui::Vector2::new(
            width,
            TAG_FIELD_CONTROL_HEIGHT,
        )))
        .height(TAG_FIELD_CONTROL_HEIGHT)
        .width(width)
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
