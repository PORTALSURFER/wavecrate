use radiant::{
    gui::types::Point,
    widgets::{DragHandleMessage, TextInputMessage},
};
use wavecrate::sample_sources::SampleCollection;

#[derive(Clone, Debug, PartialEq)]
pub(in crate::native_app) enum FolderBrowserMessage {
    AddSource,
    SelectSource(String),
    OpenSourceContextMenu(String, Point),
    ActivateFolder(String),
    ToggleFolderExpansion(String),
    OpenFolderContextMenu(String, Point),
    DragFolder(String, DragHandleMessage),
    HoverDropTarget(String, Point),
    ClearDropTargetUnless(String, Point),
    ClearDropTarget(Point),
    DropOnFolder(String),
    ResizeCollectionsPanel(DragHandleMessage),
    ResizeFilterPanel(DragHandleMessage),
    ResizeMetadataPanel(DragHandleMessage),
    ActivateCollection(SampleCollection),
    RenameCollection(SampleCollection),
    HoverCollectionDropTarget(SampleCollection, Point),
    DropOnCollection(SampleCollection),
    BeginRenameSelected,
    CancelRename,
    BeginCreateSubfolder,
    RenameInput(TextInputMessage),
    NameFilterInput(TextInputMessage),
    TagFilterInput(TextInputMessage),
    SortFileColumn(String),
    ResizeFileColumn(String, DragHandleMessage),
    DragFileColumn(String, DragHandleMessage),
    CancelFileColumnDrag,
    ExitCollectionFocus,
    ToggleSimilarityAnchor(String),
}
