use std::{
    path::{Path, PathBuf},
    time::Instant,
};

use radiant::prelude as ui;

use crate::native_app::app::{
    FileMoveConflictResolution, FileMoveConflictResolutionRequest, FileMoveProgress, GuiMessage,
    NativeAppState, emit_gui_action,
};
use crate::native_app::sample_library::committed_file_mutations::{
    FileMutationChange, FileMutationOperation,
};
use crate::native_app::sample_library::folder_browser::commands::{
    FileMoveConflictCompletion, FolderDropResult, FolderMoveCompletion, FolderMoveDropInput,
    FolderMoveRequest, execute_file_move_conflict_request_with_progress,
    execute_folder_move_request_with_progress, file_move_conflict_progress_label,
    file_move_conflict_progress_total, folder_move_progress_label, folder_move_progress_total,
};
use crate::native_app::shell::message_dispatch::waveform::PLAY_SELECTION_TRANSACTION_LABEL;
use crate::native_app::transaction_history::TransactionContext;

impl NativeAppState {
    pub(in crate::native_app) fn drop_browser_drag_on_folder(
        &mut self,
        folder_id: String,
        context: &mut ui::UiUpdateContext<GuiMessage>,
    ) {
        let started_at = Instant::now();
        context.end_drag_session();
        self.clear_pending_internal_file_drag_paths();
        match self.library.folder_browser.drop_drag_on_folder(&folder_id) {
            Ok(FolderMoveDropInput::Status(result)) => {
                self.finish_folder_move_result(
                    started_at,
                    None,
                    Vec::new(),
                    None,
                    Ok(result),
                    context,
                );
            }
            Ok(FolderMoveDropInput::Request(request)) => {
                self.queue_folder_move_request(request, started_at, context);
            }
            Err(error) => {
                self.flash_protected_source_block_if_error(&error, Path::new(&folder_id));
                self.ui.status.sample =
                    self.protected_source_status_or_error(&error, Path::new(&folder_id));
                self.library.folder_browser.clear_drag();
                emit_gui_action(
                    "browser.drag_drop.move",
                    Some("browser"),
                    None,
                    "error",
                    started_at,
                    Some(&error),
                );
            }
        }
    }

    pub(in crate::native_app) fn drop_browser_drag_on_source(
        &mut self,
        source_id: String,
        context: &mut ui::UiUpdateContext<GuiMessage>,
    ) {
        let started_at = Instant::now();
        context.end_drag_session();
        self.clear_pending_internal_file_drag_paths();
        match self.library.folder_browser.drop_drag_on_source(&source_id) {
            Ok(FolderMoveDropInput::Status(result)) => {
                self.finish_folder_move_result(
                    started_at,
                    None,
                    Vec::new(),
                    None,
                    Ok(result),
                    context,
                );
            }
            Ok(FolderMoveDropInput::Request(request)) => {
                self.queue_folder_move_request(request, started_at, context);
            }
            Err(error) => {
                if let Some(path) = self.library.folder_browser.source_root_path(&source_id) {
                    self.flash_protected_source_block_if_error(&error, &path);
                    self.ui.status.sample = self.protected_source_status_or_error(&error, &path);
                } else {
                    self.ui.status.sample = error.clone();
                }
                self.library.folder_browser.clear_drag();
                emit_gui_action(
                    "browser.drag_drop.move",
                    Some("browser"),
                    None,
                    "error",
                    started_at,
                    Some(&error),
                );
            }
        }
    }

    pub(in crate::native_app) fn submit_folder_move_input(
        &mut self,
        input: FolderMoveDropInput,
        started_at: Instant,
        context: &mut ui::UiUpdateContext<GuiMessage>,
    ) -> Option<u64> {
        match input {
            FolderMoveDropInput::Status(result) => {
                self.finish_folder_move_result(
                    started_at,
                    None,
                    Vec::new(),
                    None,
                    Ok(result),
                    context,
                );
                None
            }
            FolderMoveDropInput::Request(request) => {
                Some(self.queue_folder_move_request(request, started_at, context))
            }
        }
    }

