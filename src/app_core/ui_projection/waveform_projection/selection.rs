use super::units::normalized64_to_nanos;
use crate::app_core::actions::NativeNormalizedRangeModel as NormalizedRangeModel;
use crate::app_core::state::UiState;
use crate::app_core::state::WaveformSliceBatchProfile;

/// Project edit-selection bounds into normalized milli-space.
pub(in crate::app_core::ui_projection) fn project_waveform_edit_selection_milli(
    ui: &UiState,
) -> Option<NormalizedRangeModel> {
    ui.waveform.edit_selection.map(|selection| {
        NormalizedRangeModel::from_nanos(
            normalized64_to_nanos(selection.start_f64()),
            normalized64_to_nanos(selection.end_f64()),
        )
    })
}

/// Project waveform slice previews into the UI runtime model.
pub(in crate::app_core::ui_projection) fn project_waveform_slice_previews(
    ui: &UiState,
) -> Vec<crate::app_core::actions::NativeWaveformSlicePreviewModel> {
    let duplicate_cleanup = ui.waveform.duplicate_cleanup.as_ref();
    ui.waveform
        .slices
        .iter()
        .enumerate()
        .map(
            |(index, slice)| crate::app_core::actions::NativeWaveformSlicePreviewModel {
                range: NormalizedRangeModel::from_nanos(
                    normalized64_to_nanos(slice.start_f64()),
                    normalized64_to_nanos(slice.end_f64()),
                ),
                selected: ui.waveform.selected_slices.contains(&index),
                focused: ui.waveform.slice_review.focused_index == Some(index),
                marked_for_export: ui.waveform.slice_review.marked_indices.contains(&index),
                review_candidate: ui.waveform.slice_batch_profile
                    == WaveformSliceBatchProfile::ExactDuplicateBeats,
                review_exempted: duplicate_cleanup
                    .and_then(|cleanup| cleanup.previews.get(index))
                    .is_some_and(|preview| preview.exempted),
            },
        )
        .collect()
}
