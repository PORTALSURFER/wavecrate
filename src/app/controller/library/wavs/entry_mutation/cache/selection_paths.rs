use super::*;
use std::path::Path;

/// Rewrite selection paths when a file is renamed or moved.
pub(crate) fn update_selection_paths(
    controller: &mut AppController,
    source: &SampleSource,
    old_path: &Path,
    new_path: &Path,
) {
    controller.update_compare_anchor_path(&source.id, old_path, new_path);
    if controller.selection_state.ctx.selected_source.as_ref() == Some(&source.id) {
        if !controller.ui.browser.selection.selected_paths.is_empty() {
            let mut updated =
                Vec::with_capacity(controller.ui.browser.selection.selected_paths.len());
            let mut replaced = false;
            for path in controller.ui.browser.selection.selected_paths.iter() {
                if path == old_path {
                    replaced = true;
                    if !updated.iter().any(|candidate| candidate == new_path) {
                        updated.push(new_path.to_path_buf());
                    }
                } else {
                    updated.push(path.clone());
                }
            }
            if replaced {
                controller.set_browser_selected_paths(updated);
            }
        }
        if controller.sample_view.wav.selected_wav.as_deref() == Some(old_path) {
            controller.sample_view.wav.selected_wav = Some(new_path.to_path_buf());
        }
        if controller.sample_view.wav.loaded_wav.as_deref() == Some(old_path) {
            controller.sample_view.wav.loaded_wav = Some(new_path.to_path_buf());
            controller.set_ui_loaded_wav(Some(new_path.to_path_buf()));
        } else if controller.ui.loaded_wav.as_deref() == Some(old_path) {
            controller.set_ui_loaded_wav(Some(new_path.to_path_buf()));
        }
    }
    if let Some(audio) = controller.sample_view.wav.loaded_audio.as_mut()
        && audio.source_id == source.id
        && audio.relative_path == old_path
    {
        audio.relative_path = new_path.to_path_buf();
    }
}
