use std::path::PathBuf;

use radiant::{gui::types::Rgba8, prelude as ui};
use wavecrate::sample_sources::SampleCollection;

const COLLECTION_RENAME_INPUT_SCOPE: u64 = 0x5743_0000_0000_4301;

#[derive(Clone, Debug, PartialEq)]
pub(in crate::native_app) struct SampleCollectionView {
    pub(in crate::native_app) collection: SampleCollection,
    pub(in crate::native_app) hotkey: char,
    pub(in crate::native_app) name: String,
    pub(in crate::native_app) color: Rgba8,
    pub(in crate::native_app) selected: bool,
    pub(in crate::native_app) focused: bool,
    pub(in crate::native_app) focus_alpha: u8,
    pub(in crate::native_app) drop_target: bool,
    pub(in crate::native_app) drag_active: bool,
    pub(in crate::native_app) assigned_count: usize,
}

#[derive(Clone, Debug, PartialEq)]
pub(in crate::native_app::sample_library::folder_browser) struct SampleCollectionConfig {
    pub(in crate::native_app::sample_library::folder_browser) collection: SampleCollection,
    pub(in crate::native_app::sample_library::folder_browser) hotkey: char,
    pub(in crate::native_app::sample_library::folder_browser) name: String,
    pub(in crate::native_app::sample_library::folder_browser) color: Rgba8,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(in crate::native_app::sample_library::folder_browser) struct CollectionRenameEdit {
    pub(in crate::native_app::sample_library::folder_browser) collection: SampleCollection,
    pub(in crate::native_app::sample_library::folder_browser) draft: String,
    pub(in crate::native_app::sample_library::folder_browser) input_id: u64,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(in crate::native_app) struct CollectionRenameView {
    pub(in crate::native_app) draft: String,
    pub(in crate::native_app) input_id: u64,
    pub(in crate::native_app) selection_start: usize,
    pub(in crate::native_app) selection_end: usize,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(in crate::native_app) struct SelectedFileCollectionCandidate {
    pub(in crate::native_app) path: PathBuf,
    pub(in crate::native_app) assigned: bool,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(in crate::native_app) struct MissingCollectionFile {
    pub(in crate::native_app) root: PathBuf,
    pub(in crate::native_app) database_root: PathBuf,
    pub(in crate::native_app) relative_path: PathBuf,
    pub(in crate::native_app) absolute_path: PathBuf,
    pub(in crate::native_app) collection: SampleCollection,
}

pub(in crate::native_app::sample_library::folder_browser) fn default_collections()
-> Vec<SampleCollectionConfig> {
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

pub(in crate::native_app) fn collection_hotkey(collection: SampleCollection) -> char {
    char::from(b'1' + collection.index())
}

pub(super) fn collection_color(collection: SampleCollection) -> Rgba8 {
    const COLORS: [Rgba8; 6] = [
        Rgba8 {
            r: 242,
            g: 100,
            b: 79,
            a: 232,
        },
        Rgba8 {
            r: 210,
            g: 158,
            b: 81,
            a: 224,
        },
        Rgba8 {
            r: 199,
            g: 188,
            b: 92,
            a: 224,
        },
        Rgba8 {
            r: 112,
            g: 172,
            b: 111,
            a: 224,
        },
        Rgba8 {
            r: 83,
            g: 161,
            b: 194,
            a: 224,
        },
        Rgba8 {
            r: 149,
            g: 116,
            b: 185,
            a: 224,
        },
    ];
    COLORS[collection.index() as usize]
}

pub(super) fn collection_rename_input_id(collection: SampleCollection) -> u64 {
    ui::stable_widget_id(
        COLLECTION_RENAME_INPUT_SCOPE,
        collection.index().to_string(),
    )
}
