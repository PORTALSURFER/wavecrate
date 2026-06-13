mod collection;
mod drag_preview;
mod folder_tree;
mod layout;
mod navigation;
mod source;

use radiant::prelude as ui;

use crate::native_app::app::{GuiMessage, NativeAppState};
use crate::native_app::sample_library::folder_browser::commands::FolderBrowserMessage;

impl NativeAppState {
    pub(in crate::native_app) fn apply_folder_browser_message(
        &mut self,
        message: FolderBrowserMessage,
        context: &mut ui::UpdateContext<GuiMessage>,
    ) {
        match message {
            FolderBrowserMessage::AddSource => self.add_source_from_dialog(context),
            FolderBrowserMessage::SelectSource(id) => {
                self.select_folder_browser_source(id, context)
            }
            FolderBrowserMessage::OpenSourceContextMenu(source_id, position) => {
                self.open_source_context_menu(source_id, position);
            }
            FolderBrowserMessage::BeginRenameSelected => self.begin_folder_browser_rename(context),
            FolderBrowserMessage::CancelRename => {
                self.library.folder_browser.cancel_rename();
            }
            FolderBrowserMessage::BeginCreateSubfolder => {
                self.begin_folder_browser_subfolder_creation(context);
            }
            FolderBrowserMessage::RenameInput(message) => {
                self.apply_folder_browser_rename_input(message, context);
            }
            FolderBrowserMessage::TagFilterInput(message) => {
                self.apply_folder_browser_tag_filter_input(message);
            }
            FolderBrowserMessage::DropOnFolder(folder_id) => {
                self.drop_on_folder_browser_folder(folder_id, context);
            }
            FolderBrowserMessage::DropOnCollection(collection) => {
                self.drop_on_folder_browser_collection(collection, context);
            }
            FolderBrowserMessage::OpenFolderContextMenu(folder_id, position) => {
                self.open_folder_context_menu(folder_id, position);
            }
            FolderBrowserMessage::ActivateFolder(folder_id) => {
                self.activate_folder_browser_folder(folder_id, context);
            }
            FolderBrowserMessage::DragFolder(folder_id, drag) => {
                self.drag_folder_browser_folder(folder_id, drag, context);
            }
            FolderBrowserMessage::ActivateCollection(collection) => {
                self.activate_folder_browser_collection(collection, context);
            }
            FolderBrowserMessage::RenameCollection(collection) => {
                self.begin_collection_rename(collection, context);
            }
            FolderBrowserMessage::DragFileColumn(column_id, message) => {
                self.drag_file_column(column_id, message, context);
            }
            FolderBrowserMessage::CancelFileColumnDrag => {
                self.cancel_file_column_drag(context);
            }
            FolderBrowserMessage::ToggleSimilarityAnchor(file_id) => {
                self.toggle_similarity_anchor(file_id, context);
            }
            message => self.library.folder_browser.apply_message(message),
        }
    }
}
