use std::path::{Path, PathBuf};
#[cfg(not(test))]
use std::sync::mpsc::Sender;
use std::sync::{
    Arc,
    atomic::{AtomicBool, Ordering},
};

use super::super::*;
#[cfg(not(test))]
use super::filesystem::move_to_trash;
use super::{TrashMoveFinished, TrashMoveMessage};

#[cfg(not(test))]
pub(crate) fn run_trash_move_task(
    sources: Vec<SampleSource>,
    trash_root: PathBuf,
    cancel: Arc<AtomicBool>,
    sender: Option<&Sender<TrashMoveMessage>>,
) -> TrashMoveFinished {
    run_trash_move_task_with_progress(
        sources,
        trash_root,
        cancel,
        |message| {
            if let Some(tx) = sender {
                let _ = tx.send(message);
            }
        },
        move_to_trash,
    )
}

pub(crate) fn run_trash_move_task_with_progress<F, M>(
    sources: Vec<SampleSource>,
    trash_root: PathBuf,
    cancel: Arc<AtomicBool>,
    mut on_message: F,
    mut mover: M,
) -> TrashMoveFinished
where
    F: FnMut(TrashMoveMessage),
    M: FnMut(&SampleSource, &WavEntry, &Path) -> Result<(), String>,
{
    let mut errors = Vec::new();
    let mut trashed_by_source: Vec<(SampleSource, Vec<WavEntry>)> = Vec::new();
    for source in sources {
        if cancel.load(Ordering::Relaxed) {
            break;
        }
        let db = match SourceDatabase::open(&source.root) {
            Ok(db) => db,
            Err(err) => {
                errors.push(format!("{}: {err}", source.root.display()));
                continue;
            }
        };
        let trashed = match db.list_files_by_tag(crate::sample_sources::Rating::TRASH_3) {
            Ok(entries) => entries,
            Err(err) => {
                errors.push(format!("{}: {err}", source.root.display()));
                continue;
            }
        };
        if !trashed.is_empty() {
            trashed_by_source.push((source, trashed));
        }
    }

    let total: usize = trashed_by_source
        .iter()
        .map(|(_, entries)| entries.len())
        .sum();
    on_message(TrashMoveMessage::SetTotal(total));

    if total == 0 {
        let finished = TrashMoveFinished {
            total,
            moved: 0,
            cancelled: cancel.load(Ordering::Relaxed),
            errors,
            affected_sources: Vec::new(),
        };
        on_message(TrashMoveMessage::Finished(finished.clone()));
        return finished;
    }

    let mut moved = 0usize;
    let mut completed = 0usize;
    let mut affected_sources: std::collections::HashSet<SourceId> =
        std::collections::HashSet::new();

    for (source, entries) in trashed_by_source {
        if cancel.load(Ordering::Relaxed) {
            break;
        }
        let db = match SourceDatabase::open(&source.root) {
            Ok(db) => db,
            Err(err) => {
                errors.push(format!("{}: {err}", source.root.display()));
                continue;
            }
        };
        for entry in entries {
            if cancel.load(Ordering::Relaxed) {
                break;
            }
            let detail = format!("Moving {}", entry.relative_path.display());
            if completed.is_multiple_of(5) {
                on_message(TrashMoveMessage::Progress {
                    completed,
                    detail: Some(detail.clone()),
                });
            }

            if let Err(err) = db.set_missing(&entry.relative_path, true) {
                errors.push(format!(
                    "Failed to mark {} as missing before move: {err}",
                    entry.relative_path.display()
                ));
                completed += 1;
                continue;
            }

            match mover(&source, &entry, &trash_root) {
                Ok(()) => {
                    moved += 1;
                    affected_sources.insert(source.id.clone());
                    if let Err(err) = db.remove_file(&entry.relative_path) {
                        errors.push(format!(
                            "Moved {} to trash but retained it as missing because the database row could not be dropped: {err}",
                            entry.relative_path.display()
                        ));
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

            completed += 1;
            on_message(TrashMoveMessage::Progress {
                completed,
                detail: Some(detail),
            });
        }
    }

    let finished = TrashMoveFinished {
        total,
        moved,
        cancelled: cancel.load(Ordering::Relaxed),
        errors,
        affected_sources: affected_sources.into_iter().collect(),
    };
    on_message(TrashMoveMessage::Finished(finished.clone()));
    finished
}
