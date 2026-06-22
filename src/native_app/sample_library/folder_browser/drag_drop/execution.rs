use super::*;

impl FolderBrowserState {
    pub(in crate::native_app) fn drop_drag_on_folder(
        &mut self,
        target_folder_id: &str,
    ) -> Result<FolderMoveDropInput, String> {
        let Some(drag) = self.drag_drop.drag.clone() else {
            return Ok(FolderMoveDropInput::Status(Default::default()));
        };
        if !self.can_drop_drag_on_folder(target_folder_id) {
            self.clear_drag();
            return Ok(FolderMoveDropInput::Status(
                super::super::FolderDropResult {
                    moved_paths: Vec::new(),
                    status: Some(String::from("Drop target unchanged")),
                },
            ));
        }
        self.drag_drop.drop_target.close();
        self.drag_drop.clear_folder_hover_auto_expand();
        let result = match drag {
            FolderBrowserDrag::Folder { folder_ids } => {
                self.prepare_move_folders_to_folder(&folder_ids, target_folder_id)?
            }
            FolderBrowserDrag::Files {
                file_ids,
                remove_from_collection,
            } => self.prepare_move_files_to_folder(
                &file_ids,
                target_folder_id,
                remove_from_collection,
            )?,
            FolderBrowserDrag::ExtractedFile { path } => {
                self.prepare_move_extracted_file_to_folder(&path, target_folder_id)?
            }
            FolderBrowserDrag::WaveformExtraction { .. } => {
                return Err(String::from(
                    "Extraction drop must be committed by the app shell",
                ));
            }
        };
        self.clear_drag();
        Ok(result)
    }
}
