use radiant::widgets::{TextInputMessage, TextInputMessageKind};
use wavecrate::sample_sources::SampleCollection;

use super::{
    super::FolderBrowserState,
    model::{CollectionRenameEdit, CollectionRenameView, collection_rename_input_id},
};

impl FolderBrowserState {
    pub(in crate::native_app) fn collection_rename_view(
        &self,
        collection: SampleCollection,
    ) -> Option<CollectionRenameView> {
        let edit = self.collection_panel.rename_edit.as_ref()?;
        (edit.collection == collection).then(|| CollectionRenameView {
            selection_start: 0,
            selection_end: edit.draft.chars().count(),
            draft: edit.draft.clone(),
            input_id: edit.input_id,
        })
    }

    pub(in crate::native_app) fn begin_rename_collection(
        &mut self,
        collection: SampleCollection,
    ) -> Option<u64> {
        let entry = self
            .collection_panel
            .collections
            .iter()
            .find(|entry| entry.collection == collection)?;
        let name = entry.name.clone();
        let input_id = collection_rename_input_id(collection);
        self.activate_collection(collection);
        self.rename.folder = None;
        self.rename.file = None;
        self.collection_panel.rename_edit = Some(CollectionRenameEdit {
            collection,
            draft: name,
            input_id,
        });
        Some(input_id)
    }

    pub(in crate::native_app) fn apply_collection_rename_input(
        &mut self,
        message: &TextInputMessage,
    ) -> Option<String> {
        let edit = self.collection_panel.rename_edit.as_mut()?;
        let parts = message.parts();
        match parts.kind {
            TextInputMessageKind::CompletionRequested => return None,
            TextInputMessageKind::Changed => {
                edit.draft = parts.value.to_owned();
                return None;
            }
            TextInputMessageKind::Submitted => {}
        }

        let label = parts.value.trim();
        if label.is_empty() {
            self.collection_panel.rename_edit = None;
            return Some(String::from("Collection rename cancelled"));
        }
        if let Some(entry) = self
            .collection_panel
            .collections
            .iter_mut()
            .find(|entry| entry.collection == edit.collection)
        {
            entry.name = label.to_string();
        }
        self.collection_panel.rename_edit = None;
        Some(String::from("Collection renamed"))
    }
}
