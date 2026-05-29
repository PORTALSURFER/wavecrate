use std::{collections::BTreeMap, path::PathBuf};

use radiant::{
    gui::types::{Point, Rgba8},
    widgets::{DragHandleMessage, TextInputMessage},
};
use wavecrate::sample_sources::SampleCollection;

use super::{FolderBrowserDrag, FolderBrowserState};

mod collection_hit_target;
pub(in crate::gui_app) use collection_hit_target::{CollectionHitMessage, CollectionHitTarget};

pub(in crate::gui_app) const COLLECTION_ROW_HEIGHT: f32 = 22.0;
pub(in crate::gui_app) const MIN_COLLECTIONS_PANEL_HEIGHT: f32 = 72.0;
pub(in crate::gui_app) const MAX_COLLECTIONS_PANEL_HEIGHT: f32 = 260.0;
pub(in crate::gui_app) const DEFAULT_COLLECTIONS_PANEL_HEIGHT: f32 = 148.0;

#[derive(Clone, Debug, PartialEq)]
pub(in crate::gui_app) struct SampleCollectionView {
    pub(in crate::gui_app) collection: SampleCollection,
    pub(in crate::gui_app) hotkey: char,
    pub(in crate::gui_app) name: String,
    pub(in crate::gui_app) color: Rgba8,
    pub(in crate::gui_app) selected: bool,
    pub(in crate::gui_app) drop_target: bool,
    pub(in crate::gui_app) drag_active: bool,
    pub(in crate::gui_app) assigned_count: usize,
}

#[derive(Clone, Debug, PartialEq)]
pub(super) struct SampleCollectionConfig {
    pub(super) collection: SampleCollection,
    pub(super) hotkey: char,
    pub(super) name: String,
    pub(super) color: Rgba8,
}

