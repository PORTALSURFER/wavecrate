use radiant::gui::types::{Point, Rect, Rgba8};
use radiant::layout::{LayoutOutput, Vector2};
use radiant::prelude as ui;
use radiant::runtime::{PaintFillRect, PaintPrimitive};
use radiant::theme::ThemeTokens;
use radiant::widgets::{
    FocusBehavior, Widget, WidgetCommon, WidgetInput, WidgetOutput, WidgetSizing,
};

use super::waveform::{self, WaveformInteraction, WaveformState};
use super::{GuiAppState, GuiMessage, WAVEFORM_PANEL_HEIGHT, WAVEFORM_VIEW_HEIGHT};

pub(super) fn waveform_panel(state: &GuiAppState) -> ui::View<GuiMessage> {
    ui::column([
        waveform_panel_header(&state.waveform),
        ui::text(waveform_title(&state.waveform))
            .height(18.0)
            .fill_width()
            .truncate(),
        waveform_viewport_with_loading_state(state),
        waveform_scrollbar(&state.waveform),
    ])
    .spacing(2.0)
    .style(ui::WidgetStyle::default())
    .fill_width()
    .height(WAVEFORM_PANEL_HEIGHT)
}

fn waveform_panel_header(_waveform: &WaveformState) -> ui::View<GuiMessage> {
    ui::text("Waveform").height(18.0).fill_width()
}

fn waveform_viewport_with_loading_state(state: &GuiAppState) -> ui::View<GuiMessage> {
    let viewport = waveform::waveform_viewport_view(&state.waveform)
        .fill_width()
        .height(WAVEFORM_VIEW_HEIGHT);
    let mut layers = vec![viewport];
    if let Some(hover) = state.native_file_drop_hover.as_ref() {
        layers.push(waveform_drop_hover_visual(hover.supported));
    }
    if state.waveform_loading_label.is_some() {
        layers.push(waveform_loading_visual(
            state.waveform_loading_label.as_deref().unwrap_or_default(),
            state.waveform_loading_progress,
        ));
        layers.push(
            ui::custom_widget_mapped(WaveformLoadingInputBlocker::new(), |message: GuiMessage| {
                message
            })
            .key("waveform-loading-input-blocker")
            .input_only()
            .fill_width()
            .height(WAVEFORM_VIEW_HEIGHT),
        );
    }
    if layers.len() == 1 {
        layers.pop().expect("viewport layer")
    } else {
        ui::stack(layers).fill_width().height(WAVEFORM_VIEW_HEIGHT)
    }
}

#[cfg_attr(test, allow(dead_code))]
pub(crate) fn waveform_loading_visual(_label: &str, progress: f32) -> ui::View<GuiMessage> {
    ui::custom_widget(WaveformLoadingVisual::new(progress), |_| None)
        .key("waveform-loading-visual")
        .fill_width()
        .height(WAVEFORM_VIEW_HEIGHT)
}

fn waveform_drop_hover_visual(supported: bool) -> ui::View<GuiMessage> {
    ui::custom_widget(WaveformDropHoverVisual::new(supported), |_| None)
        .key("waveform-drop-hover-visual")
        .fill_width()
        .height(WAVEFORM_VIEW_HEIGHT)
}

#[derive(Clone, Debug)]
struct WaveformDropHoverVisual {
    common: WidgetCommon,
    supported: bool,
}

impl WaveformDropHoverVisual {
    fn new(supported: bool) -> Self {
        let mut common = WidgetCommon::new(0, WidgetSizing::fixed(Vector2::new(1.0, 1.0)));
        common.focus = FocusBehavior::None;
        common.paint.paints_focus = false;
        common.paint.paints_state_layers = false;
        Self { common, supported }
    }
}

impl Widget for WaveformDropHoverVisual {
    fn common(&self) -> &WidgetCommon {
        &self.common
    }

    fn common_mut(&mut self) -> &mut WidgetCommon {
        &mut self.common
    }

    fn handle_input(&mut self, _bounds: Rect, _input: WidgetInput) -> Option<WidgetOutput> {
        None
    }

    fn needs_state_synchronization(&self) -> bool {
        false
    }

    fn append_paint(
        &self,
        primitives: &mut Vec<PaintPrimitive>,
        bounds: Rect,
        _layout: &LayoutOutput,
        _theme: &ThemeTokens,
    ) {
        let (r, g, b) = if self.supported {
            (74, 178, 116)
        } else {
            (214, 62, 62)
        };
        primitives.push(PaintPrimitive::FillRect(PaintFillRect {
            widget_id: self.common.id,
            rect: bounds,
            color: Rgba8 { r, g, b, a: 56 },
        }));
        let edge = 3.0_f32.min(bounds.height().max(1.0));
        primitives.push(PaintPrimitive::FillRect(PaintFillRect {
            widget_id: self.common.id,
            rect: Rect::from_min_max(
                bounds.min,
                Point::new(bounds.max.x, (bounds.min.y + edge).min(bounds.max.y)),
            ),
            color: Rgba8 { r, g, b, a: 210 },
        }));
        primitives.push(PaintPrimitive::FillRect(PaintFillRect {
            widget_id: self.common.id,
            rect: Rect::from_min_max(
                Point::new(bounds.min.x, (bounds.max.y - edge).max(bounds.min.y)),
                bounds.max,
            ),
            color: Rgba8 { r, g, b, a: 210 },
        }));
    }
}

