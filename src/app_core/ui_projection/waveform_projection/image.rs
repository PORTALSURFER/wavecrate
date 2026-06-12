use super::*;

/// Reuse or rebuild the projected waveform raster payload for the UI model.
pub(super) fn project_waveform_image(
    controller: &mut AppController,
    signature: Option<u64>,
) -> Option<Arc<ImageRgba>> {
    let has_item_image = controller.ui.waveform.image.is_some();
    let has_cached_image = controller.projected_waveform_image.is_some();
    if controller.projected_waveform_image_signature == signature
        && has_item_image == has_cached_image
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
            .and_then(super::super::waveform_image_to_native_rgba)
    });
    controller.projected_waveform_image_signature = signature;
    controller.projected_waveform_image = projected_waveform_image.clone();
    projected_waveform_image
}

/// Return the image signature only when the stored raster and current overlays share a view.
pub(crate) fn effective_waveform_image_signature(controller: &AppController) -> Option<u64> {
    let signature = controller.ui.waveform.waveform_image_signature?;
    let meta = controller.waveform_render_meta()?;
    (controller.ui.waveform.image.is_some()
        && meta.matches_view_identity(controller.ui.waveform.view))
    .then_some(signature)
}
