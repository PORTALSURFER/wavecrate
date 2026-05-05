//! Top-right options button and options-panel helpers for the native shell.

use super::*;

#[path = "options_panel/actions.rs"]
mod actions;
#[path = "options_panel/geometry.rs"]
mod geometry;
#[path = "options_panel/render.rs"]
mod render;
#[path = "options_panel/style.rs"]
mod style;

#[derive(Clone, Debug, PartialEq)]
pub(super) struct OptionsPanelButton {
    pub(super) rect: Rect,
    pub(super) text: String,
    pub(super) action: UiAction,
    pub(super) active: bool,
}

#[derive(Clone, Debug, PartialEq)]
pub(super) struct OptionsPanelLayout {
    pub(super) panel_rect: Rect,
    pub(super) title_rect: Rect,
    pub(super) detail_rect: Option<Rect>,
    pub(super) title: String,
    pub(super) buttons: Vec<OptionsPanelButton>,
}

pub(super) fn status_right_text_rect(
    segment: Rect,
    sizing: SizingTokens,
    button_rect: Option<Rect>,
) -> Rect {
    geometry::status_right_text_rect(segment, sizing, button_rect)
}

pub(super) fn options_panel_layout(
    layout: &ShellLayout,
    style: &StyleTokens,
    model: &AppModel,
) -> Option<OptionsPanelLayout> {
    geometry::options_panel_layout(layout, style, model)
}

pub(super) fn options_panel_contains_point(
    layout: &ShellLayout,
    style: &StyleTokens,
    model: &AppModel,
    point: Point,
) -> bool {
    geometry::options_panel_contains_point(layout, style, model, point)
}

pub(super) fn options_panel_action_at_point(
    layout: &ShellLayout,
    style: &StyleTokens,
    model: &AppModel,
    point: Point,
) -> Option<UiAction> {
    geometry::options_panel_action_at_point(layout, style, model, point)
}

pub(super) fn render_status_options_button(
    primitives: &mut impl PrimitiveSink,
    style: &StyleTokens,
    sizing: SizingTokens,
    button_rect: Rect,
    chip_label: &str,
    error: bool,
    hovered: bool,
    flashed: bool,
    motion_wave: f32,
) {
    render::render_status_options_button(
        primitives,
        style,
        sizing,
        button_rect,
        chip_label,
        error,
        hovered,
        flashed,
        motion_wave,
    );
}

pub(super) fn render_options_panel(
    primitives: &mut impl PrimitiveSink,
    text_runs: &mut impl TextRunSink,
    layout: &ShellLayout,
    style: &StyleTokens,
    model: &AppModel,
) {
    render::render_options_panel(primitives, text_runs, layout, style, model);
}

pub(super) fn render_status_options_button_label(
    text_runs: &mut impl TextRunSink,
    style: &StyleTokens,
    sizing: SizingTokens,
    button_rect: Rect,
    chip_label: &str,
    error: bool,
    hovered: bool,
    flashed: bool,
    motion_wave: f32,
) {
    render::render_status_options_button_label(
        text_runs,
        style,
        sizing,
        button_rect,
        chip_label,
        error,
        hovered,
        flashed,
        motion_wave,
    );
}
