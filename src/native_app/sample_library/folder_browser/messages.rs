use radiant::{
    gui::types::Point,
    widgets::{DragHandleMessage, PointerModifiers, TextInputMessage},
};
use wavecrate::sample_sources::SampleCollection;

use super::curation::BrowserCurationScope;
use super::harvest_filter::HarvestFilter;
use super::playback_type_filter::PlaybackTypeFilter;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(in crate::native_app) enum FilterFamily {
    Name,
    Tags,
    Curation,
    Harvest,
    PlaybackType,
    Rating,
}

#[derive(Clone, Debug, PartialEq)]
pub(in crate::native_app) enum FolderBrowserMessage {
    AddSource,
    SelectSource(String),
    OpenSourceContextMenu(String, Point),
    ActivateFolder(String, PointerModifiers),
    ToggleFolderExpansion(String),
    OpenFolderContextMenu(String, Point),
    DragFolder(String, DragHandleMessage),
    HoverDropTarget(String, Point),
    ClearDropTargetUnless(String, Point),
    ClearDropTarget(Point),
    DropOnFolder(String),
    HoverSourceDropTarget(String, Point),
    ClearSourceDropTargetUnless(String, Point),
    DropOnSource(String),
    ToggleFolderSubtreeListing,
    ToggleEmptyFolderVisibility,
    ResizeCollectionsPanel(DragHandleMessage),
    ResizeFilterPanel(DragHandleMessage),
    ResizeMetadataPanel(DragHandleMessage),
    ActivateCollection(SampleCollection),
    OpenCollectionContextMenu(SampleCollection, Point),
    RenameCollection(SampleCollection),
    HoverCollectionDropTarget(SampleCollection, Point),
    DropOnCollection(SampleCollection),
    BeginRenameSelected,
    CancelRename,
    BeginCreateSubfolder,
    RenameInput(TextInputMessage),
    NameFilterInput(TextInputMessage),
    TagFilterInput(TextInputMessage),
    SetFilterFamilyEnabled(FilterFamily, bool),
    TogglePlaybackTypeFilter(PlaybackTypeFilter, bool),
    ToggleRatingFilter(i8, bool),
    SetCurationScope(BrowserCurationScope, bool),
    SetHarvestFilter(HarvestFilter, bool),
    SortFileColumn(String),
    ResizeFileColumn(String, DragHandleMessage),
    DragFileColumn(String, DragHandleMessage),
    CancelFileColumnDrag,
    ExitCollectionFocus,
    ToggleSimilarityAnchor(String),
}
