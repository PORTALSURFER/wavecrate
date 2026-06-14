use radiant::prelude as ui;

use crate::native_app::app::{GuiMessage, NativeAppState};

impl NativeAppState {
    pub(super) fn apply_browser_dispatch(
        &mut self,
        message: GuiMessage,
        context: &mut ui::UiUpdateContext<GuiMessage>,
    ) {
        match message {
            GuiMessage::ResizeFolder(message) => self.resize_folder_browser(message),
            GuiMessage::AddSourceDialogFinished(result) => {
                self.finish_add_source_dialog(result, context);
            }
            GuiMessage::FolderBrowser(message) => {
                self.apply_folder_browser_message(message, context);
            }
            GuiMessage::PrepareSimilarityForSelectedSource => {
                self.prepare_similarity_for_selected_source(context);
            }
            GuiMessage::SimilarityPrepStatusResolved(result) => {
                self.finish_similarity_prep_status(result);
            }
            GuiMessage::SimilarityPrepEnqueueFinished(result) => {
                self.finish_similarity_prep_enqueue(result, context);
            }
            GuiMessage::FolderScanProgress(progress) => {
                self.apply_folder_scan_progress(progress);
            }
            GuiMessage::FolderScanDiscoveryBatch(batch) => {
                self.apply_folder_scan_discovery_batch(batch);
            }
            GuiMessage::FolderScanFinished(result) => self.finish_folder_scan(result, context),
            GuiMessage::StartupFolderVerifyFinished(ticket) => {
                self.finish_startup_folder_verify(ticket)
            }
            GuiMessage::SelectedFolderVerifyFinished(ticket) => self.finish_folder_verify(ticket),
            GuiMessage::SourceFilesystemChanged {
                source_id,
                paths,
                overflowed,
            } => {
                self.refresh_source_after_filesystem_change(source_id, paths, overflowed, context);
            }
            GuiMessage::NormalizationProgress(progress) => {
                self.apply_normalization_progress(progress);
            }
            GuiMessage::NormalizationFinished(result) => self.finish_normalization(result),
            GuiMessage::SelectSampleWithModifiers { path, modifiers } => {
                self.ui.browser_interaction.context_menu = None;
                self.select_sample_with_modifiers(path, modifiers, context);
            }
            GuiMessage::OpenSampleContextMenu { path, position } => {
                self.open_sample_context_menu(path, position);
            }
            GuiMessage::DragSampleFile { path, drag } => {
                self.ui.browser_interaction.context_menu = None;
                self.drag_sample_file(path, drag, context);
            }
            GuiMessage::ExternalDragCompleted(result) => {
                self.external_drag_completed(result, context)
            }
            _ => unreachable!("browser dispatcher received a non-browser message"),
        }
    }
}
