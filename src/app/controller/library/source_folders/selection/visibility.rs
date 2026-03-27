use super::super::*;

impl AppController {
    /// Toggle whether the folder tree includes folders discovered on disk that
    /// do not currently contain any WAV-backed samples.
    pub(crate) fn toggle_show_all_folders(&mut self) {
        self.record_meaningful_ui_transaction("Toggle folder visibility", |controller| {
            let Some(source_id) = controller.selected_source_id() else {
                return;
            };
            let (before_selected, before_negated) = controller
                .ui_cache
                .folders
                .models
                .get(&source_id)
                .map(|model| (model.selected.clone(), model.negated.clone()))
                .unwrap_or_default();
            if let Some(model) = controller.ui_cache.folders.models.get_mut(&source_id) {
                model.show_all_folders = !model.show_all_folders;
            } else {
                let mut model = FolderBrowserModel::default();
                model.show_all_folders = !model.show_all_folders;
                controller.ui_cache.folders.models.insert(source_id.clone(), model);
            }
            controller.refresh_folder_browser();
            let selection_changed = controller
                .ui_cache
                .folders
                .models
                .get(&source_id)
                .is_some_and(|model| {
                    model.selected != before_selected || model.negated != before_negated
                });
            if selection_changed {
                controller.rebuild_browser_lists();
            }
        });
    }
}
