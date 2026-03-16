use super::super::*;
use crate::app::state::ProgressTaskKind;
use std::path::PathBuf;
#[cfg(test)]
use std::sync::atomic::Ordering;
#[cfg(not(test))]
use std::sync::mpsc::channel;
use std::sync::{Arc, atomic::AtomicBool};
use super::super::trash_move::{self, TrashMoveFinished};
#[cfg(test)]
use super::super::trash_move::TrashMoveMessage;
#[cfg(not(test))]
use super::super::trash_move::run_trash_move_task;
#[cfg(test)]
use super::super::trash_move::run_trash_move_task_with_progress;

impl AppController {
    /// Move all samples tagged as Trash into the configured trash folder after confirmation.
    pub fn move_all_trashed_to_folder(&mut self) {
        if self.runtime.jobs.trash_move_in_progress() {
            self.set_status("Trash move already in progress", StatusTone::Warning);
            return;
        }
        if self.settings.trash_folder.is_none() {
            self.pick_trash_folder();
            return;
        }
        if !self.confirm_warning(
            "Move trashed samples?",
            "All samples tagged as Trash will be moved to the configured trash folder. Continue?",
        ) {
            return;
        }
        let Ok(trash_root) = self.ensure_trash_folder_ready() else {
            return;
        };
        self.set_status("Moving trashed samples...", StatusTone::Busy);
        self.show_status_progress(
            ProgressTaskKind::TrashMove,
            "Moving trashed samples",
            0,
            true,
        );
        self.update_progress_detail("Scanning for trashed samples...");

        let cancel = Arc::new(AtomicBool::new(false));
        let sources = self.library.sources.clone();

        #[cfg(test)]
        {
            let cancel_after = self.runtime.progress_cancel_after;
            let finished = run_trash_move_task_with_progress(
                sources,
                trash_root,
                cancel.clone(),
                |message| match message {
                    TrashMoveMessage::SetTotal(total) => {
                        self.ui
                            .progress
                            .set_counts(total, self.ui.progress.completed);
                    }
                    TrashMoveMessage::Progress { completed, detail } => {
                        self.ui
                            .progress
                            .set_counts(self.ui.progress.total, completed);
                        self.ui.progress.set_detail(detail);
                        if let Some(cancel_after) = cancel_after
                            && completed >= cancel_after
                        {
                            cancel.store(true, Ordering::Relaxed);
                        }
                    }
                    TrashMoveMessage::Finished(_) => {}
                },
                trash_move::move_to_trash,
            );
            self.apply_trash_move_finished(finished);
        }

        #[cfg(not(test))]
        {
            let (tx, rx) = channel();
            self.runtime.jobs.start_trash_move(rx, cancel.clone());
            std::thread::spawn(move || {
                let _ = run_trash_move_task(sources, trash_root, cancel, Some(&tx));
            });
        }
    }

    /// Move specific samples into the configured trash folder and refresh affected sources.
    pub(crate) fn move_samples_to_configured_trash(
        &mut self,
        samples: Vec<(SampleSource, WavEntry)>,
        next_focus: Option<PathBuf>,
    ) -> bool {
        if samples.is_empty() {
            return false;
        }
        let Some(trash_root) = self.prepare_trash_folder_for_auto_move() else {
            return false;
        };
        let mut errors = Vec::new();
        let mut moved = 0usize;
        let total = samples.len();
        let mut affected_sources = std::collections::HashSet::new();
        for (source, entry) in samples {
            let db = match SourceDatabase::open(&source.root) {
                Ok(db) => db,
                Err(err) => {
                    errors.push(format!("{}: {err}", source.root.display()));
                    continue;
                }
            };
            if let Err(err) = db.set_missing(&entry.relative_path, true) {
                errors.push(format!(
                    "Failed to mark {} as missing before move: {err}",
                    entry.relative_path.display()
                ));
                continue;
            }
            match trash_move::move_to_trash(&source, &entry, &trash_root) {
                Ok(()) => {
                    if let Err(err) = db.remove_file(&entry.relative_path) {
                        errors.push(format!(
                            "Failed to drop database row for {}: {err}",
                            entry.relative_path.display()
                        ));
                    } else {
                        moved += 1;
                        affected_sources.insert(source.id.clone());
                    }
                }
                Err(err) => {
                    errors.push(err);
                    if let Err(rollback_err) = db.set_missing(&entry.relative_path, false) {
                        errors.push(format!(
                            "Failed to rollback missing status for {}: {rollback_err}",
                            entry.relative_path.display()
                        ));
                    }
                }
            }
        }
        self.apply_trash_move_finished(TrashMoveFinished {
            total,
            moved,
            cancelled: false,
            errors,
            affected_sources: affected_sources.into_iter().collect(),
        });
        self.refocus_path_after_trash_move(next_focus);
        moved > 0
    }

    pub(super) fn refocus_path_after_trash_move(&mut self, next_focus: Option<PathBuf>) {
        let Some(path) = next_focus else {
            return;
        };
        if self.wav_index_for_path(&path).is_none() {
            return;
        }
        if let Some(row) = self.visible_row_for_path(&path) {
            self.focus_browser_row_only(row);
        } else {
            self.select_wav_by_path_with_rebuild(&path, true);
        }
    }
}
