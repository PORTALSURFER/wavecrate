//! Waveform panel and waveform chrome projection helpers.

use super::*;

pub(crate) fn project_waveform_model(controller: &mut AppController) -> WaveformPanelModel {
    let ui = &controller.ui;
    let view_span = (ui.waveform.view.end - ui.waveform.view.start).clamp(0.000_1, 1.0) as f32;
    let zoom_percent = (100.0 / view_span).round().clamp(100.0, 9999.0);
    WaveformPanelModel {
        loaded_label: ui
            .loaded_wav
            .as_deref()
            .map(view_model::sample_display_label),
        cursor_milli: ui.waveform.cursor.map(normalized_to_milli),
        playhead_milli: ui
            .waveform
            .playhead
            .visible
            .then_some(normalized_to_milli(ui.waveform.playhead.position)),
        selection_milli: ui.waveform.selection.map(|selection| {
            NormalizedRangeModel::new(
                normalized_to_milli(selection.start()),
                normalized_to_milli(selection.end()),
            )
        }),
        view_start_milli: normalized64_to_milli(ui.waveform.view.start),
        view_end_milli: normalized64_to_milli(ui.waveform.view.end),
        loop_enabled: ui.waveform.loop_enabled,
        tempo_label: ui.waveform.bpm_value.map(|bpm| format!("{bpm:.1} BPM")),
        zoom_label: Some(format!("{zoom_percent:.0}%")),
        waveform_image_signature: ui.waveform.waveform_image_signature,
        waveform_image: project_waveform_image(controller),
    }
}

/// Reuse or rebuild the projected waveform raster payload for the native model.
fn project_waveform_image(controller: &mut AppController) -> Option<Arc<ImageRgba>> {
    let signature = controller.ui.waveform.waveform_image_signature;
    let has_source_image = controller.ui.waveform.image.is_some();
    let has_cached_image = controller.projected_waveform_image.is_some();
    if signature.is_some()
        && controller.projected_waveform_image_signature == signature
        && has_source_image == has_cached_image
    {
        return controller.projected_waveform_image.clone();
    }
    // Producer-side waveform rendering now publishes shared immutable RGBA payloads and
    // versioned identities. Keep a projection-side fallback for tests/manual image assignment.
    let projected_waveform_image = controller.projected_waveform_image.clone().or_else(|| {
        controller
            .ui
            .waveform
            .image
            .as_ref()
            .and_then(waveform_image_to_native_rgba)
    });
    controller.projected_waveform_image_signature = signature;
    controller.projected_waveform_image = projected_waveform_image.clone();
    projected_waveform_image
}

/// Convert a rendered waveform image into the native immutable RGBA payload.
fn waveform_image_to_native_rgba(image: &crate::waveform::WaveformImage) -> Option<Arc<ImageRgba>> {
    if image.size[0] == 0 || image.size[1] == 0 {
        return None;
    }
    let mut pixels = Vec::with_capacity(
        image.size[0]
            .saturating_mul(image.size[1])
            .saturating_mul(4),
    );
    for pixel in &image.pixels {
        pixels.push(pixel.r());
        pixels.push(pixel.g());
        pixels.push(pixel.b());
        pixels.push(pixel.a());
    }
    ImageRgba::new(image.size[0], image.size[1], pixels).map(Arc::new)
}

/// Project waveform chrome labels and action-hint copy.
pub(crate) fn project_waveform_chrome_model(ui: &UiState) -> WaveformChromeModel {
    WaveformChromeModel {
        transport_hint: if ui.waveform.loop_enabled {
            String::from("Loop enabled")
        } else {
            String::from("Loop disabled")
        },
    }
}