    fn queue_folder_move_request(
        &mut self,
        request: FolderMoveRequest,
        started_at: Instant,
        context: &mut ui::UiUpdateContext<GuiMessage>,
    ) -> u64 {
        let task_id = self.background.next_task_id();
        self.begin_file_move_progress(FileMoveProgress {
            task_id,
            label: folder_move_progress_label(&request),
            completed: 0,
            total: folder_move_progress_total(&request),
            detail: String::from("Queued"),
        });
        context
            .business()
            .background("gui-folder-browser-move")
            .stream_latest(
                move |_context, events| {
                    execute_folder_move_request_with_progress(request, task_id, move |progress| {
                        events.emit(progress)
                    })
                },
                GuiMessage::FileMoveProgress,
                move |completion: FolderMoveCompletion| GuiMessage::FolderMoveFinished {
                    started_at,
                    completion,
                },
            );
        task_id
    }

    pub(in crate::native_app) fn resolve_file_move_conflict(
        &mut self,
        request: FileMoveConflictResolutionRequest,
        context: &mut ui::UiUpdateContext<GuiMessage>,
    ) {
        let started_at = Instant::now();
        self.ui
            .browser_interaction
            .file_move_conflict_apply_to_remaining = false;
        if request.resolution != FileMoveConflictResolution::Skip
            && let Some(view) = self
                .library
                .folder_browser
                .pending_file_move_conflict_view()
        {
            let target_error = view.destination_path.parent().and_then(|target| {
                self.library
                    .folder_browser
                    .folder_target_lock_error(target, "File conflict")
            });
            let source_error = self
                .library
                .folder_browser
                .file_change_lock_error(&view.source_path, "File conflict");
            if let Some(error) = source_error.or(target_error) {
                self.flash_protected_source_block_if_error(&error, &view.source_path);
                self.ui.status.sample =
                    self.protected_source_status_or_error(&error, &view.source_path);
                emit_gui_action(
                    "browser.drag_drop.conflict",
                    Some("browser"),
                    None,
                    "blocked",
                    started_at,
                    Some(&error),
                );
                return;
            }
        }
        let Some(batch) = self.library.folder_browser.take_file_move_conflict_batch() else {
            self.finish_file_move_conflict_result(
                started_at,
                None,
                Vec::new(),
                None,
                Ok(Default::default()),
                context,
            );
            return;
        };
        if batch.current_index >= batch.conflicts.len() {
            self.finish_file_move_conflict_result(
                started_at,
                None,
                Vec::new(),
                None,
                Ok(FolderDropResult {
                    moved_paths: Vec::new(),
                    status: Some(String::from("No file move conflicts pending")),
                }),
                context,
            );
            return;
        }
        let task_id = self.background.next_task_id();
        self.begin_file_move_progress(FileMoveProgress {
            task_id,
            label: file_move_conflict_progress_label(&batch, request),
            completed: 0,
            total: file_move_conflict_progress_total(&batch, request),
            detail: String::from("Queued"),
        });
        context
            .business()
            .background("gui-file-move-conflict")
            .stream_latest(
                move |_context, events| {
                    execute_file_move_conflict_request_with_progress(
                        batch,
                        request,
                        task_id,
                        move |progress| events.emit(progress),
                    )
                },
                GuiMessage::FileMoveProgress,
                move |completion: FileMoveConflictCompletion| {
                    GuiMessage::FileMoveConflictFinished {
                        started_at,
                        completion,
                    }
                },
            );
    }

    pub(in crate::native_app) fn cancel_file_move_conflicts(&mut self) {
        self.ui
            .browser_interaction
            .file_move_conflict_apply_to_remaining = false;
        if let Some(status) = self.library.folder_browser.cancel_file_move_conflicts() {
            self.ui.status.sample = status;
        }
    }

    fn begin_file_move_progress(&mut self, progress: FileMoveProgress) {
        self.ui.status.sample = format!("{} | {}", progress.label, progress.detail);
        self.background.file_move_progress = Some(progress);
    }

    pub(in crate::native_app) fn apply_file_move_progress(&mut self, progress: FileMoveProgress) {
        if self
            .background
            .file_move_progress
            .as_ref()
            .is_some_and(|active| active.task_id == progress.task_id)
        {
            self.background.file_move_progress = Some(progress);
        }
    }

    fn finish_file_move_progress(&mut self, task_id: u64) {
        if self
            .background
            .file_move_progress
            .as_ref()
            .is_some_and(|active| active.task_id == task_id)
        {
            self.background.file_move_progress = None;
            self.background.progress_tick = 0.0;
        }
    }

