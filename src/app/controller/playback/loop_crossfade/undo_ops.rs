use super::*;
use crate::app::controller::jobs::UndoFileJob;
use crate::app::controller::undo;
use crate::sample_sources::Rating;
use std::path::PathBuf;

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
            Ok(undo::UndoExecution::Deferred(UndoFileJob::RemoveSample {
                source_id: undo_source_id.clone(),
                source_root: source.root,
                relative_path: undo_relative.clone(),
                absolute_path: undo_absolute.clone(),
            }))
        },
        move |controller: &mut AppController| {
            let source = super::loop_crossfade_source(controller, &redo_source_id)?;
            Ok(undo::UndoExecution::Deferred(UndoFileJob::RestoreSample {
                source_id: redo_source_id.clone(),
                source_root: source.root,
                relative_path: redo_relative.clone(),
                absolute_path: redo_absolute.clone(),
                backup_path: after.clone(),
                tag,
            }))
        },
    )
    .with_cleanup_dir(backup_dir)
}
