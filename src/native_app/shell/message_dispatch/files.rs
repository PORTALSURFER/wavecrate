use radiant::prelude as ui;

use crate::native_app::app::{GuiMessage, NativeAppState};

impl NativeAppState {
    pub(super) fn apply_file_dispatch(
        &mut self,
        message: GuiMessage,
        context: &mut ui::UpdateContext<GuiMessage>,
    ) {
        match message {
            GuiMessage::FocusLoadedFile => self.focus_loaded_file(context),
            GuiMessage::AdjustSelectedRating(delta) => self.adjust_selected_rating(delta, context),
            GuiMessage::AssignSelectedCollection(collection) => {
                self.assign_selected_collection(collection, context)
            }
            GuiMessage::RemoveContextSampleFromCollection => {
                self.remove_context_sample_from_collection(context)
            }
            GuiMessage::NormalizeSelectedSamples => self.normalize_selected_samples(context),
            GuiMessage::CopySelectedFiles => self.copy_selected_files(),
            GuiMessage::SetFileMoveConflictApplyToRemaining(apply_to_remaining) => {
                self.ui
                    .browser_interaction
                    .file_move_conflict_apply_to_remaining = apply_to_remaining;
            }
            GuiMessage::ResolveFileMoveConflict(request) => {
                self.resolve_file_move_conflict(request);
            }
            GuiMessage::CancelFileMoveConflicts => self.cancel_file_move_conflicts(),
            GuiMessage::CopyContextPath => self.copy_context_path(context),
            GuiMessage::TrashFolderDialogFinished(result) => {
                self.finish_trash_folder_dialog(result);
            }
            GuiMessage::ContextPathCopyFinished { kind, path, result } => {
                self.finish_context_path_copy(kind, path, result);
            }
            GuiMessage::OpenContextTarget => self.open_context_target(context),
            GuiMessage::MoveContextTargetToTrash => self.move_context_target_to_trash(),
            GuiMessage::ContextTargetOpenFinished { kind, path, result } => {
                self.finish_context_target_open(kind, path, result);
            }
            GuiMessage::RefreshContextSource => self.refresh_context_source(context),
            GuiMessage::RemoveContextSource => self.remove_context_source(),
            GuiMessage::CloseContextMenu => {
                self.ui.browser_interaction.context_menu = None;
            }
            GuiMessage::WaveformFileDrop(drop) => self.apply_native_file_drop(drop, context),
            _ => unreachable!("file dispatcher received a non-file message"),
        }
    }
}
