use super::super::*;
use crate::app::controller::state::cache::FolderBrowserCacheKey;

impl AppController {
    /// Toggle whether the folder tree includes folders discovered on disk that
    /// do not currently contain any WAV-backed samples.
    pub(crate) fn toggle_show_all_folders(&mut self) {
        self.record_meaningful_ui_transaction("Toggle folder visibility", |controller| {
            let Some(source_id) = controller.selected_source_id() else {
                return;
            };
            let cache_key = FolderBrowserCacheKey {
                pane: controller.active_folder_pane(),
                source_id: source_id.clone(),
            };
            let (before_selected, before_negated) = controller
                .ui_cache
                .folders
                .models
                .get(&cache_key)
                .map(|model| (model.selected.clone(), model.negated.clone()))
                .unwrap_or_default();
            if let Some(model) = controller.ui_cache.folders.models.get_mut(&cache_key) {
                model.show_all_folders = !model.show_all_folders;
            } else {
                let mut model = FolderBrowserModel::default();
                model.show_all_folders = !model.show_all_folders;
                controller
                    .ui_cache
                    .folders
                    .models
                    .insert(cache_key.clone(), model);
            }
            controller.refresh_folder_browser();
            let selection_changed = controller
                .ui_cache
                .folders
                .models
                .get(&cache_key)
                .is_some_and(|model| {
                    model.selected != before_selected || model.negated != before_negated
                });
            if selection_changed {
                controller.rebuild_browser_lists();
            }
        });
    }

    /// Toggle whether folder filtering includes descendant files.
    pub(crate) fn toggle_folder_flattened_view(&mut self) {
        self.record_meaningful_ui_transaction("Toggle folder flattened view", |controller| {
            let Some(source_id) = controller.selected_source_id() else {
                return;
            };
            let cache_key = FolderBrowserCacheKey {
                pane: controller.active_folder_pane(),
                source_id: source_id.clone(),
            };
            if let Some(model) = controller.ui_cache.folders.models.get_mut(&cache_key) {
                model.file_scope_mode = model.file_scope_mode.toggle();
            } else {
                let mut model = FolderBrowserModel::default();
                model.file_scope_mode = model.file_scope_mode.toggle();
                controller.ui_cache.folders.models.insert(cache_key, model);
            }
            controller.refresh_folder_browser();
            controller.rebuild_browser_lists();
        });
    }
}
