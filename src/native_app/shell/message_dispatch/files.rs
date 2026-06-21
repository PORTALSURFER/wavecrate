use radiant::prelude as ui;

use crate::native_app::app::{GuiMessage, NativeAppState};

impl NativeAppState {
    pub(super) fn apply_file_dispatch(
        &mut self,
        message: GuiMessage,
        context: &mut ui::UiUpdateContext<GuiMessage>,
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
            GuiMessage::CopySelectedFiles => self.copy_selected_files(context),
            GuiMessage::SelectedFilesCopyFinished {
                count,
                started_at,
                result,
            } => self.finish_copy_selected_files(count, started_at, result),
            GuiMessage::WaveformSelectionCopyFinished {
                source_path,
                selection,
                started_at,
                result,
            } => self.finish_waveform_selection_copy(source_path, selection, started_at, result),
            GuiMessage::FileMoveProgress(progress) => self.apply_file_move_progress(progress),
            GuiMessage::SetFileMoveConflictApplyToRemaining(apply_to_remaining) => {
                self.ui
                    .browser_interaction
                    .file_move_conflict_apply_to_remaining = apply_to_remaining;
            }
            GuiMessage::ResolveFileMoveConflict(request) => {
                self.resolve_file_move_conflict(request, context);
            }
            GuiMessage::FolderMoveFinished {
                started_at,
                completion,
            } => {
                self.finish_folder_move(started_at, completion, context);
            }
            GuiMessage::FileMoveConflictFinished {
                started_at,
                completion,
            } => {
                self.finish_file_move_conflict(started_at, completion, context);
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
            GuiMessage::CreateFolderAtContextTarget => {
                self.create_folder_at_context_target(context)
            }
            GuiMessage::RenameContextFolder => self.rename_context_folder(context),
            GuiMessage::ContextFolderCreateFinished {
                parent_id,
                started_at,
                result,
            } => self.finish_context_folder_create(parent_id, started_at, result, context),
            GuiMessage::MoveContextTargetToTrash => self.move_context_target_to_trash(context),
            GuiMessage::ToggleContextFolderLock => self.toggle_context_folder_lock(),
            GuiMessage::RequestDeleteContextFolder => self.request_delete_context_folder(),
            GuiMessage::ConfirmContextFolderDelete => self.confirm_context_folder_delete(context),
            GuiMessage::CancelContextFolderDelete => self.cancel_context_folder_delete(),
            GuiMessage::TrashMoveFinished {
                target,
                action,
                started_at,
                result,
            } => self.finish_trash_move(target, action, started_at, result, context),
            GuiMessage::ContextTargetOpenFinished { kind, path, result } => {
                self.finish_context_target_open(kind, path, result);
            }
            GuiMessage::RefreshContextSource => self.refresh_context_source(context),
            GuiMessage::ProcessContextSource => self.process_context_source(context),
            GuiMessage::RemoveContextSource => self.remove_context_source(),
            GuiMessage::CloseContextMenu => {
                self.ui.browser_interaction.context_menu = None;
            }
            GuiMessage::ExternalWaveformFileDropFinished {
                source,
                started_at,
                result,
            } => self.finish_external_waveform_file_drop(source, started_at, result, context),
            GuiMessage::WaveformFileDrop(drop) => self.apply_native_file_drop(drop, context),
            _ => unreachable!("file dispatcher received a non-file message"),
        }
    }
}
