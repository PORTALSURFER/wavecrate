use radiant::{gui::list as list_ui, prelude as ui};

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
pub(super) const SOURCE_ROW_HEIGHT: f32 = 22.0;
const SOURCE_ROLE_ICON_WIDTH: f32 = 32.0;
const SOURCE_MISSING_BADGE_WIDTH: f32 = 56.0;
const SOURCE_MISSING_COLOR: ui::Rgba8 = ui::Rgba8::new(255, 112, 86, 230);
const SOURCE_ROLE_ICON_COLOR: ui::Rgba8 = ui::Rgba8::new(255, 255, 255, 255);
const SOURCE_PROTECTED_ERROR_ICON_COLOR: ui::Rgba8 = ui::Rgba8::new(255, 69, 54, 255);
const SOURCE_PROTECTED_ERROR_FILL: ui::Rgba8 = ui::Rgba8::new(255, 69, 54, 145);
const SOURCE_PROTECTED_ERROR_HOVER_FILL: ui::Rgba8 = ui::Rgba8::new(255, 82, 62, 175);
const SOURCE_ROW_OUTLINE_INSET: f32 = 0.5;
const SOURCE_ROW_OUTLINE_WIDTH: f32 = 1.0;
const SOURCE_ROW_OUTLINE_COLOR: ui::Rgba8 = ui::Rgba8::new(255, 255, 255, 30);
const SOURCE_ADD_BUTTON_TOOLTIP: &str = "Add source";

pub(super) fn source_add_button(help_tooltips_enabled: bool) -> ui::View<GuiMessage> {
    ui::icon_button(source_add_icon())
        .message(GuiMessage::FolderBrowser(FolderBrowserMessage::AddSource))
        .id(AUTOMATION_SOURCE_ADD_BUTTON_ID)
        .size(SOURCE_ADD_BUTTON_WIDTH, SOURCE_ADD_BUTTON_HEIGHT)
        .tooltip_if(help_tooltips_enabled, SOURCE_ADD_BUTTON_TOOLTIP)
}

fn source_add_icon() -> ui::SvgIcon {
    SOURCE_ADD_ICON.icon(toolbar_icon_color(true, false))
}

