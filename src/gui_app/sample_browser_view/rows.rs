use radiant::prelude as ui;

use super::{SampleFileHitMessage, SampleFileHitTarget};
use crate::gui_app::{
    GuiMessage, SAMPLE_BROWSER_LIST_ID, SAMPLE_BROWSER_OVERSCAN_ROWS, SAMPLE_BROWSER_ROW_HEIGHT,
    folder_browser::{self, FileColumn, FileEntry, FolderBrowserMessage, FolderBrowserState},
};

pub(super) fn sample_browser_rows(
    folder_browser: &FolderBrowserState,
    files: &[&FileEntry],
    columns: &[&FileColumn],
    window: ui::VirtualListWindow,
) -> ui::View<GuiMessage> {
    if files.is_empty() {
        return ui::text("No audio files in selected folder")
            .height(24.0)
            .fill_width()
            .fill_height();
    }

    ui::virtual_list_window(
        window,
        SAMPLE_BROWSER_ROW_HEIGHT,
        |index| {
            let file = files[index];
            sample_browser_row(
                file,
                folder_browser.is_file_selected(&file.id),
                folder_browser.file_rename_view(&file.id),
                folder_browser.drag_revision(),
                columns,
            )
        },
        SAMPLE_BROWSER_ROW_HEIGHT * SAMPLE_BROWSER_OVERSCAN_ROWS as f32,
    )
    .id(SAMPLE_BROWSER_LIST_ID)
    .fill()
}

fn sample_browser_row(
    file: &FileEntry,
    selected: bool,
    rename: Option<folder_browser::FileRenameView>,
    drag_revision: u64,
    columns: &[&FileColumn],
) -> ui::View<GuiMessage> {
    let hit_path = file.id.clone();
    let hit_target = sample_file_hit_target(file, selected, drag_revision, hit_path);
    let row = ui::stack([
        hit_target,
        compact_details_row(
            columns
                .iter()
                .map(|column| sample_column_cell(file, rename.clone(), column)),
        ),
    ])
    .key(format!("sample-row-{}", file.id))
    .fill_width()
    .height(22.0);
    row.style(ui::WidgetStyle::default())
}

fn sample_file_hit_target(
    file: &FileEntry,
    selected: bool,
    drag_revision: u64,
    hit_path: String,
) -> ui::View<GuiMessage> {
    ui::custom_widget_mapped(
        SampleFileHitTarget::new(selected),
        move |message| match message {
            SampleFileHitMessage::Activate(modifiers) => GuiMessage::SelectSampleWithModifiers {
                path: hit_path.clone(),
                modifiers,
            },
            SampleFileHitMessage::ContextMenu(position) => GuiMessage::OpenSampleContextMenu {
                path: hit_path.clone(),
                position,
            },
            SampleFileHitMessage::Drag(drag) => GuiMessage::DragSampleFile {
                path: hit_path.clone(),
                drag,
            },
        },
    )
    .key(format!("sample-row-hit-{}-{drag_revision}", file.id))
    .fill_width()
    .height(22.0)
}

fn sample_column_cell(
    file: &FileEntry,
    rename: Option<folder_browser::FileRenameView>,
    column: &FileColumn,
) -> ui::View<GuiMessage> {
    if column.id == "name" {
        return sample_name_cell(file, rename, column.width);
    }
    sample_file_cell(
        file,
        sample_file_column_value(file, column.id.as_str()),
        column.width,
        column.id.as_str(),
    )
}

fn sample_name_cell(
    file: &FileEntry,
    rename: Option<folder_browser::FileRenameView>,
    width: f32,
) -> ui::View<GuiMessage> {
    let Some(rename) = rename else {
        return sample_file_cell(file, file.stem.clone(), width, "name");
    };
    ui::text_input(rename.draft)
        .selection(rename.selection_start, rename.selection_end)
        .message_event(|message| {
            GuiMessage::FolderBrowser(FolderBrowserMessage::RenameInput(message))
        })
        .id(rename.input_id)
        .key(format!("sample-rename-input-{}", file.id))
        .width(width)
        .height(20.0)
}

fn sample_file_column_value(file: &FileEntry, column_id: &str) -> String {
    match column_id {
        "extension" => file.extension.clone(),
        "size" => file.size.clone(),
        "modified" => file.modified.clone(),
        "kind" => file.kind.clone(),
        "path" => file.id.clone(),
        _ => file.stem.clone(),
    }
}

fn sample_file_cell(
    file: &FileEntry,
    value: String,
    width: f32,
    column_id: &str,
) -> ui::View<GuiMessage> {
    ui::text(value)
        .key(format!("sample-{}-{column_id}", file.id))
        .height(20.0)
        .width(width)
        .truncate()
}

fn compact_details_row(
    children: impl IntoIterator<Item = ui::View<GuiMessage>>,
) -> ui::View<GuiMessage> {
    ui::row(children)
        .fill_width()
        .height(22.0)
        .padding_x(8.0)
        .padding_y(1.0)
        .spacing(10.0)
}
