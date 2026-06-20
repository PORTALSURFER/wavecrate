use std::path::PathBuf;

use radiant::gui::types::Point;
use wavecrate::sample_sources::SampleCollection;

use super::{
    super::{FolderBrowserDrag, FolderBrowserDropTarget, FolderBrowserState},
    model::SelectedFileCollectionCandidate,
};

impl FolderBrowserState {
    pub(in crate::native_app) fn drag_file_collection_candidates(
        &self,
        collection: SampleCollection,
    ) -> Vec<SelectedFileCollectionCandidate> {
        match &self.drag_drop.drag {
            Some(FolderBrowserDrag::Files { file_ids, .. }) => file_ids
                .iter()
                .filter_map(|file_id| {
                    self.selected_audio_files()
                        .iter()
                        .find(|file| file.id == **file_id)
                        .map(|file| SelectedFileCollectionCandidate {
                            path: PathBuf::from(&file.id),
                            assigned: file.belongs_to_collection(collection),
                        })
                })
                .collect(),
            _ => Vec::new(),
        }
    }

    pub(in crate::native_app::sample_library::folder_browser) fn hover_drop_target_collection(
        &mut self,
        collection: SampleCollection,
        position: Point,
    ) {
        self.update_drag_pointer(position);
        let changed = if self.file_drag_active() {
            self.drag_drop
                .drop_target
                .open_changed(FolderBrowserDropTarget::Collection(collection))
        } else {
            self.drag_drop.drop_target.close_changed()
        };
        if changed {
            self.drag_drop.revision.bump();
        }
    }
}
