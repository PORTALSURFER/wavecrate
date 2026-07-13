use super::super::undo;
use super::super::*;
use crate::app::controller::jobs::UndoFileJob;
use std::cell::RefCell;
use std::path::PathBuf;
use std::rc::Rc;

#[derive(Clone)]
struct RestoreSampleMetadata {
    tag: crate::sample_sources::Rating,
    looped: bool,
    last_played_at: Option<i64>,
    normal_tags: Vec<String>,
}

impl AppController {
    pub(crate) fn selection_edit_undo_entry(
        &self,
        label: String,
        source_id: SourceId,
        relative_path: PathBuf,
        absolute_path: PathBuf,
        backup: undo::OverwriteBackup,
    ) -> undo::UndoEntry<AppController> {
        let before = backup.before.clone();
        let after = backup.after.clone();
        let backup_dir = backup.dir.clone();
        let undo_source_id = source_id.clone();
        let redo_source_id = source_id;
        let undo_relative = relative_path.clone();
        let redo_relative = relative_path;
        let undo_absolute = absolute_path.clone();
        let redo_absolute = absolute_path;
        undo::UndoEntry::<AppController>::new(
            label,
            move |controller: &mut AppController| {
                let source = controller
                    .library
                    .sources
                    .iter()
                    .find(|s| s.id == undo_source_id)
                    .cloned()
                    .ok_or_else(|| "Source not available".to_string())?;
                Ok(undo::UndoExecution::Deferred(UndoFileJob::Overwrite {
                    source_id: undo_source_id.clone(),
                    source_root: source.root,
                    relative_path: undo_relative.clone(),
                    absolute_path: undo_absolute.clone(),
                    backup_path: before.clone(),
                }))
            },
            move |controller: &mut AppController| {
                let source = controller
                    .library
                    .sources
                    .iter()
                    .find(|s| s.id == redo_source_id)
                    .cloned()
                    .ok_or_else(|| "Source not available".to_string())?;
                Ok(undo::UndoExecution::Deferred(UndoFileJob::Overwrite {
                    source_id: redo_source_id.clone(),
                    source_root: source.root,
                    relative_path: redo_relative.clone(),
                    absolute_path: redo_absolute.clone(),
                    backup_path: after.clone(),
                }))
            },
        )
        .with_cleanup_dir(backup_dir)
    }

    pub(crate) fn crop_new_sample_undo_entry(
        &self,
        label: String,
        source_id: SourceId,
        relative_path: PathBuf,
        absolute_path: PathBuf,
        tag: crate::sample_sources::Rating,
        looped: bool,
        last_played_at: Option<i64>,
        backup: undo::OverwriteBackup,
    ) -> undo::UndoEntry<AppController> {
        let after = backup.after.clone();
        let backup_dir = backup.dir.clone();
        let restore_metadata = Rc::new(RefCell::new(RestoreSampleMetadata {
            tag,
            looped,
            last_played_at,
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
                let source = controller
                    .library
                    .sources
                    .iter()
                    .find(|s| s.id == undo_source_id)
                    .cloned()
                    .ok_or_else(|| "Source not available".to_string())?;
                capture_current_restore_metadata(
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
                let source = controller
                    .library
                    .sources
                    .iter()
                    .find(|s| s.id == redo_source_id)
                    .cloned()
                    .ok_or_else(|| "Source not available".to_string())?;
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
                    normal_tags: metadata.normal_tags.clone(),
                }))
            },
        )
        .with_cleanup_dir(backup_dir)
    }
}

fn capture_current_restore_metadata(
    controller: &mut AppController,
    source: &SampleSource,
    relative_path: &std::path::Path,
    restore_metadata: &Rc<RefCell<RestoreSampleMetadata>>,
) {
    let mut metadata = restore_metadata.borrow().clone();
    if controller.selection_state.ctx.selected_source.as_ref() == Some(&source.id)
        && let Some(index) = controller.wav_index_for_path(relative_path)
        && let Some(entry) = controller.wav_entry(index)
    {
        metadata = RestoreSampleMetadata {
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
