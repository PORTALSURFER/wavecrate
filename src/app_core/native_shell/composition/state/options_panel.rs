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

#[derive(Clone, Copy, Debug, PartialEq)]
pub(super) struct OptionsPanelDrag {
    pub(super) grab_offset_x: f32,
    pub(super) grab_offset_y: f32,
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
    geometry::options_panel_layout(layout, style, model, None)
}

pub(super) fn options_panel_layout_with_origin(
    layout: &ShellLayout,
    style: &StyleTokens,
    model: &AppModel,
    origin: Option<Point>,
) -> Option<OptionsPanelLayout> {
    geometry::options_panel_layout(layout, style, model, origin)
}

pub(super) fn options_panel_contains_point(
    layout: &ShellLayout,
    style: &StyleTokens,
    model: &AppModel,
    point: Point,
) -> bool {
    geometry::options_panel_contains_point(layout, style, model, None, point)
}

pub(super) fn options_panel_contains_point_with_origin(
    layout: &ShellLayout,
    style: &StyleTokens,
    model: &AppModel,
    origin: Option<Point>,
    point: Point,
) -> bool {
    geometry::options_panel_contains_point(layout, style, model, origin, point)
}

pub(super) fn options_panel_action_at_point(
    layout: &ShellLayout,
    style: &StyleTokens,
    model: &AppModel,
    point: Point,
) -> Option<UiAction> {
    geometry::options_panel_action_at_point(layout, style, model, None, point)
}

pub(super) fn options_panel_action_at_point_with_origin(
    layout: &ShellLayout,
    style: &StyleTokens,
    model: &AppModel,
    origin: Option<Point>,
    point: Point,
) -> Option<UiAction> {
    geometry::options_panel_action_at_point(layout, style, model, origin, point)
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
    origin: Option<Point>,
) {
    render::render_options_panel(primitives, text_runs, layout, style, model, origin);
}

#[cfg(test)]
pub(super) fn options_panel_layout_for_origin(
    layout: &ShellLayout,
    style: &StyleTokens,
    model: &AppModel,
    origin: Option<Point>,
) -> Option<OptionsPanelLayout> {
    options_panel_layout_with_origin(layout, style, model, origin)
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

impl NativeShellState {
    pub(crate) fn begin_options_panel_drag(
        &mut self,
        layout: &ShellLayout,
        model: &AppModel,
        point: Point,
    ) -> bool {
        let style = style_for_layout(layout);
        let Some(panel) =
            options_panel_layout_with_origin(layout, &style, model, self.options_panel_origin)
        else {
            self.options_panel_drag = None;
            return false;
        };
        if panel
            .buttons
            .iter()
            .any(|button| button.rect.contains(point))
        {
            self.options_panel_drag = None;
            return false;
        }
        if !panel.title_rect.contains(point) {
            self.options_panel_drag = None;
            return false;
        }
        self.options_panel_origin = Some(panel.panel_rect.min);
        self.options_panel_drag = Some(OptionsPanelDrag {
            grab_offset_x: point.x - panel.panel_rect.min.x,
            grab_offset_y: point.y - panel.panel_rect.min.y,
        });
        true
    }

    pub(crate) fn update_options_panel_drag(
        &mut self,
        layout: &ShellLayout,
        model: &AppModel,
        point: Point,
    ) -> bool {
        let Some(drag) = self.options_panel_drag else {
            return false;
        };
        let requested = Point::new(point.x - drag.grab_offset_x, point.y - drag.grab_offset_y);
        self.options_panel_origin = Some(requested);
        let style = style_for_layout(layout);
        if let Some(panel) =
            options_panel_layout_with_origin(layout, &style, model, self.options_panel_origin)
        {
            self.options_panel_origin = Some(panel.panel_rect.min);
        }
        true
    }

    pub(crate) fn finish_options_panel_drag(&mut self) -> bool {
        self.options_panel_drag.take().is_some()
    }

    fn options_panel_layout_live(
        &self,
        layout: &ShellLayout,
        model: &AppModel,
    ) -> Option<OptionsPanelLayout> {
        let style = style_for_layout(layout);
        options_panel_layout_with_origin(layout, &style, model, self.options_panel_origin)
    }

    pub(crate) fn options_panel_title_rect_live(
        &self,
        layout: &ShellLayout,
        model: &AppModel,
    ) -> Option<Rect> {
        self.options_panel_layout_live(layout, model)
            .map(|panel| panel.title_rect)
    }
}
