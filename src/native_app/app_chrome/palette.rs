//! Wavecrate-specific colors layered over Radiant's shared dark theme.

use radiant::prelude::{
    DenseRowMarkerStyle, DenseRowOutlineStyle, DenseRowPalette, Rgba8, WidgetStyle,
};

pub(in crate::native_app) const ACCENT: Rgba8 = Rgba8::new(233, 88, 67, 255);
pub(in crate::native_app) const ACCENT_SOFT: Rgba8 = Rgba8::new(241, 121, 98, 255);
pub(in crate::native_app) const DANGER: Rgba8 = Rgba8::new(239, 76, 61, 255);
pub(in crate::native_app) const WARNING: Rgba8 = Rgba8::new(217, 151, 95, 255);
pub(in crate::native_app) const TEXT_PRIMARY: Rgba8 = Rgba8::new(216, 215, 211, 255);
pub(in crate::native_app) const TEXT_MUTED: Rgba8 = Rgba8::new(153, 155, 154, 255);
pub(in crate::native_app) const COOL_SELECTION: Rgba8 = Rgba8::new(174, 176, 173, 255);
pub(in crate::native_app) const PALE_MARKER: Rgba8 = Rgba8::new(231, 229, 223, 255);
pub(in crate::native_app) const SELECTED_ROW_FILL: Rgba8 = ACCENT.with_alpha(18);
pub(in crate::native_app) const SELECTED_ROW_HOVER_FILL: Rgba8 = ACCENT.with_alpha(28);
pub(in crate::native_app) const SELECTED_ROW_MARKER_WIDTH: f32 = 3.0;
pub(in crate::native_app) const LIST_ROW_FOCUS_OUTLINE_WIDTH: f32 = 1.0;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(in crate::native_app) struct ListItemState {
    pub(in crate::native_app) selected: bool,
    pub(in crate::native_app) focused: bool,
}

impl ListItemState {
    pub(in crate::native_app) const fn new(selected: bool, focused: bool) -> Self {
        Self { selected, focused }
    }
}

pub(in crate::native_app) fn selected_row_palette(style: WidgetStyle) -> DenseRowPalette {
    let mut palette = radiant::gui::list::dense_row_palette_from_style(
        &radiant::prelude::ThemeTokens::default(),
        style,
    );
    palette.selected = Some(SELECTED_ROW_FILL);
    palette.selected_hovered = Some(SELECTED_ROW_HOVER_FILL);
    // Pointer-down uses the focus outline instead of Radiant's opaque pressed
    // fill. A selected row falls back to its quiet persistent selection fill.
    palette.pressed = None;
    palette
}

pub(in crate::native_app) fn selected_row_marker() -> DenseRowMarkerStyle {
    DenseRowMarkerStyle::new(
        radiant::gui::list::DenseRowMarkerParts::leading(SELECTED_ROW_MARKER_WIDTH)
            .edge_inset(0.0)
            .vertical_inset(0.0),
        ACCENT,
    )
}

pub(in crate::native_app) fn selected_row_trailing_marker() -> DenseRowMarkerStyle {
    DenseRowMarkerStyle::new(
        radiant::gui::list::DenseRowMarkerParts::trailing(SELECTED_ROW_MARKER_WIDTH)
            .edge_inset(0.0)
            .vertical_inset(0.0),
        ACCENT,
    )
}

pub(in crate::native_app) fn hovered_row_trailing_marker() -> DenseRowMarkerStyle {
    DenseRowMarkerStyle::new(
        radiant::gui::list::DenseRowMarkerParts::trailing(SELECTED_ROW_MARKER_WIDTH)
            .edge_inset(0.0)
            .vertical_inset(0.0),
        PALE_MARKER.with_alpha(180),
    )
}

pub(in crate::native_app) fn focused_row_outline() -> DenseRowOutlineStyle {
    // The outline is deliberately opaque and inset by half its width. Radiant
    // paints outlines after row markers, so its leading edge remains a crisp
    // pale rail on top of the thicker coral selection rail when both states
    // are active.
    DenseRowOutlineStyle::new(0.5, PALE_MARKER, LIST_ROW_FOCUS_OUTLINE_WIDTH)
}

pub(in crate::native_app) trait WavecrateListRowStyle<Message: 'static> {
    /// Apply the common interaction layer. Row-specific semantic states such
    /// as keep, anchor, cached, processing, warning, or error may override the
    /// palette or individual markers afterward without replacing focus state.
    fn wavecrate_list_row_style(self, style: WidgetStyle, state: ListItemState) -> Self;
}

impl<Message: 'static> WavecrateListRowStyle<Message>
    for radiant::application::InteractiveRowUnderlayBuilder<Message>
{
    fn wavecrate_list_row_style(self, style: WidgetStyle, state: ListItemState) -> Self {
        self.dense_chrome_palette(selected_row_palette(style))
            .leading_marker_if(state.selected, selected_row_marker())
            .trailing_marker_if(
                state.selected || state.focused,
                selected_row_trailing_marker(),
            )
            .hover_trailing_marker(hovered_row_trailing_marker())
            .pressed_outline(focused_row_outline())
            .outline_if(state.focused, focused_row_outline())
    }
}

pub(in crate::native_app) trait WavecrateTreeRowStyle {
    fn wavecrate_tree_row_style(self, style: WidgetStyle) -> Self;
}

impl WavecrateTreeRowStyle for radiant::application::TreeRowBuilder {
    fn wavecrate_tree_row_style(self, style: WidgetStyle) -> Self {
        self.palette(selected_row_palette(style))
            .selected_marker(selected_row_marker())
            .selected_trailing_marker(selected_row_trailing_marker())
            .hover_trailing_marker(hovered_row_trailing_marker())
            .focus_outline(focused_row_outline())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn app_palette_matches_the_editorial_terminal_theme() {
        let theme = radiant::theme::ThemeTokens::default();

        assert_eq!(ACCENT, theme.accent_mint);
        assert_eq!(DANGER, theme.accent_danger);
        assert_eq!(WARNING, theme.accent_warning);
        assert_eq!(TEXT_PRIMARY, theme.text_primary);
        assert_eq!(TEXT_MUTED, theme.text_muted);
    }

    #[test]
    fn selected_rows_share_one_quiet_fill_and_leading_rail() {
        let palette =
            selected_row_palette(WidgetStyle::subtle(radiant::prelude::WidgetTone::Accent));
        let marker = selected_row_marker();
        let trailing = selected_row_trailing_marker();
        let hover = hovered_row_trailing_marker();
        let focus = focused_row_outline();

        assert_eq!(palette.selected, Some(SELECTED_ROW_FILL));
        assert_eq!(palette.selected_hovered, Some(SELECTED_ROW_HOVER_FILL));
        assert_eq!(marker.color, ACCENT);
        assert_eq!(marker.parts.width, SELECTED_ROW_MARKER_WIDTH);
        assert_eq!(marker.parts.vertical_inset, 0.0);
        assert_eq!(trailing.parts.width, SELECTED_ROW_MARKER_WIDTH);
        assert_eq!(trailing.color, ACCENT);
        assert_eq!(hover.color, PALE_MARKER.with_alpha(180));
        assert_eq!(focus.width, LIST_ROW_FOCUS_OUTLINE_WIDTH);
        assert_eq!(focus.color, PALE_MARKER);
    }
}
