use super::*;

impl FolderBrowserState {
    pub(in crate::native_app) fn drag_preview(&self) -> Option<FolderDragPreview> {
        let drag = self.drag_drop.drag.as_ref()?;
        let pointer = self.drag_drop.drag_pointer?;
        Some(FolderDragPreview {
            label: self.drag_preview_label(drag)?,
            pointer,
        })
    }

    pub(in crate::native_app) fn file_drag_active(&self) -> bool {
        matches!(self.drag_drop.drag, Some(FolderBrowserDrag::Files { .. }))
    }

    pub(in crate::native_app) fn extracted_file_drag_active(&self) -> bool {
        matches!(
            self.drag_drop.drag,
            Some(
                FolderBrowserDrag::ExtractedFile { .. }
                    | FolderBrowserDrag::WaveformExtraction { .. }
            )
        )
    }

    pub(in crate::native_app) fn extracted_file_drag_path(&self) -> Option<PathBuf> {
        match &self.drag_drop.drag {
            Some(FolderBrowserDrag::ExtractedFile { path }) => Some(path.clone()),
            _ => None,
        }
    }

    pub(in crate::native_app) fn drag_active(&self) -> bool {
        self.drag_drop.drag.is_some()
    }

    pub(in crate::native_app) fn file_drag_source(&self, file_id: &str) -> bool {
        match &self.drag_drop.drag {
            Some(FolderBrowserDrag::Files { file_ids, .. }) => {
                file_ids.iter().any(|id| id == file_id)
            }
            _ => false,
        }
    }

    pub(in crate::native_app) fn external_drag_request(&self) -> Option<ui::ExternalDragRequest> {
        let drag = self.drag_drop.drag.as_ref()?;
        let label = self.drag_preview_label(drag)?;
        let FolderBrowserDrag::Files { file_ids, .. } = drag else {
            return match drag {
                FolderBrowserDrag::ExtractedFile { path } => {
                    Some(ui::ExternalDragRequest::files([path.clone()], label))
                }
                FolderBrowserDrag::WaveformExtraction { .. } => None,
                _ => None,
            };
        };
        let paths = file_ids.iter().map(PathBuf::from).collect::<Vec<_>>();
        Some(ui::ExternalDragRequest::files(paths, label))
    }

    fn drag_preview_label(&self, drag: &FolderBrowserDrag) -> Option<String> {
        match drag {
            FolderBrowserDrag::Folder { folder_ids } => match folder_ids.as_slice() {
                [] => None,
                [folder_id] => self
                    .find_folder(folder_id)
                    .map(|folder| folder.name.clone()),
                folders => Some(format!("{} folders", folders.len())),
            },
            FolderBrowserDrag::Files { file_ids, .. } => match file_ids.as_slice() {
                [] => None,
                [file_id] => Some(file_label(Path::new(file_id))),
                files => Some(format!("{} files", files.len())),
            },
            FolderBrowserDrag::ExtractedFile { path } => Some(file_label(path)),
            FolderBrowserDrag::WaveformExtraction { label, .. } => Some(label.clone()),
        }
    }
}
