use radiant::prelude as ui;
use radiant::widgets::ButtonMessage;

use super::folder_browser::{FILE_COLUMN_GAP, FileColumn, FolderBrowserMessage};
use super::{
    GuiAppState, GuiMessage, SAMPLE_BROWSER_EDGE_CONTEXT_ROWS, SAMPLE_BROWSER_OVERSCAN_ROWS,
    SAMPLE_BROWSER_PROJECTED_VIEWPORT_ROWS, SampleNameViewMode,
};

mod hit_target;
pub(super) use hit_target::{SampleFileHitMessage, SampleFileHitTarget};

mod row_widgets;
mod rows;
use rows::sample_browser_rows;

pub(super) fn sample_browser(
    state: &mut GuiAppState,
    suppress_row_hover: bool,
) -> ui::View<GuiMessage> {
    let window = state.folder_browser.follow_selected_file_view(
        SAMPLE_BROWSER_PROJECTED_VIEWPORT_ROWS,
        SAMPLE_BROWSER_OVERSCAN_ROWS,
        SAMPLE_BROWSER_EDGE_CONTEXT_ROWS,
    );
    let audio_files = state.folder_browser.selected_audio_files();
    let audio_count = audio_files.len();
    let columns = state.folder_browser.visible_file_columns();
    let browser = ui::column([
        sample_browser_header_bar(
            &columns,
            state.folder_browser.file_sort(),
            state.sample_name_view_mode,
        ),
        sample_browser_rows(
            &state.folder_browser,
            &audio_files,
            &columns,
            window,
            state.sample_name_view_mode,
            &state.metadata_tags_by_file,
            &state.cached_sample_paths,
            suppress_row_hover,
        ),
        sample_browser_status(audio_count),
    ])
    .spacing(0.0)
    .style(ui::WidgetStyle::default())
    .fill();
    if !state.folder_browser.extracted_file_drag_active() {
        return browser;
    }
    ui::stack([
        browser,
        ui::pointer_drop_shield(true)
            .on_drop(GuiMessage::DropWaveformSelectionOnSampleList)
            .key("sample-list-waveform-drop-target")
            .input_only()
            .fill(),
    ])
    .fill()
}

fn sample_browser_header_bar(
    columns: &[&FileColumn],
    sort: &ui::DetailsSort,
    mode: SampleNameViewMode,
) -> ui::View<GuiMessage> {
    ui::row([
        sample_browser_header(columns, sort).fill_width(),
        sample_name_view_mode_button(mode),
    ])
    .fill_width()
    .height(24.0)
    .spacing(6.0)
}

fn sample_name_view_mode_button(mode: SampleNameViewMode) -> ui::View<GuiMessage> {
    let label = match mode {
        SampleNameViewMode::DiskFilename => "Disk",
        SampleNameViewMode::MetadataLabel => "Label",
    };
    ui::button(label)
        .message(GuiMessage::ToggleSampleNameViewMode)
        .key("sample-name-view-mode-toggle")
        .size(58.0, 22.0)
}

fn sample_browser_header(columns: &[&FileColumn], sort: &ui::DetailsSort) -> ui::View<GuiMessage> {
    details_header_row(
        columns
            .iter()
            .map(|column| sample_header_cell(column, sort)),
    )
}

fn sample_header_cell(column: &FileColumn, sort: &ui::DetailsSort) -> ui::View<GuiMessage> {
    let marker = if sort.column_id == column.id {
        match sort.direction {
            ui::SortDirection::Ascending => " ^",
            ui::SortDirection::Descending => " v",
        }
    } else {
        ""
    };
    let sort_id = column.id.clone();
    let drag_id = column.id.clone();
    let resize_id = column.id.clone();
    let label = format!("{}{marker}", column.label);
    ui::row([
        ui::stack([
            ui::text(label.clone())
                .key(format!("sample-header-label-{}", column.id))
                .align_text(ui::TextAlign::Left)
                .fill_width()
                .height(20.0)
                .truncate(),
            ui::button(label)
                .draggable()
                .mapped(move |message| match message {
                    ButtonMessage::Activate => GuiMessage::FolderBrowser(
                        FolderBrowserMessage::SortFileColumn(sort_id.clone()),
                    ),
                    ButtonMessage::Drag(drag) => GuiMessage::FolderBrowser(
                        FolderBrowserMessage::DragFileColumn(drag_id.clone(), drag),
                    ),
                    ButtonMessage::SecondaryActivate { .. } => GuiMessage::Noop,
                })
                .key(format!("sample-sort-{}", column.id))
                .fill_width()
                .height(20.0)
                .input_only(),
        ])
        .fill_width()
        .height(20.0),
        ui::drag_handle()
            .mapped(move |message| {
                GuiMessage::FolderBrowser(FolderBrowserMessage::ResizeFileColumn(
                    resize_id.clone(),
                    message,
                ))
            })
            .key(format!("sample-column-resize-{}", column.id))
            .size(4.0, 20.0),
    ])
    .width(column.width)
    .height(20.0)
    .spacing(1.0)
}

fn details_header_row(
    children: impl IntoIterator<Item = ui::View<GuiMessage>>,
) -> ui::View<GuiMessage> {
    ui::row(children)
        .style(ui::WidgetStyle {
            tone: ui::WidgetTone::Accent,
            prominence: ui::WidgetProminence::Subtle,
        })
        .fill_width()
        .height(24.0)
        .padding_x(8.0)
        .padding_y(2.0)
        .spacing(FILE_COLUMN_GAP)
}

fn sample_browser_status(audio_count: usize) -> ui::View<GuiMessage> {
    ui::row([
        ui::text("Listed").height(20.0).width(90.0),
        ui::text(format!(
            "{audio_count} audio file{} in selected folder",
            if audio_count == 1 { "" } else { "s" }
        ))
        .height(20.0)
        .fill_width(),
    ])
    .padding_x(3.0)
    .fill_width()
    .height(28.0)
}
