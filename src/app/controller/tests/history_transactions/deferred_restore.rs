use crate::app::controller::jobs::{FileOpResult, UndoFileJob, UndoFileOpResult, UndoFileOutcome};
use crate::app::controller::test_support::{prepare_with_source_and_wav_entries, sample_entry};
use crate::app::controller::undo::{UndoEntry, UndoExecution};
use crate::app::controller::*;
use std::path::{Path, PathBuf};

#[test]
fn deferred_meaningful_ui_restore_hooks_reapply_before_and_after_snapshots() {
    let (mut controller, source) = prepare_with_source_and_wav_entries(vec![
        sample_entry("clip.wav", crate::sample_sources::Rating::NEUTRAL),
        sample_entry(
            "clip_selection_001.wav",
            crate::sample_sources::Rating::KEEP_1,
        ),
    ]);
    let before_selection = SelectionRange::new(0.2, 0.6);
    let before_edit = SelectionRange::new(0.25, 0.55);
    controller.focus_browser_row_only(0);
    controller.set_browser_selected_paths(vec![PathBuf::from("clip.wav")]);
    controller.ui.browser.selection.autoscroll = false;
    controller.sample_view.wav.selected_wav = Some(PathBuf::from("clip.wav"));
    controller
        .selection_state
        .range
        .set_range(Some(before_selection));
    controller.apply_selection(Some(before_selection));
    controller.set_edit_selection_range(before_edit);
    controller.ui.waveform.view = crate::app::state::WaveformView {
        start: 0.1,
        end: 0.7,
    };
    controller.ui.waveform.cursor = Some(0.4);
    controller.ui.waveform.loop_enabled = false;
    let before = controller.capture_meaningful_ui_snapshot();

    controller.sample_view.wav.selected_wav = Some(PathBuf::from("clip_selection_001.wav"));
    controller.set_browser_selected_paths(vec![PathBuf::from("clip_selection_001.wav")]);
    controller.ui.browser.selection.autoscroll = true;
    controller.selection_state.range.set_range(None);
    controller.selection_state.edit_range.set_range(None);
    controller.apply_selection(None);
    controller.apply_edit_selection(None);
    controller.ui.waveform.view = crate::app::state::WaveformView {
        start: 0.0,
        end: 0.35,
    };
    controller.ui.waveform.cursor = Some(0.1);
    controller.ui.waveform.loop_enabled = true;
    let after = controller.capture_meaningful_ui_snapshot();

    let source_id = source.id.clone();
    let source_root = source.root.clone();
    let exported_relative = PathBuf::from("clip_selection_001.wav");
    let exported_absolute = source_root.join(&exported_relative);
    let backup_path = source_root.join("clip_selection_001.undo.wav");
    let entry = AppController::attach_meaningful_ui_restore(
        UndoEntry::new(
            "Deferred history restore",
            {
                let source_id = source_id.clone();
                let source_root = source_root.clone();
                let exported_relative = exported_relative.clone();
                let exported_absolute = exported_absolute.clone();
                move |_controller| {
                    Ok(UndoExecution::Deferred(UndoFileJob::RemoveSample {
                        source_id: source_id.clone(),
                        source_root: source_root.clone(),
                        relative_path: exported_relative.clone(),
                        absolute_path: exported_absolute.clone(),
                    }))
                }
            },
            move |_controller| {
                Ok(UndoExecution::Deferred(UndoFileJob::RestoreSample {
                    source_id: source_id.clone(),
                    source_root: source_root.clone(),
                    relative_path: exported_relative.clone(),
                    absolute_path: exported_absolute.clone(),
                    backup_path: backup_path.clone(),
                    tag: crate::sample_sources::Rating::KEEP_1,
                    looped: false,
                    last_played_at: None,
                    normal_tags: Vec::new(),
                }))
            },
        ),
        before.clone(),
        after.clone(),
    );
    controller.push_undo_entry(entry);

    controller.undo();
    controller.apply_file_op_result(FileOpResult::UndoFile(UndoFileOpResult {
        result: Ok(UndoFileOutcome::Removed {
            source_id: source.id.clone(),
            relative_path: PathBuf::from("clip_selection_001.wav"),
        }),
        cancelled: false,
    }));

    assert_eq!(
        controller.browser_selected_paths_snapshot(),
        vec![PathBuf::from("clip.wav")]
    );
    assert_eq!(
        controller.sample_view.wav.selected_wav.as_deref(),
        Some(Path::new("clip.wav"))
    );
    assert!(!controller.ui.browser.selection.autoscroll);
    assert_eq!(
        controller.selection_state.range.range(),
        Some(before_selection)
    );
    assert_eq!(
        controller.selection_state.edit_range.range(),
        Some(before_edit)
    );
    assert_eq!(
        controller.ui.waveform.view,
        crate::app::state::WaveformView {
            start: 0.1,
            end: 0.7,
        }
    );
    assert_eq!(controller.ui.waveform.cursor, Some(0.4));
    assert!(!controller.ui.waveform.loop_enabled);
}
