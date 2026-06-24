use radiant::prelude as ui;

use super::identity::{SOURCE_ADD_BUTTON_ID, SOURCE_ROW_INPUT_SCOPE, source_row_key};
use crate::native_app::app::GuiMessage;
use crate::native_app::app_chrome::library_browser::library_sidebar::sidebar_row::sidebar_row_underlay;
use crate::native_app::app_chrome::toolbar::toolbar_icon_color;
use crate::native_app::app_chrome::view_models::library_sidebar::SourceRowViewModel;
use crate::native_app::sample_library::folder_browser::commands::FolderBrowserMessage;

pub(super) const SOURCE_ROW_LABEL_PADDING_X: f32 = 12.0;
pub(super) const SOURCE_ADD_BUTTON_WIDTH: f32 = 28.0;
pub(super) const SOURCE_ADD_BUTTON_HEIGHT: f32 = 24.0;
const SOURCE_ROW_HEIGHT: f32 = 24.0;

pub(super) fn source_add_button() -> ui::View<GuiMessage> {
    ui::icon_button(source_add_icon())
        .message(GuiMessage::FolderBrowser(FolderBrowserMessage::AddSource))
        .id(SOURCE_ADD_BUTTON_ID)
        .size(SOURCE_ADD_BUTTON_WIDTH, SOURCE_ADD_BUTTON_HEIGHT)
}

fn source_add_icon() -> ui::SvgIcon {
    SOURCE_ADD_ICON.icon(toolbar_icon_color(true, false))
}

pub(super) fn source_row(source: &SourceRowViewModel) -> ui::View<GuiMessage> {
    let visual = source_row_content(source_row_label(source));
    sidebar_row_underlay(visual)
        .stable_input_id(SOURCE_ROW_INPUT_SCOPE, source.id.as_str())
        .selected(source.selected)
        .actions(ui::row_actions().primary_secondary_key(
            source.id.clone(),
            |source_id| GuiMessage::FolderBrowser(FolderBrowserMessage::SelectSource(source_id)),
            |source_id, position| {
                GuiMessage::FolderBrowser(FolderBrowserMessage::OpenSourceContextMenu(
                    source_id, position,
                ))
            },
        ))
        .key(source_row_key(source.id.as_str()))
        .fill_width()
        .height(SOURCE_ROW_HEIGHT)
}

fn source_row_label(source: &SourceRowViewModel) -> String {
    if source.scanning {
        format!("{} (scanning)", source.label)
    } else {
        source.label.clone()
    }
}

fn source_row_content(label: String) -> ui::View<GuiMessage> {
    ui::row([
        ui::spacer()
            .width(SOURCE_ROW_LABEL_PADDING_X)
            .height(SOURCE_ROW_HEIGHT),
        ui::text_line(label, SOURCE_ROW_HEIGHT),
    ])
    .spacing(0.0)
    .fill_width()
    .height(SOURCE_ROW_HEIGHT)
}

static SOURCE_ADD_ICON: ui::SvgIconTintCache = ui::SvgIconTintCache::new(
    r#"<svg viewBox="0 0 16 16" xmlns="http://www.w3.org/2000/svg">
  <rect x="4" y="7.25" width="8" height="1.5"/>
  <rect x="7.25" y="4" width="1.5" height="8"/>
</svg>"#,
);
