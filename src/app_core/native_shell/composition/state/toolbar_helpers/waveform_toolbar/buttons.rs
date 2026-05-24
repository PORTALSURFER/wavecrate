use super::*;

mod specs;

use specs::{WaveformToolbarButtonSpec, waveform_toolbar_button_specs};

pub(in crate::app_core::native_shell::composition::state) fn waveform_toolbar_buttons(
    layout: &ShellLayout,
    style: &StyleTokens,
    model: &NativeMotionModel,
    bpm_input_active: bool,
    bpm_input_display: Option<&str>,
) -> Vec<WaveformToolbarButton> {
    let specs = waveform_toolbar_button_specs(style, model, bpm_input_active, bpm_input_display);
    let content = waveform_toolbar_surface_content(&specs);
    let surface_layout =
        resolve_waveform_toolbar_surface_layout(layout.waveform_header, style.sizing, &content);

    surface_layout
        .item_rects
        .iter()
        .copied()
        .zip(specs)
        .filter(|(rect, _)| rect.width() > 1.0 && rect.height() > 1.0)
        .map(|(rect, spec)| spec.into_button(rect))
        .collect()
}

fn waveform_toolbar_surface_content(
    specs: &[WaveformToolbarButtonSpec],
) -> WaveformToolbarSurfaceContent {
    WaveformToolbarSurfaceContent {
        items: specs
            .iter()
            .map(|spec| WaveformToolbarSurfaceItem {
                label: spec.label.to_string(),
                kind: waveform_toolbar_surface_item_kind(spec.label),
                value: spec.display_text.clone(),
                enabled: spec.enabled,
                active: spec.active,
            })
            .collect(),
    }
}

fn waveform_toolbar_surface_item_kind(label: &str) -> WaveformToolbarSurfaceItemKind {
    match label {
        "BPM Value" => WaveformToolbarSurfaceItemKind::TextInput,
        "Channel" | "Norm" | "BPM Snap" | "Rel Grid" | "Tr Snap" | "Show Tr" | "Slice" | "Loop" => {
            WaveformToolbarSurfaceItemKind::Toggle
        }
        _ => WaveformToolbarSurfaceItemKind::Button,
    }
}
