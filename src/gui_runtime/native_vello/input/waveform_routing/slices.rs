use super::*;

/// Resolve one slice-toggle action when the pointer lands inside a preview slice.
pub(super) fn waveform_slice_toggle_action_from_pointer(
    layout: &ShellLayout,
    model: &AppModel,
    point: Point,
) -> Option<UiAction> {
    if !layout.waveform_plot.contains(point) {
        return None;
    }
    compute_waveform_slice_preview_rects(
        layout.waveform_plot,
        &model.waveform.slices,
        model.waveform.view_start_micros,
        model.waveform.view_end_micros,
    )
    .into_iter()
    .enumerate()
    .find(|(_, slice)| slice.rect.contains(point))
    .map(|(index, _)| {
        if model.waveform_chrome.exact_duplicate_cleanup_available {
            UiAction::AuditionWaveformDuplicateSlice { index }
        } else {
            UiAction::ToggleWaveformSliceSelection { index }
        }
    })
}

/// Resolve one duplicate-cleanup exemption toggle when the pointer lands inside a preview slice.
pub(crate) fn duplicate_cleanup_exemption_action_from_pointer(
    layout: &ShellLayout,
    model: &AppModel,
    point: Point,
) -> Option<UiAction> {
    if !model.waveform_chrome.exact_duplicate_cleanup_available
        || !layout.waveform_plot.contains(point)
    {
        return None;
    }
    compute_waveform_slice_preview_rects(
        layout.waveform_plot,
        &model.waveform.slices,
        model.waveform.view_start_micros,
        model.waveform.view_end_micros,
    )
    .into_iter()
    .enumerate()
    .find(|(_, slice)| slice.rect.contains(point))
    .map(|(index, _)| UiAction::ToggleWaveformDuplicateSliceExemption { index })
}
