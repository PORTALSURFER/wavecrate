use radiant::{
    prelude as ui,
    widgets::{WidgetStyle, WidgetTone},
};

use super::super::{FolderBrowserMessage, FolderBrowserState, GuiMessage, SourceEntry};

pub(super) fn source_selector(state: &FolderBrowserState) -> ui::View<GuiMessage> {
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
        .mapped(move |message| {
            if let Some(position) = message.secondary_position() {
                return GuiMessage::FolderBrowser(FolderBrowserMessage::OpenSourceContextMenu(
                    menu_id.clone(),
                    position,
                ));
            }
            GuiMessage::FolderBrowser(FolderBrowserMessage::SelectSource(id.clone()))
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
        WidgetStyle::new(WidgetTone::Accent, ui::WidgetProminence::Subtle)
    } else {
        WidgetStyle::default()
    })
    .fill_width()
}
