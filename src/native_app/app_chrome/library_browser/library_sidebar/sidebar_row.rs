#[cfg(test)]
use radiant::gui::list as list_ui;
use radiant::prelude as ui;

use crate::native_app::app::GuiMessage;
#[cfg(test)]
use crate::native_app::app_chrome::palette::selected_row_palette;
use crate::native_app::app_chrome::palette::{ListItemState, WavecrateListRowStyle};

pub(super) const SIDEBAR_ROW_STYLE: ui::WidgetStyle =
    ui::WidgetStyle::subtle(ui::WidgetTone::Accent);

pub(super) fn sidebar_row_underlay(
    content: ui::View<GuiMessage>,
    state: ListItemState,
) -> ui::InteractiveRowUnderlayBuilder<GuiMessage> {
    ui::interactive_row_underlay(content)
        .custom_paint_hit_target()
        .style(SIDEBAR_ROW_STYLE)
        .wavecrate_list_row_style(SIDEBAR_ROW_STYLE, state)
        .dense_chrome()
}

#[cfg(test)]
pub(super) fn sidebar_row_full_palette(theme: &ui::ThemeTokens) -> ui::DenseRowPalette {
    let mut palette = list_ui::dense_row_palette_from_style(theme, SIDEBAR_ROW_STYLE);
    let selected = selected_row_palette(SIDEBAR_ROW_STYLE);
    palette.selected = selected.selected;
    palette.selected_hovered = selected.selected_hovered;
    palette
}

#[cfg(test)]
pub(super) fn sidebar_row_palette_for_tests() -> ui::DenseRowPalette {
    sidebar_row_full_palette(&ui::ThemeTokens::default())
}

#[cfg(test)]
pub(super) fn sidebar_row_hover_fill_for_tests() -> ui::Rgba8 {
    sidebar_row_palette_for_tests()
        .hovered
        .expect("dense-row hover fill")
}

#[cfg(test)]
pub(super) fn sidebar_row_selected_fill_for_tests() -> ui::Rgba8 {
    sidebar_row_palette_for_tests()
        .selected
        .expect("dense-row selected fill")
}

#[cfg(test)]
pub(super) fn sidebar_row_active_target_fill_for_tests() -> ui::Rgba8 {
    sidebar_row_palette_for_tests()
        .active_target
        .expect("dense-row active-target fill")
}

#[cfg(test)]
mod tests {
    use super::*;
    use radiant::prelude::IntoView;

    #[test]
    fn selected_sidebar_row_paints_orange_highlight_fill() {
        let frame = sidebar_row_underlay(
            ui::text_line("Source", 22.0),
            ListItemState::new(true, false),
        )
        .selected(true)
        .actions(ui::row_actions())
        .view_frame_at_size_with_default_theme(ui::Vector2::new(120.0, 22.0));

        assert!(
            frame
                .paint_plan
                .fill_rects()
                .any(|fill| fill.color == sidebar_row_selected_fill_for_tests()),
            "selected sidebar rows should use the orange selected/focused highlight"
        );
    }
}
