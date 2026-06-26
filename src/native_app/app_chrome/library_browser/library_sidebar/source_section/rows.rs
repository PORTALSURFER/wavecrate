use radiant::prelude as ui;

use super::identity::{
    AUTOMATION_SOURCE_ADD_BUTTON_ID, RETAINED_SOURCE_ROW_INPUT_SCOPE, retained_source_row_key,
};
use crate::native_app::app::GuiMessage;
use crate::native_app::app_chrome::library_browser::library_sidebar::sidebar_row::sidebar_row_underlay;
use crate::native_app::app_chrome::toolbar::toolbar_icon_color;
use crate::native_app::app_chrome::view_models::library_sidebar::SourceRowViewModel;
use crate::native_app::sample_library::folder_browser::commands::FolderBrowserMessage;
use wavecrate::sample_sources::SourceRole;

pub(super) const SOURCE_ROW_LABEL_PADDING_X: f32 = 12.0;
pub(super) const SOURCE_ADD_BUTTON_WIDTH: f32 = 28.0;
pub(super) const SOURCE_ADD_BUTTON_HEIGHT: f32 = 24.0;
const SOURCE_ROW_HEIGHT: f32 = 24.0;
const SOURCE_ROLE_MARKER_WIDTH: f32 = 10.0;
const SOURCE_ROLE_BADGE_WIDTH: f32 = 56.0;
const SOURCE_MISSING_COLOR: ui::Rgba8 = ui::Rgba8::new(255, 112, 86, 230);
const SOURCE_ROLE_PROTECTED_COLOR: ui::Rgba8 = ui::Rgba8::new(234, 178, 73, 230);
const SOURCE_ROLE_PRIMARY_COLOR: ui::Rgba8 = ui::Rgba8::new(72, 196, 162, 230);

pub(super) fn source_add_button() -> ui::View<GuiMessage> {
    ui::icon_button(source_add_icon())
        .message(GuiMessage::FolderBrowser(FolderBrowserMessage::AddSource))
        .id(AUTOMATION_SOURCE_ADD_BUTTON_ID)
        .size(SOURCE_ADD_BUTTON_WIDTH, SOURCE_ADD_BUTTON_HEIGHT)
}

fn source_add_icon() -> ui::SvgIcon {
    SOURCE_ADD_ICON.icon(toolbar_icon_color(true, false))
}

pub(super) fn source_row(source: &SourceRowViewModel) -> ui::View<GuiMessage> {
    let visual = source_row_content(source);
    sidebar_row_underlay(visual)
        .stable_row_identity(
            RETAINED_SOURCE_ROW_INPUT_SCOPE,
            retained_source_row_key(source.id.as_str()),
        )
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

fn source_row_content(source: &SourceRowViewModel) -> ui::View<GuiMessage> {
    ui::row([
        ui::spacer()
            .width(SOURCE_ROW_LABEL_PADDING_X)
            .height(SOURCE_ROW_HEIGHT),
        ui::color_marker(source_marker_color(source))
            .side(7)
            .inset(0)
            .align(ui::ColorMarkerAlign::Center)
            .view()
            .width(SOURCE_ROLE_MARKER_WIDTH)
            .height(SOURCE_ROW_HEIGHT),
        source_label(source),
        source_status_badge(source),
    ])
    .spacing(0.0)
    .fill_width()
    .height(SOURCE_ROW_HEIGHT)
}

fn source_label(source: &SourceRowViewModel) -> ui::View<GuiMessage> {
    let label = ui::text_line(source_row_label(source), SOURCE_ROW_HEIGHT).fill_width();
    if source.missing {
        label.text_color(ui::TextColorRole::Custom(SOURCE_MISSING_COLOR))
    } else {
        label
    }
}

fn source_status_badge(source: &SourceRowViewModel) -> ui::View<GuiMessage> {
    let label = if source.missing {
        "MISSING"
    } else {
        match source.role {
            SourceRole::Protected => "PROT",
            SourceRole::Primary => "PRI",
            SourceRole::Normal => "",
        }
    };
    let text_color = if source.missing {
        ui::TextColorRole::Custom(SOURCE_MISSING_COLOR)
    } else {
        ui::TextColorRole::Muted
    };
    ui::text_line(label, SOURCE_ROW_HEIGHT)
        .text_color(text_color)
        .width(SOURCE_ROLE_BADGE_WIDTH)
        .height(SOURCE_ROW_HEIGHT)
}

fn source_marker_color(source: &SourceRowViewModel) -> Option<ui::Rgba8> {
    if source.missing {
        return Some(SOURCE_MISSING_COLOR);
    }
    match source.role {
        SourceRole::Protected => Some(SOURCE_ROLE_PROTECTED_COLOR),
        SourceRole::Primary => Some(SOURCE_ROLE_PRIMARY_COLOR),
        SourceRole::Normal => None,
    }
}

pub(super) fn source_missing_color() -> ui::Rgba8 {
    SOURCE_MISSING_COLOR
}

#[cfg(test)]
pub(super) fn source_missing_color_for_tests() -> ui::Rgba8 {
    source_missing_color()
}

static SOURCE_ADD_ICON: ui::SvgIconTintCache = ui::SvgIconTintCache::new(
    r#"<svg viewBox="0 0 16 16" xmlns="http://www.w3.org/2000/svg">
  <rect x="4" y="7.25" width="8" height="1.5"/>
  <rect x="7.25" y="4" width="1.5" height="8"/>
</svg>"#,
);