    pub(in crate::native_app) fn apply_moved_sample_paths(
        &mut self,
        moved_paths: &[(PathBuf, PathBuf)],
    ) {
        for (old_path, new_path) in moved_paths {
            let loaded_path_moved = self
                .waveform
                .current
                .rewrite_path_prefix(old_path, new_path);
            self.remap_renamed_waveform_cache_path(old_path, new_path);
            if loaded_path_moved {
                let moved_file_id = self.waveform.current.path().to_string_lossy().to_string();
                self.reconcile_playback_mode_after_metadata_tag_change(&moved_file_id);
            }
        }
    }

    pub(in crate::native_app) fn finish_folder_move(
        &mut self,
        started_at: Instant,
        completion: FolderMoveCompletion,
        context: &mut ui::UiUpdateContext<GuiMessage>,
    ) {
        let task_id = completion.task_id;
        let request = completion.request;
        self.finish_file_move_progress(task_id);
        let previous_selected = self
            .library
            .folder_browser
            .selected_file_id()
            .map(str::to_owned);
        let committed_moves = completion
            .result
            .as_ref()
            .map(|success| success.moved_paths.clone())
            .unwrap_or_default();
        let metadata_error = completion
            .result
            .as_ref()
            .ok()
            .and_then(|success| success.metadata_error.clone());
        let committed_changes = folder_move_mutation_changes(&request, &committed_moves);
        let result = completion.result.and_then(|success| {
            let moved_paths = success.moved_paths.clone();
            self.remap_metadata_tags_for_moved_files(&moved_paths);
            let result = self.library.folder_browser.apply_folder_move_completion(
                &request,
                success,
                &self.metadata.tags_by_file,
            );
            if result.is_ok() {
                if let FolderMoveRequest::Folder {
                    source_root,
                    source_database_root,
                    ..
                } = &request
                    && !moved_paths.is_empty()
                {
                    self.register_folder_move_transaction(
                        source_root.clone(),
                        source_database_root.clone(),
                        moved_paths.clone(),
                    );
                }
                self.reconcile_harvest_graph_after_folder_move(&request, &moved_paths);
            }
            result
        });
        let cut_paste_succeeded = self
            .ui
            .browser_interaction
            .cut_file_paste_task_id
            .is_some_and(|paste_task_id| paste_task_id == task_id)
            && result.is_ok();
        self.finish_folder_move_result(
            started_at,
            previous_selected,
            committed_changes,
            metadata_error,
            result,
            context,
        );
        if self.ui.browser_interaction.cut_file_paste_task_id == Some(task_id) {
            self.ui.browser_interaction.cut_file_paste_task_id = None;
            if cut_paste_succeeded {
                self.ui.browser_interaction.cut_file_clipboard = None;
            }
        }
    }

    fn finish_folder_move_result(
        &mut self,
        started_at: Instant,
        previous_selected: Option<String>,
        committed_changes: Vec<FileMutationChange>,
        metadata_error: Option<String>,
        result: Result<FolderDropResult, String>,
        context: &mut ui::UiUpdateContext<GuiMessage>,
    ) {
        match result {
            Ok(result) => {
                let moved = !committed_changes.is_empty() || !result.moved_paths.is_empty();
                self.apply_moved_sample_paths(&result.moved_paths);
                if let Some(status) = result.status {
                    self.ui.status.sample = status;
                }
                if moved {
                    self.persist_source_scan_cache_after_move(
                        "browser.drag_drop.move.cache_persist",
                        started_at,
                    );
                }
                self.load_selected_sample_after_move_if_needed(previous_selected, moved, context);
                if moved {
                    self.queue_partially_committed_file_mutation(
                        FileMutationOperation::Move,
                        committed_changes,
                        metadata_error
                            .into_iter()
                            .map(|error| (None, error))
                            .collect(),
                        context,
                    );
                }
                emit_gui_action(
                    "browser.drag_drop.move",
                    Some("browser"),
                    None,
                    if result.moved_paths.is_empty() {
                        "unchanged"
                    } else {
                        "success"
                    },
                    started_at,
                    None,
                );
            }
            Err(error) => {
                if committed_changes.is_empty() {
                    self.record_rolled_back_file_mutation(
                        FileMutationOperation::Move,
                        None,
                        error.clone(),
                        context,
                    );
                } else {
                    let mut failures = vec![(None, error.clone())];
                    failures.extend(metadata_error.map(|error| (None, error)));
                    self.queue_partially_committed_file_mutation(
                        FileMutationOperation::Move,
                        committed_changes,
                        failures,
                        context,
                    );
                }
                self.ui.status.sample = error.clone();
                emit_gui_action(
                    "browser.drag_drop.move",
                    Some("browser"),
                    None,
                    "error",
                    started_at,
                    Some(&error),
                );
            }
        }
    }

