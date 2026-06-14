use super::super::*;
use super::{pipeline, side_effects::SelectionSideEffects};

/// Shared wav-path selection target resolution.
///
/// Path lookup stays separate from the side-effect pipeline so preview and
/// commit callers can share resolution without inheriting each other's behavior.
pub(super) fn select_wav_path_with_options(
    controller: &mut AppController,
    path: &Path,
    rebuild: bool,
    side_effects: SelectionSideEffects,
) {
    let Some(index) = controller.wav_index_for_path(path) else {
        return;
    };
    pipeline::select_wav_known_index_with_options(
        controller,
        index,
        path.to_path_buf(),
        rebuild,
        side_effects,
    );
}

/// Shared wav-index selection target resolution for hot paths where the entry
/// index is already known.
pub(super) fn select_wav_index_with_options(
    controller: &mut AppController,
    index: usize,
    rebuild: bool,
    side_effects: SelectionSideEffects,
) {
    let path = match controller.wav_entry(index) {
        Some(entry) => entry.relative_path.clone(),
        None => return,
    };
    pipeline::select_wav_known_index_with_options(controller, index, path, rebuild, side_effects);
}