#[derive(Clone, Debug, PartialEq)]
pub(super) struct CollectionPanelResize {
    pub(super) start_y: f32,
    pub(super) start_height: f32,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) struct CollectionRenameEdit {
    pub(super) collection: SampleCollection,
    pub(super) draft: String,
    pub(super) input_id: u64,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(in crate::gui_app) struct CollectionRenameView {
    pub(in crate::gui_app) draft: String,
    pub(in crate::gui_app) input_id: u64,
}

impl FolderBrowserState {
    pub(super) fn default_collections() -> Vec<SampleCollectionConfig> {
        (0..SampleCollection::COUNT)
            .filter_map(|index| {
                let collection = SampleCollection::new(index as u8)?;
                Some(SampleCollectionConfig {
                    collection,
                    hotkey: collection_hotkey(collection),
                    name: format!("Collection {}", collection_hotkey(collection)),
                    color: collection_color(collection),
                })
            })
            .collect()
    }

    pub(in crate::gui_app) fn collections_panel_height(&self) -> f32 {
        self.collections_panel_height
    }

    pub(in crate::gui_app) fn visible_collections(&self) -> Vec<SampleCollectionView> {
        let counts = self.collection_counts();
        self.collections
            .iter()
            .map(|collection| SampleCollectionView {
                collection: collection.collection,
                hotkey: collection.hotkey,
                name: collection.name.clone(),
                color: collection.color,
                selected: self.selected_collection == Some(collection.collection),
                drop_target: self.drop_target_collection == Some(collection.collection),
                drag_active: self.file_drag_active(),
                assigned_count: counts
                    .get(&collection.collection.index())
                    .copied()
                    .unwrap_or_default(),
            })
            .collect()
    }

    pub(in crate::gui_app) fn collection_color(
        &self,
        collection: SampleCollection,
    ) -> Option<Rgba8> {
        self.collections
            .iter()
            .find(|entry| entry.collection == collection)
            .map(|entry| entry.color)
    }

    pub(in crate::gui_app) fn set_file_collection_state(
        &mut self,
        path: &std::path::Path,
        collection: SampleCollection,
    ) -> bool {
        let path_id = path.to_string_lossy();
        let mut updated = false;
        for folder in &mut self.folders {
            updated |= folder.set_file_collection(path_id.as_ref(), collection);
        }
        for source in &mut self.sources {
            if let Some(root_folder) = &mut source.root_folder {
                updated |= root_folder.set_file_collection(path_id.as_ref(), collection);
            }
        }
        updated
    }

    pub(in crate::gui_app) fn selected_file_collection_candidates(
        &self,
    ) -> Vec<SelectedFileCollectionCandidate> {
        self.selected_audio_files()
            .into_iter()
            .filter(|file| self.is_file_selected(&file.id))
            .map(|file| SelectedFileCollectionCandidate {
                path: PathBuf::from(&file.id),
            })
            .collect()
    }

    pub(in crate::gui_app) fn drag_file_collection_candidates(
        &self,
    ) -> Vec<SelectedFileCollectionCandidate> {
        match &self.drag {
            Some(FolderBrowserDrag::Files { file_ids }) => file_ids
                .iter()
                .filter(|file_id| {
                    self.selected_audio_files()
                        .iter()
                        .any(|file| file.id == **file_id)
                })
                .map(PathBuf::from)
                .map(|path| SelectedFileCollectionCandidate { path })
                .collect(),
            _ => Vec::new(),
        }
    }

    pub(super) fn resize_collections_panel(&mut self, message: DragHandleMessage) {
        match message {
            DragHandleMessage::Started { position } => {
                self.collection_panel_resize = Some(CollectionPanelResize {
                    start_y: position.y,
                    start_height: self.collections_panel_height,
                });
            }
            DragHandleMessage::Moved { position } | DragHandleMessage::Ended { position } => {
                let Some(resize) = self.collection_panel_resize.clone() else {
                    return;
                };
                self.collections_panel_height = (resize.start_height + resize.start_y - position.y)
                    .clamp(MIN_COLLECTIONS_PANEL_HEIGHT, MAX_COLLECTIONS_PANEL_HEIGHT);
                if matches!(message, DragHandleMessage::Ended { .. }) {
                    self.collection_panel_resize = None;
                }
            }
        }
    }

    pub(super) fn activate_collection(&mut self, collection: SampleCollection) {
        if self.selected_collection != Some(collection) {
            self.collection_rename_edit = None;
            self.selected_file = None;
            self.selected_file_ids.clear();
            self.file_view_start = 0;
        }
        self.selected_collection = Some(collection);
    }

    pub(in crate::gui_app) fn collection_rename_view(
        &self,
        collection: SampleCollection,
    ) -> Option<CollectionRenameView> {
        let edit = self.collection_rename_edit.as_ref()?;
        (edit.collection == collection).then(|| CollectionRenameView {
            draft: edit.draft.clone(),
            input_id: edit.input_id,
        })
    }

    pub(in crate::gui_app) fn begin_rename_collection(
        &mut self,
        collection: SampleCollection,
    ) -> Option<u64> {
        let entry = self
            .collections
            .iter()
            .find(|entry| entry.collection == collection)?;
        let input_id = collection_rename_input_id(collection);
        self.selected_collection = Some(collection);
        self.collection_rename_edit = Some(CollectionRenameEdit {
            collection,
            draft: entry.name.clone(),
            input_id,
        });
        Some(input_id)
    }

    pub(in crate::gui_app) fn apply_collection_rename_input(
        &mut self,
        message: TextInputMessage,
    ) -> Option<String> {
        let edit = self.collection_rename_edit.as_mut()?;
        match message {
            TextInputMessage::Changed { value } => {
                edit.draft = value;
                None
            }
            TextInputMessage::Submitted { value } => {
                let label = value.trim();
                if label.is_empty() {
                    self.collection_rename_edit = None;
                    return Some(String::from("Collection rename cancelled"));
                }
                if let Some(entry) = self
                    .collections
                    .iter_mut()
                    .find(|entry| entry.collection == edit.collection)
                {
                    entry.name = label.to_string();
                }
                self.collection_rename_edit = None;
                Some(String::from("Collection renamed"))
            }
            TextInputMessage::CompletionRequested { .. } => None,
        }
    }

    pub(super) fn hover_drop_target_collection(
        &mut self,
        collection: SampleCollection,
        position: Point,
    ) {
        self.update_drag_pointer(position);
        self.drop_target_collection = self.file_drag_active().then_some(collection);
    }

    fn collection_counts(&self) -> BTreeMap<u8, usize> {
        let mut counts = BTreeMap::new();
        for file in self.selected_source_audio_files() {
            if let Some(collection) = file.collection {
                *counts.entry(collection.index()).or_insert(0) += 1;
            }
        }
        counts
    }
}

pub(in crate::gui_app) fn collection_hotkey(collection: SampleCollection) -> char {
    char::from(b'1' + collection.index())
}

pub(in crate::gui_app) fn collection_color(collection: SampleCollection) -> Rgba8 {
    const COLORS: [Rgba8; 6] = [
        Rgba8 {
            r: 255,
            g: 86,
            b: 98,
            a: 240,
        },
        Rgba8 {
            r: 255,
            g: 166,
            b: 62,
            a: 240,
        },
        Rgba8 {
            r: 249,
            g: 220,
            b: 82,
            a: 240,
        },
        Rgba8 {
            r: 118,
            g: 226,
            b: 97,
            a: 240,
        },
        Rgba8 {
            r: 58,
            g: 197,
            b: 255,
            a: 240,
        },
        Rgba8 {
            r: 174,
            g: 112,
            b: 255,
            a: 240,
        },
    ];
    COLORS[collection.index() as usize]
}

fn collection_rename_input_id(collection: SampleCollection) -> u64 {
    0x5743_0000_0000_4300 + collection.index() as u64
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(in crate::gui_app) struct SelectedFileCollectionCandidate {
    pub(in crate::gui_app) path: PathBuf,
}