    pub(in crate::native_app) fn finish_file_move_conflict(
        &mut self,
        started_at: Instant,
        completion: FileMoveConflictCompletion,
        context: &mut ui::UiUpdateContext<GuiMessage>,
    ) {
        self.finish_file_move_progress(completion.task_id);
        let previous_selected = self
            .library
            .folder_browser
            .selected_file_id()
            .map(str::to_owned);
        let moved_paths = match &completion.result {
            Ok(success) => success.moved_paths.clone(),
            Err(failure) => failure.moved_paths.clone(),
        };
        let metadata_error = match &completion.result {
            Ok(success) => success.metadata_error.clone(),
            Err(failure) => failure.metadata_error.clone(),
        };
        self.remap_metadata_tags_for_moved_files(&moved_paths);
        let result = self
            .library
            .folder_browser
            .apply_file_move_conflict_completion(completion, &self.metadata.tags_by_file);
        self.finish_file_move_conflict_result(
            started_at,
            previous_selected,
            moved_paths,
            metadata_error,
            result,
            context,
        );
    }

    fn finish_file_move_conflict_result(
        &mut self,
        started_at: Instant,
        previous_selected: Option<String>,
        committed_moves: Vec<(PathBuf, PathBuf)>,
        metadata_error: Option<String>,
        result: Result<FolderDropResult, String>,
        context: &mut ui::UiUpdateContext<GuiMessage>,
    ) {
        match result {
            Ok(result) => {
                let moved = !committed_moves.is_empty() || !result.moved_paths.is_empty();
                let mut authoritative_moves = committed_moves;
                for moved_path in &result.moved_paths {
                    if !authoritative_moves.contains(moved_path) {
                        authoritative_moves.push(moved_path.clone());
                    }
                }
                self.apply_moved_sample_paths(&result.moved_paths);
                if let Some(status) = result.status {
                    self.ui.status.sample = status;
                }
                if moved {
                    self.persist_source_scan_cache_after_move(
                        "browser.drag_drop.file_conflict.cache_persist",
                        started_at,
                    );
                }
                self.load_selected_sample_after_move_if_needed(previous_selected, moved, context);
                if moved {
                    self.queue_partially_committed_file_mutation(
                        FileMutationOperation::Move,
                        authoritative_moves
                            .into_iter()
                            .map(|(before, after)| {
                                FileMutationChange::path_only_move(before, after)
                            })
                            .collect(),
                        metadata_error
                            .into_iter()
                            .map(|error| (None, error))
                            .collect(),
                        context,
                    );
                }
                emit_gui_action(
                    "browser.drag_drop.file_conflict.resolve",
                    Some("browser"),
                    None,
                    if result.moved_paths.is_empty() {
                        "skipped"
                    } else {
                        "success"
                    },
                    started_at,
                    None,
                );
            }
            Err(error) => {
                let mut failures = vec![(None, error.clone())];
                failures.extend(metadata_error.map(|error| (None, error)));
                self.queue_partially_committed_file_mutation(
                    FileMutationOperation::Move,
                    committed_moves
                        .into_iter()
                        .map(|(before, after)| FileMutationChange::path_only_move(before, after))
                        .collect(),
                    failures,
                    context,
                );
                self.ui.status.sample = error.clone();
                emit_gui_action(
                    "browser.drag_drop.file_conflict.resolve",
                    Some("browser"),
                    None,
                    "error",
                    started_at,
                    Some(&error),
                );
            }
        }
    }

    fn persist_source_scan_cache_after_move(&mut self, action: &'static str, started_at: Instant) {
        if let Err(error) = self.library.folder_browser.save_source_scan_cache() {
            self.ui.status.sample = if self.ui.status.sample.is_empty() {
                format!("Source cache not saved: {error}")
            } else {
                format!("{}; source cache not saved: {error}", self.ui.status.sample)
            };
            emit_gui_action(
                action,
                Some("browser"),
                None,
                "error",
                started_at,
                Some(&error),
            );
        }
    }

    fn load_selected_sample_after_move_if_needed(
        &mut self,
        previous_selected: Option<String>,
        moved: bool,
        context: &mut ui::UiUpdateContext<GuiMessage>,
    ) {
        if !moved {
            return;
        }
        let Some(selected) = self
            .library
            .folder_browser
            .selected_file_id()
            .map(str::to_owned)
        else {
            return;
        };
        if previous_selected.as_deref() == Some(selected.as_str()) {
            return;
        }
        self.cancel_metadata_tag_entry();
        self.metadata.selected_tag = None;
        self.load_navigation_sample(selected, context);
    }

