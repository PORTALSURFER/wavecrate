use radiant::prelude as ui;

use crate::native_app::app::{GuiMessage, NativeAppState, NativeFileDropHover};
use crate::native_app::waveform::{self, WaveformInteraction, WaveformState};

pub(in crate::native_app) const WAVEFORM_VIEW_HEIGHT: f32 = 172.0;
pub(in crate::native_app) const WAVEFORM_PANEL_HEIGHT: f32 = 226.0;

pub(in crate::native_app) struct WaveformPanelViewModel<'a> {
    waveform: &'a WaveformState,
    loading_label: Option<&'a str>,
    loading_progress: f32,
    drop_hover: Option<&'a NativeFileDropHover>,
    block_input_while_loading: bool,
}

impl<'a> WaveformPanelViewModel<'a> {
    pub(in crate::native_app) fn from_app_state(state: &'a NativeAppState) -> Self {
        let loading_label = state.waveform_loading_label.as_deref();
        Self {
            waveform: &state.waveform,
            loading_label,
            loading_progress: state.waveform_loading_progress,
            drop_hover: state.native_file_drop_hover.as_ref(),
            block_input_while_loading: loading_label.is_some()
                && !state.folder_browser.drag_active(),
        }
    }
}

pub(in crate::native_app) fn waveform_panel(
    model: WaveformPanelViewModel<'_>,
) -> ui::View<GuiMessage> {
    ui::column([
        waveform_panel_header(model.waveform),
        ui::text_line(waveform_title(model.waveform), 18.0),
        waveform_viewport_with_loading_state(&model),
        waveform_scrollbar(model.waveform),
    ])
    .spacing(2.0)
    .style(ui::WidgetStyle::default())
    .fill_width()
    .height(WAVEFORM_PANEL_HEIGHT)
}

fn waveform_panel_header(_waveform: &WaveformState) -> ui::View<GuiMessage> {
    ui::text_line("Waveform", 18.0)
}

fn waveform_viewport_with_loading_state(
    model: &WaveformPanelViewModel<'_>,
) -> ui::View<GuiMessage> {
    let viewport = waveform::waveform_viewport_view(model.waveform)
        .fill_width()
        .height(WAVEFORM_VIEW_HEIGHT);
    let mut layers = vec![viewport];
    if let Some(hover) = model.drop_hover {
        layers.push(waveform_drop_hover_visual(hover.supported));
    }
    if let Some(label) = model.loading_label {
        layers.push(waveform_loading_visual(label, model.loading_progress));
        if model.block_input_while_loading {
            layers.push(
                ui::pointer_shield(true)
                    .view()
                    .key("waveform-loading-input-blocker")
                    .input_only()
                    .fill_width()
                    .height(WAVEFORM_VIEW_HEIGHT),
            );
        }
    }
    ui::stack_layers(layers)
        .fill_width()
        .height(WAVEFORM_VIEW_HEIGHT)
}

#[cfg_attr(test, allow(dead_code))]
pub(crate) fn waveform_loading_visual(_label: &str, progress: f32) -> ui::View<GuiMessage> {
    ui::feedback_overlay()
        .background(ui::Rgba8::new(22, 24, 25, 72))
        .progress(progress, ui::Rgba8::new(174, 178, 181, 118))
        .view()
        .key("waveform-loading-visual")
        .fill_width()
        .height(WAVEFORM_VIEW_HEIGHT)
}

fn waveform_drop_hover_visual(supported: bool) -> ui::View<GuiMessage> {
    let color = if supported {
        ui::Rgba8::new(74, 178, 116, 255)
    } else {
        ui::Rgba8::new(214, 62, 62, 255)
    };
    ui::feedback_overlay()
        .background(color.with_alpha(56))
        .edge(
            color.with_alpha(210),
            3.0,
            ui::BorderSides {
                top: true,
                bottom: true,
                left: false,
                right: false,
            },
        )
        .view()
        .key("waveform-drop-hover-visual")
        .fill_width()
        .height(WAVEFORM_VIEW_HEIGHT)
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
        return ui::empty().fill_width();
    }
    ui::scrollbar(ui::ScrollbarAxis::Horizontal)
        .viewport_fraction(waveform.visible_fraction())
        .offset_fraction(waveform.offset_fraction())
        .message(|offset_fraction| {
            GuiMessage::Waveform(WaveformInteraction::ScrollTo { offset_fraction })
        })
        .fill_width()
        .height(6.0)
}