pub(super) fn source_row(source: &SourceRowViewModel) -> ui::View<GuiMessage> {
    let visual = source_row_content(source);
    let row = sidebar_row_underlay(visual)
        .tracked_drop_candidate(
            source.drag_active,
            source.drop_target,
            source.drop_candidate,
            source.drop_target_active,
        )
        .stable_row_identity(
            RETAINED_SOURCE_ROW_INPUT_SCOPE,
            retained_source_row_key(source.id.as_str()),
        )
        .selected(source.selected || source.protected_source_error_flash)
        .outline(source_row_outline());
    let row = if source.protected_source_error_flash {
        row.dense_chrome_palette(source_protected_error_palette())
    } else {
        row
    };
    row.actions(
        ui::row_actions()
            .primary_secondary_key(
                source.id.clone(),
                |source_id| {
                    GuiMessage::FolderBrowser(FolderBrowserMessage::SelectSource(source_id))
                },
                |source_id, position| {
                    GuiMessage::FolderBrowser(FolderBrowserMessage::OpenSourceContextMenu(
                        source_id, position,
                    ))
                },
            )
            .tracked_drop_candidate_key(
                source.id.clone(),
                |source_id| {
                    GuiMessage::FolderBrowser(FolderBrowserMessage::DropOnSource(source_id))
                },
                |source_id, position| {
                    GuiMessage::FolderBrowser(FolderBrowserMessage::HoverSourceDropTarget(
                        source_id, position,
                    ))
                },
                |source_id, position| {
                    GuiMessage::FolderBrowser(FolderBrowserMessage::ClearSourceDropTargetUnless(
                        source_id, position,
                    ))
                },
            ),
    )
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
        source_label(source),
        source_status_indicator(source),
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

fn source_status_indicator(source: &SourceRowViewModel) -> ui::View<GuiMessage> {
    if source.missing {
        return ui::text_line("MISSING", SOURCE_ROW_HEIGHT)
            .text_color(ui::TextColorRole::Custom(SOURCE_MISSING_COLOR))
            .width(SOURCE_MISSING_BADGE_WIDTH)
            .height(SOURCE_ROW_HEIGHT);
    }
    match source.role {
        SourceRole::Protected => {
            source_role_icon(&SOURCE_ROLE_PROTECTED_ICON, source_role_icon_color(source))
        }
        SourceRole::Primary => {
            source_role_icon(&SOURCE_ROLE_PRIMARY_ICON, source_role_icon_color(source))
        }
        SourceRole::Normal => ui::spacer()
            .width(SOURCE_ROLE_ICON_WIDTH)
            .height(SOURCE_ROW_HEIGHT),
    }
}

fn source_role_icon_color(source: &SourceRowViewModel) -> ui::Rgba8 {
    if source.role == SourceRole::Protected && source.protected_source_error_flash {
        SOURCE_PROTECTED_ERROR_ICON_COLOR
    } else {
        SOURCE_ROLE_ICON_COLOR
    }
}

fn source_role_icon(
    cache: &'static ui::SvgIconTintCache,
    color: ui::Rgba8,
) -> ui::View<GuiMessage> {
    ui::icon_button(cache.icon(color))
        .bare()
        .passive()
        .width(SOURCE_ROLE_ICON_WIDTH)
        .height(SOURCE_ROW_HEIGHT)
}

fn source_row_outline() -> list_ui::DenseRowOutlineStyle {
    list_ui::DenseRowOutlineStyle::new(
        SOURCE_ROW_OUTLINE_INSET,
        SOURCE_ROW_OUTLINE_COLOR,
        SOURCE_ROW_OUTLINE_WIDTH,
    )
}

fn source_protected_error_palette() -> list_ui::DenseRowPalette {
    list_ui::DenseRowPalette::new()
        .selected(SOURCE_PROTECTED_ERROR_FILL)
        .selected_hovered(SOURCE_PROTECTED_ERROR_HOVER_FILL)
        .interaction_fills(
            SOURCE_PROTECTED_ERROR_HOVER_FILL,
            SOURCE_PROTECTED_ERROR_HOVER_FILL,
        )
}

pub(super) fn source_missing_color() -> ui::Rgba8 {
    SOURCE_MISSING_COLOR
}

#[cfg(test)]
pub(super) fn source_missing_color_for_tests() -> ui::Rgba8 {
    source_missing_color()
}

#[cfg(test)]
pub(super) fn source_role_icon_color_for_tests() -> ui::Rgba8 {
    SOURCE_ROLE_ICON_COLOR
}

#[cfg(test)]
pub(super) fn source_protected_error_icon_color_for_tests() -> ui::Rgba8 {
    SOURCE_PROTECTED_ERROR_ICON_COLOR
}

#[cfg(test)]
pub(super) fn source_role_icon_color_for_source_for_tests(
    source: &SourceRowViewModel,
) -> ui::Rgba8 {
    source_role_icon_color(source)
}

#[cfg(test)]
pub(super) fn source_row_outline_for_tests() -> list_ui::DenseRowOutlineStyle {
    source_row_outline()
}

#[cfg(test)]
pub(super) fn source_add_button_tooltip_for_tests() -> &'static str {
    SOURCE_ADD_BUTTON_TOOLTIP
}

static SOURCE_ADD_ICON: ui::SvgIconTintCache = ui::SvgIconTintCache::new(
    r#"<svg viewBox="0 0 16 16" xmlns="http://www.w3.org/2000/svg">
  <rect x="4" y="7.25" width="8" height="1.5"/>
  <rect x="7.25" y="4" width="1.5" height="8"/>
</svg>"#,
);

static SOURCE_ROLE_PRIMARY_ICON: ui::SvgIconTintCache = ui::SvgIconTintCache::new(
    r#"<svg viewBox="0 0 16 16" xmlns="http://www.w3.org/2000/svg">
  <path d="M2.25 7.15 8 2.35l5.75 4.8v1.95L8 4.25 2.25 9.1z" fill="currentColor"/>
  <path d="M4 8.15h8v5.6H9.55V10.1h-3.1v3.65H4z" fill="currentColor"/>
</svg>"#,
);

static SOURCE_ROLE_PROTECTED_ICON: ui::SvgIconTintCache = ui::SvgIconTintCache::new(
    r#"<svg viewBox="0 0 16 16" xmlns="http://www.w3.org/2000/svg">
  <path d="M4.1 7.1V5.6C4.1 3.45 5.65 2 8 2s3.9 1.45 3.9 3.6v1.5" fill="none" stroke="currentColor" stroke-width="1.35" stroke-linecap="round"/>
  <rect x="3" y="6.75" width="10" height="7" rx="1.2" fill="currentColor"/>
  <rect x="7.3" y="9" width="1.4" height="2.7" rx=".55" fill="rgb(24,24,24)"/>
</svg>"#,
);