    fn register_folder_move_transaction(
        &mut self,
        source_root: PathBuf,
        source_database_root: PathBuf,
        moved_paths: Vec<(PathBuf, PathBuf)>,
    ) {
        self.discard_play_selection_transactions_for_moved_paths(&moved_paths);
        let undo_moves = moved_paths
            .iter()
            .map(|(old_path, new_path)| (new_path.clone(), old_path.clone()))
            .collect::<Vec<_>>();
        let redo_moves = moved_paths;
        let label = if redo_moves.len() == 1 {
            String::from("Move folder")
        } else {
            format!("Move {} folders", redo_moves.len())
        };
        let undo_source_root = source_root.clone();
        let undo_source_database_root = source_database_root.clone();
        self.begin_transaction(label);
        self.register_transaction_action(
            "Move folders",
            move |transaction| {
                transaction.apply_folder_move_paths(
                    &undo_source_root,
                    &undo_source_database_root,
                    &undo_moves,
                )
            },
            move |transaction| {
                transaction.apply_folder_move_paths(
                    &source_root,
                    &source_database_root,
                    &redo_moves,
                )
            },
        );
        self.commit_transaction();
    }

    fn discard_play_selection_transactions_for_moved_paths(
        &mut self,
        moved_paths: &[(PathBuf, PathBuf)],
    ) {
        let loaded_path_moved = moved_paths
            .iter()
            .any(|(old_path, _)| self.waveform.current.path().starts_with(old_path));
        if !loaded_path_moved {
            return;
        }
        self.waveform.pending_play_selection_transaction = None;
        self.transactions
            .history
            .remove_transactions_with_action_label(PLAY_SELECTION_TRANSACTION_LABEL);
    }

    fn apply_folder_move_paths_for_transaction(
        &mut self,
        source_root: &Path,
        source_database_root: &Path,
        moves: &[(PathBuf, PathBuf)],
    ) -> Result<(), String> {
        self.transactions.pending_file_mutation_attempted = true;
        let metadata_error = self.library.folder_browser.apply_folder_move_transaction(
            source_root,
            source_database_root,
            moves,
        )?;
        self.remap_metadata_tags_for_moved_files(moves);
        self.apply_moved_sample_paths(moves);
        let request = FolderMoveRequest::Folder {
            source_root: source_root.to_path_buf(),
            source_database_root: source_database_root.to_path_buf(),
            moves: moves.to_vec(),
            target_folder: moves
                .first()
                .and_then(|(_, new_path)| new_path.parent().map(Path::to_path_buf))
                .unwrap_or_else(|| source_root.to_path_buf()),
        };
        self.reconcile_harvest_graph_after_folder_move(&request, moves);
        if let Some(error) = metadata_error {
            tracing::warn!("folder move transaction metadata update failed: {error}");
            self.transactions.pending_file_mutation_failures.push(error);
        }
        if let Err(error) = self.library.folder_browser.save_source_scan_cache() {
            tracing::warn!("folder move transaction source cache save failed: {error}");
        }
        self.transactions
            .pending_file_mutations
            .extend(moves.iter().map(|(before, after)| {
                FileMutationChange::path_only_move(before.clone(), after.clone())
            }));
        Ok(())
    }
}

fn folder_move_mutation_changes(
    request: &FolderMoveRequest,
    moved_paths: &[(PathBuf, PathBuf)],
) -> Vec<FileMutationChange> {
    moved_paths
        .iter()
        .map(|(before, after)| {
            let copy_only = match request {
                FolderMoveRequest::SourcedFiles { file_moves, .. } => file_moves
                    .iter()
                    .find(|item| Path::new(&item.file_id) == before)
                    .is_some_and(|item| item.copy_only),
                _ => false,
            };
            if copy_only {
                FileMutationChange::created(after.clone())
            } else {
                FileMutationChange::path_only_move(before.clone(), after.clone())
            }
        })
        .collect()
}

impl TransactionContext<'_> {
    fn apply_folder_move_paths(
        &mut self,
        source_root: &Path,
        source_database_root: &Path,
        moves: &[(PathBuf, PathBuf)],
    ) -> Result<(), String> {
        self.state
            .apply_folder_move_paths_for_transaction(source_root, source_database_root, moves)
    }
}
