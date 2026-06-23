use radiant::prelude as ui;

use crate::native_app::app::GuiMessage;
use crate::native_app::app_chrome::view_models::waveform_panel::WaveformPanelViewModel;
use crate::native_app::ui::ids as widget_ids;
use crate::native_app::waveform::{self, WaveformInteraction, WaveformState};

const WAVEFORM_STATUS_HEIGHT: f32 = 16.0;
const WAVEFORM_SAMPLE_DRAG_HANDLE_WIDTH: f32 = 14.0;
pub(in crate::native_app) const WAVEFORM_VIEW_HEIGHT: f32 = 172.0;
pub(in crate::native_app) const WAVEFORM_PANEL_HEIGHT: f32 = 202.0;

pub(in crate::native_app) fn waveform_panel(
    model: WaveformPanelViewModel<'_>,
) -> ui::View<GuiMessage> {
    ui::column([
        waveform_title_row(model.waveform, model.loading_label),
        waveform_viewport_with_loading_state(&model),
        waveform_scrollbar(model.waveform),
    ])
    .spacing(1.0)
    .style(ui::WidgetStyle::default())
    .fill_width()
    .height(WAVEFORM_PANEL_HEIGHT)
}

fn waveform_viewport_with_loading_state(
    model: &WaveformPanelViewModel<'_>,
) -> ui::View<GuiMessage> {
    let tooltip = model.help_tooltips_enabled.then_some(
        "Waveform: click to set playback start, drag to select, Z zooms to selection, X zooms out.",
    );
    let viewport = waveform::waveform_viewport_view_with_tooltip(
        model.waveform,
        tooltip,
        model.beat_guides_enabled,
        model.beat_guide_count,
    )
    .fill_width()
    .height(WAVEFORM_VIEW_HEIGHT);
    ui::overlay_stack(viewport)
        .overlay_opt(
            model
                .drop_hover
                .map(|hover| waveform_drop_hover_visual(hover.supported)),
        )
        .input_opt(waveform_loading_input_blocker(model))
        .into_view()
        .accepts_native_file_drop()
        .on_native_file_drop(GuiMessage::WaveformFileDrop)
        .fill_width()
        .height(WAVEFORM_VIEW_HEIGHT)
}

fn waveform_loading_input_blocker(
    model: &WaveformPanelViewModel<'_>,
) -> Option<ui::View<GuiMessage>> {
    model.block_input_while_loading.then(|| {
        ui::pointer_shield(true)
            .consume()
            .key("waveform-loading-input-blocker")
            .input_only()
            .fill_width()
            .height(WAVEFORM_VIEW_HEIGHT)
    })
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

fn waveform_title_row(
    waveform: &WaveformState,
    loading_label: Option<&str>,
) -> ui::View<GuiMessage> {
    let title = waveform_title(waveform, loading_label);
    if loading_label.is_some() || !waveform.has_loaded_sample() {
        return ui::text_line(title, WAVEFORM_STATUS_HEIGHT);
    }
    ui::row([
        loaded_sample_drag_handle(),
        ui::text_line(title, WAVEFORM_STATUS_HEIGHT),
    ])
    .spacing(3.0)
    .fill_width()
    .height(WAVEFORM_STATUS_HEIGHT)
}

fn loaded_sample_drag_handle() -> ui::View<GuiMessage> {
    ui::stack([
        ui::card()
            .style(ui::WidgetStyle::subtle(ui::WidgetTone::Accent))
            .id(widget_ids::WAVEFORM_LOADED_SAMPLE_DRAG_HANDLE_VISUAL_ID)
            .size(WAVEFORM_SAMPLE_DRAG_HANDLE_WIDTH, WAVEFORM_STATUS_HEIGHT),
        ui::drag_handle()
            .mapped(|drag| GuiMessage::Waveform(WaveformInteraction::DragLoadedSample(drag)))
            .id(widget_ids::WAVEFORM_LOADED_SAMPLE_DRAG_HANDLE_ID)
            .style(ui::WidgetStyle::subtle(ui::WidgetTone::Accent))
            .size(WAVEFORM_SAMPLE_DRAG_HANDLE_WIDTH, WAVEFORM_STATUS_HEIGHT)
            .tooltip("Drag loaded sample"),
    ])
    .key("waveform-loaded-sample-drag-shell")
    .size(WAVEFORM_SAMPLE_DRAG_HANDLE_WIDTH, WAVEFORM_STATUS_HEIGHT)
}

fn waveform_title(waveform: &WaveformState, loading_label: Option<&str>) -> String {
    if let Some(label) = loading_label {
        return format!("Loading {label}");
    }
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
