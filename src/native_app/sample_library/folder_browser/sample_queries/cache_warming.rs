use super::{FolderBrowserState, FolderEntry};

pub(super) fn prewarm_selected_source_audio_projection_cache(state: &FolderBrowserState) {
    if let Some(root) = state.tree.folders.first() {
        prewarm_folder_audio_projection_cache(state, root);
    }
}

fn prewarm_folder_audio_projection_cache(state: &FolderBrowserState, folder: &FolderEntry) {
    let _ = state.selected_folder_audio_file_indices_ref(folder);
    for child in &folder.children {
        prewarm_folder_audio_projection_cache(state, child);
    }
}
