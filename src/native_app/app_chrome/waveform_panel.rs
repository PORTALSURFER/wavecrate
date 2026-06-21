use radiant::prelude as ui;

use crate::native_app::app::GuiMessage;
use crate::native_app::app_chrome::view_models::waveform_panel::WaveformPanelViewModel;
use crate::native_app::waveform::{self, WaveformInteraction, WaveformState};

const WAVEFORM_STATUS_HEIGHT: f32 = 16.0;
pub(in crate::native_app) const WAVEFORM_VIEW_HEIGHT: f32 = 172.0;
pub(in crate::native_app) const WAVEFORM_PANEL_HEIGHT: f32 = 202.0;

pub(in crate::native_app) fn waveform_panel(
    model: WaveformPanelViewModel<'_>,
) -> ui::View<GuiMessage> {
    ui::column([
        ui::text_line(
            waveform_title(model.waveform, model.loading_label),
            WAVEFORM_STATUS_HEIGHT,
        ),
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
    let tooltip = model
        .help_tooltips_enabled
        .then_some("Waveform: click to set playback start, drag to select, scroll to zoom.");
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::native_app::test_support::state::NativeAppStateFixture;
    use radiant::prelude::IntoView;

    #[test]
    fn waveform_panel_omits_section_header_label() {
        let state = NativeAppStateFixture::default().build();
        let frame = waveform_panel(WaveformPanelViewModel::from_app_state(&state))
            .view_frame_at_size_with_default_theme(ui::Vector2::new(800.0, WAVEFORM_PANEL_HEIGHT));

        assert!(frame.paint_plan.contains_text("No sample loaded"));
        assert!(!frame.paint_plan.contains_text("Waveform"));
    }

    #[test]
    fn waveform_help_tooltip_attaches_to_interaction_widget() {
        let mut state = NativeAppStateFixture::default()
            .with_synthetic_waveform()
            .build();
        state.ui.chrome.help_tooltips_enabled = true;
        let surface = waveform_panel(WaveformPanelViewModel::from_app_state(&state)).into_surface();
        let tooltip = surface
            .find_widget(crate::native_app::ui::ids::WAVEFORM_WIDGET_ID)
            .and_then(|widget| widget.widget_object().common().tooltip.as_deref());

        assert_eq!(
            tooltip,
            Some("Waveform: click to set playback start, drag to select, scroll to zoom.")
        );
    }
}
