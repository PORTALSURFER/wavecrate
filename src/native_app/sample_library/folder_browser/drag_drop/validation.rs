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
            Some(FolderBrowserDrag::Folder { folder_id }) => {
                if self.selected_folder_is_source_root_id(folder_id) {
                    return false;
                }
                let Some(source) = self.find_folder(folder_id) else {
                    return false;
                };
                let source_path = Path::new(&source.id);
                !self.selected_folder_is_source_root_id(folder_id)
                    && source.id != target.id
                    && !target_path.starts_with(source_path)
            }
            Some(FolderBrowserDrag::Files { file_ids, .. }) => file_ids.iter().any(|id| {
                let path = Path::new(id);
                self.source_contains_audio_file(id) && path.parent() != Some(target_path)
            }),
            Some(FolderBrowserDrag::ExtractedFile { path }) => path.parent() != Some(target_path),
            Some(FolderBrowserDrag::WaveformExtraction { .. }) => true,
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