#[derive(Clone, Debug)]
struct WaveformLoadingVisual {
    common: WidgetCommon,
    progress: f32,
}

impl WaveformLoadingVisual {
    fn new(progress: f32) -> Self {
        let mut common = WidgetCommon::new(0, WidgetSizing::fixed(Vector2::new(1.0, 1.0)));
        common.focus = FocusBehavior::None;
        common.paint.paints_focus = false;
        common.paint.paints_state_layers = false;
        Self {
            common,
            progress: progress.clamp(0.0, 1.0),
        }
    }
}

impl Widget for WaveformLoadingVisual {
    fn common(&self) -> &WidgetCommon {
        &self.common
    }

    fn common_mut(&mut self) -> &mut WidgetCommon {
        &mut self.common
    }

    fn handle_input(&mut self, _bounds: Rect, _input: WidgetInput) -> Option<WidgetOutput> {
        None
    }

    fn needs_state_synchronization(&self) -> bool {
        false
    }

    fn append_paint(
        &self,
        primitives: &mut Vec<PaintPrimitive>,
        bounds: Rect,
        _layout: &LayoutOutput,
        _theme: &ThemeTokens,
    ) {
        primitives.push(PaintPrimitive::FillRect(PaintFillRect {
            widget_id: self.common.id,
            rect: bounds,
            color: Rgba8 {
                r: 22,
                g: 24,
                b: 25,
                a: 72,
            },
        }));

        let fill_width = bounds.width() * self.progress;
        if fill_width > 0.5 {
            primitives.push(PaintPrimitive::FillRect(PaintFillRect {
                widget_id: self.common.id,
                rect: Rect::from_min_max(
                    bounds.min,
                    Point::new((bounds.min.x + fill_width).min(bounds.max.x), bounds.max.y),
                ),
                color: Rgba8 {
                    r: 174,
                    g: 178,
                    b: 181,
                    a: 118,
                },
            }));
        }
    }
}

#[derive(Clone, Debug)]
struct WaveformLoadingInputBlocker {
    common: WidgetCommon,
}

impl WaveformLoadingInputBlocker {
    fn new() -> Self {
        let mut common = WidgetCommon::new(0, WidgetSizing::fixed(Vector2::new(1.0, 1.0)));
        common.focus = FocusBehavior::None;
        common.paint.paints_focus = false;
        common.paint.paints_state_layers = false;
        Self { common }
    }
}

impl Widget for WaveformLoadingInputBlocker {
    fn common(&self) -> &WidgetCommon {
        &self.common
    }

    fn common_mut(&mut self) -> &mut WidgetCommon {
        &mut self.common
    }

    fn handle_input(&mut self, bounds: Rect, input: WidgetInput) -> Option<WidgetOutput> {
        match input {
            WidgetInput::PointerMove { position }
            | WidgetInput::PointerPress { position, .. }
            | WidgetInput::PointerRelease { position, .. }
            | WidgetInput::PointerDrop { position, .. }
                if bounds.contains(position) =>
            {
                Some(WidgetOutput::typed(GuiMessage::Noop))
            }
            _ => None,
        }
    }

    fn accepts_pointer_move(&self) -> bool {
        true
    }

    fn append_paint(
        &self,
        _primitives: &mut Vec<PaintPrimitive>,
        _bounds: Rect,
        _layout: &LayoutOutput,
        _theme: &ThemeTokens,
    ) {
    }
}

fn waveform_title(waveform: &WaveformState) -> String {
    if !waveform.has_loaded_sample() {
        return String::from("No sample loaded");
    }
    format!(
        "{} | {} Hz | {} channel{} -> mono | {} frames",
        waveform.file_name(),
        waveform.sample_rate(),
        waveform.channels(),
        if waveform.channels() == 1 { "" } else { "s" },
        waveform.frames()
    )
}

fn waveform_scrollbar(waveform: &WaveformState) -> ui::View<GuiMessage> {
    if waveform.fully_zoomed_out() {
        return ui::text("").fill_width().height(0.0);
    }
    ui::scrollbar(ui::ScrollbarAxis::Horizontal)
        .viewport_fraction(waveform.visible_fraction())
        .offset_fraction(waveform.offset_fraction())
        .mapped(|message| match message {
            ui::ScrollbarMessage::OffsetChanged { offset_fraction } => {
                GuiMessage::Waveform(WaveformInteraction::ScrollTo { offset_fraction })
            }
        })
        .fill_width()
        .height(6.0)
}
