//! Waveform toolbar layout and button rendering helpers.

mod buttons;
mod render;

use super::super::*;

pub(in crate::app_core::native_shell::composition::state) use self::{
    buttons::waveform_toolbar_buttons,
    render::{WaveformToolbarRenderContext, render_waveform_toolbar_buttons},
};

pub(in crate::app_core::native_shell::composition::state) fn waveform_toolbar_bpm_value_label(
    model: &NativeMotionModel,
    bpm_input_display: Option<&str>,
) -> String {
    if let Some(display) = bpm_input_display {
        return display.to_string();
    }
    model
        .waveform_presentation()
        .primary_label
        .as_deref()
        .and_then(parse_waveform_tempo_number_text)
        .unwrap_or_else(|| String::from("120.0"))
}

fn parse_waveform_tempo_number_text(label: &str) -> Option<String> {
    let number = label.split_ascii_whitespace().next()?.trim();
    if number.is_empty() {
        return None;
    }
    let parsed = number.parse::<f32>().ok()?;
    if !parsed.is_finite() || parsed <= 0.0 {
        return None;
    }
    Some(number.to_string())
}

pub(in crate::app_core::native_shell::composition::state) fn waveform_toolbar_left_edge(
    buttons: &[WaveformToolbarButton],
    fallback: f32,
) -> f32 {
    buttons
        .iter()
        .map(|button| button.rect.min.x)
        .min_by(f32::total_cmp)
        .unwrap_or(fallback)
}
