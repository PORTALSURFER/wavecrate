use super::{GuiAppState, GuiMessage};
use crate::gui_app::{
    audio_settings::top_status_bar, context_menu, folder_browser,
    sample_browser_view::sample_browser, status_bar, toolbar::main_toolbar,
    waveform_panel::waveform_panel,
};
use radiant::prelude as ui;

pub(super) fn view(state: &mut GuiAppState) -> ui::View<GuiMessage> {
    let content = ui::column([
        top_status_bar(state),
        center_panel(state),
        status_bar::bottom_status_bar(state),
    ])
    .spacing(0.0)
    .fill();
    let mut layers = vec![content];
    if state.job_details_open {
        if let Some(progress) = state.folder_progress.as_ref() {
            layers.push(status_bar::job_details_popover(progress));
        }
    }
    if let Some(menu) = state.context_menu.as_ref() {
        layers.push(context_menu::overlay(menu));
    }
    if layers.len() > 1 {
        ui::stack(layers).fill()
    } else {
        layers.remove(0)
    }
}

fn center_panel(state: &mut GuiAppState) -> ui::View<GuiMessage> {
    ui::row([folder_sidebar(state), folder_splitter(), main_area(state)])
        .padding(6.0)
        .fill()
}

fn folder_sidebar(state: &GuiAppState) -> ui::View<GuiMessage> {
    folder_browser::folder_browser_view(
        &state.folder_browser,
        state.metadata_tag_draft.as_str(),
        &state.metadata_tags,
    )
    .width(state.folder_width)
    .fill_height()
}

fn folder_splitter() -> ui::View<GuiMessage> {
    ui::column([
        ui::spacer().fill(),
        ui::drag_handle()
            .mapped(GuiMessage::ResizeFolder)
            .key("folder-browser-splitter-handle")
            .size(5.0, 32.0),
        ui::spacer().fill(),
    ])
    .style(ui::WidgetStyle {
        tone: ui::WidgetTone::Accent,
        prominence: ui::WidgetProminence::Subtle,
    })
    .width(11.0)
    .fill_height()
    .padding(2.0)
    .spacing(4.0)
}

fn main_area(state: &mut GuiAppState) -> ui::View<GuiMessage> {
    ui::column([
        main_toolbar(state),
        waveform_panel(state),
        sample_browser(state),
    ])
    .padding(4.0)
    .fill()
}
