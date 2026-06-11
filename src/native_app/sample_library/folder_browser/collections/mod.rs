mod assignment;
mod drag_drop;
mod focus;
mod layout;
mod model;
mod rename;
mod view;

pub(in crate::native_app) use layout::{
    COLLECTION_ROW_HEIGHT, COLLECTION_ROW_SPACING, COLLECTIONS_PANEL_HEADER_CONTENT_SPACING,
    COLLECTIONS_PANEL_HEADER_HEIGHT, COLLECTIONS_PANEL_PADDING, DEFAULT_COLLECTIONS_PANEL_HEIGHT,
};
pub(super) use model::{CollectionRenameEdit, SampleCollectionConfig, default_collections};
pub(in crate::native_app) use model::{
    CollectionRenameView, SampleCollectionView, SelectedFileCollectionCandidate, collection_hotkey,
};

#[cfg(test)]
pub(super) use layout::{COLLAPSED_COLLECTIONS_PANEL_HEIGHT, MIN_COLLECTIONS_PANEL_HEIGHT};
