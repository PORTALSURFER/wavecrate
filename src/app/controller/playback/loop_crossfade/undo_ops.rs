use super::*;
use crate::app::controller::jobs::UndoFileJob;
use crate::app::controller::undo;
use crate::sample_sources::Rating;
use std::cell::RefCell;
use std::path::PathBuf;
use std::rc::Rc;

#[derive(Clone)]
struct LoopCrossfadeRestoreMetadata {
    tag: Rating,
    looped: bool,
    last_played_at: Option<i64>,
    normal_tags: Vec<String>,
}

/// Capture deferred undo/redo jobs for one generated loop-crossfade copy.
pub(super) fn maybe_capture_loop_crossfade_undo(
    controller: &mut AppController,
    source: &SampleSource,
    output: &file_output::LoopCrossfadeFileOutput,
    tag: Rating,
) {
    let Ok(backup) = undo::OverwriteBackup::capture_before(&output.absolute_path) else {
        return;
    };
    if backup.capture_after(&output.absolute_path).is_ok() {
        controller.push_undo_entry(loop_crossfade_undo_entry(
            format!("Loop crossfaded {}", output.relative_path.display()),
            source.id.clone(),
            output.relative_path.clone(),
            output.absolute_path.clone(),
            tag,
            backup,
        ));
    }
}

/// Build the deferred undo/redo entry used for generated crossfade files.
fn loop_crossfade_undo_entry(
    label: String,
    source_id: SourceId,
    relative_path: PathBuf,
    absolute_path: PathBuf,
    tag: Rating,
    backup: undo::OverwriteBackup,
) -> undo::UndoEntry<AppController> {
    let after = backup.after.clone();
    let backup_dir = backup.dir.clone();
    let restore_metadata = Rc::new(RefCell::new(LoopCrossfadeRestoreMetadata {
        tag,
        looped: true,
        last_played_at: None,
        normal_tags: Vec::new(),
    }));
    let undo_restore_metadata = Rc::clone(&restore_metadata);
    let redo_restore_metadata = restore_metadata;
    let undo_source_id = source_id.clone();
    let redo_source_id = source_id;
    let undo_relative = relative_path.clone();
    let redo_relative = relative_path;
    let undo_absolute = absolute_path.clone();
    let redo_absolute = absolute_path;
    undo::UndoEntry::<AppController>::new(
        label,
        move |controller: &mut AppController| {
            let source = super::loop_crossfade_source(controller, &undo_source_id)?;
            capture_current_loop_crossfade_restore_metadata(
                controller,
                &source,
                &undo_relative,
                &undo_restore_metadata,
            );
            Ok(undo::UndoExecution::Deferred(UndoFileJob::RemoveSample {
                source_id: undo_source_id.clone(),
                source_root: source.root,
                relative_path: undo_relative.clone(),
                absolute_path: undo_absolute.clone(),
            }))
        },
        move |controller: &mut AppController| {
            let source = super::loop_crossfade_source(controller, &redo_source_id)?;
            let metadata = redo_restore_metadata.borrow().clone();
            Ok(undo::UndoExecution::Deferred(UndoFileJob::RestoreSample {
                source_id: redo_source_id.clone(),
                source_root: source.root,
                relative_path: redo_relative.clone(),
                absolute_path: redo_absolute.clone(),
                backup_path: after.clone(),
                tag: metadata.tag,
                looped: metadata.looped,
                last_played_at: metadata.last_played_at,
                normal_tags: metadata.normal_tags,
            }))
        },
    )
    .with_cleanup_dir(backup_dir)
}

fn capture_current_loop_crossfade_restore_metadata(
    controller: &mut AppController,
    source: &SampleSource,
    relative_path: &std::path::Path,
    restore_metadata: &Rc<RefCell<LoopCrossfadeRestoreMetadata>>,
) {
    let mut metadata = restore_metadata.borrow().clone();
    if controller.selection_state.ctx.selected_source.as_ref() == Some(&source.id)
        && let Some(index) = controller.wav_index_for_path(relative_path)
        && let Some(entry) = controller.wav_entry(index)
    {
        metadata = LoopCrossfadeRestoreMetadata {
            tag: entry.tag,
            looped: entry.looped,
            last_played_at: entry.last_played_at,
            normal_tags: entry.normal_tags.clone(),
        };
    }
    if let Ok(db) = crate::sample_sources::SourceDatabase::open_for_background_job(&source.root) {
        if let Ok(Some(tag)) = db.tag_for_path(relative_path) {
            metadata.tag = tag;
        }
        if let Ok(Some(looped)) = db.looped_for_path(relative_path) {
            metadata.looped = looped;
        }
        if let Ok(last_played_at) = db.last_played_at_for_path(relative_path) {
            metadata.last_played_at = metadata.last_played_at.or(last_played_at);
        }
        if let Ok(normal_tags) = db.tag_labels_for_path(relative_path) {
            metadata.normal_tags = normal_tags;
        }
    }
    *restore_metadata.borrow_mut() = metadata;
}
