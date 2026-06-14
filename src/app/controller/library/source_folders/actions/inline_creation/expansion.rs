use super::*;
use std::path::Path;

pub(super) fn ensure_folder_expanded_for_creation(controller: &mut AppController, parent: &Path) {
    if parent.as_os_str().is_empty() {
        return;
    }
    let Some(model) = controller.current_folder_model_mut() else {
        return;
    };
    if model.expanded.insert(parent.to_path_buf()) {
        let snapshot = model.clone();
        if let Some(source_id) = controller.selected_source_id() {
            controller.queue_folder_projection_for_pane(
                controller.active_folder_pane(),
                source_id,
                snapshot,
            );
        }
    }
}
