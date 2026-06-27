use super::*;

impl FolderBrowserState {
    pub(in crate::native_app::sample_library::folder_browser) fn can_drop_drag_on_folder(
        &self,
        target_folder_id: &str,
    ) -> bool {
        let Some(target) = self.find_folder(target_folder_id) else {
            return false;
        };
        let target_path = Path::new(&target.id);
        match &self.drag_drop.drag {
            Some(FolderBrowserDrag::Folder { folder_ids }) => {
                if folder_ids.is_empty() {
                    return false;
                }
                if self.folder_path_is_locked(target_path) {
                    return false;
                }
                let mut moving = false;
                for folder_id in folder_ids {
                    if self.selected_folder_is_source_root_id(folder_id) {
                        continue;
                    }
                    let Some(source) = self.find_folder(folder_id) else {
                        return false;
                    };
                    let source_path = Path::new(&source.id);
                    if source.id == target.id
                        || target_path.starts_with(source_path)
                        || self.folder_tree_change_is_locked(source_path)
                    {
                        return false;
                    }
                    if source_path.parent() != Some(target_path) {
                        moving = true;
                    }
                }
                moving
            }
            Some(FolderBrowserDrag::Files { file_ids, .. }) => {
                !self.folder_path_is_locked(target_path)
                    && file_ids.iter().any(|id| {
                        let path = Path::new(id);
                        self.source_contains_audio_file(id)
                            && path.parent() != Some(target_path)
                            && !self.file_path_is_locked(path)
                    })
            }
            Some(FolderBrowserDrag::ExtractedFile { path }) => {
                path.parent() != Some(target_path) && !self.folder_path_is_locked(target_path)
            }
            None => false,
        }
    }

    pub(in crate::native_app::sample_library::folder_browser) fn source_contains_audio_file(
        &self,
        file_id: &str,
    ) -> bool {
        self.selected_source_root_folder()
            .is_some_and(|folder| folder_contains_audio_file(folder, file_id))
    }
}

fn folder_contains_audio_file(folder: &super::super::FolderEntry, file_id: &str) -> bool {
    folder
        .files
        .iter()
        .any(|file| file.id == file_id && file.is_audio())
        || folder
            .children
            .iter()
            .any(|child| folder_contains_audio_file(child, file_id))
}
