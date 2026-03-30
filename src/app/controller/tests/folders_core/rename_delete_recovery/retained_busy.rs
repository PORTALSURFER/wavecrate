use super::super::support::*;
use crate::app::controller::jobs::{
    ActiveRetainedDeleteResolution, RetainedDeleteBusyEntry, RetainedDeleteResolutionMode,
};
use crate::app::state::{InlineFolderEdit, InlineFolderEditKind};

#[test]
fn deleting_folder_warns_when_retained_recovery_is_processing_the_folder() -> Result<(), String> {
    let (mut controller, source) = dummy_controller();
    controller.library.sources.push(source.clone());
    controller.selection_state.ctx.selected_source = Some(source.id.clone());
    let target = source.root.join("gone");
    std::fs::create_dir_all(&target).unwrap();
    write_test_wav(&target.join("sample.wav"), &[0.0, 0.2]);
    controller.set_wav_entries_for_tests(vec![sample_entry(
        "gone/sample.wav",
        crate::sample_sources::Rating::NEUTRAL,
    )]);
    controller.rebuild_wav_lookup();
    controller.rebuild_browser_lists();
    controller.refresh_folder_browser_for_tests();
    let focus_row = controller
        .ui
        .sources
        .folders
        .rows
        .iter()
        .position(|row| row.path == PathBuf::from("gone"))
        .unwrap();
    controller.focus_folder_row(focus_row);
    controller.runtime.active_retained_delete_resolution = Some(ActiveRetainedDeleteResolution {
        entries: vec![RetainedDeleteBusyEntry {
            mode: RetainedDeleteResolutionMode::Restore,
            source_id: source.id.clone(),
            source_label: "source".into(),
            relative_path: PathBuf::from("gone"),
        }],
    });

    controller.delete_focused_folder();

    assert!(target.exists());
    assert_eq!(
        controller.ui.status.status_tone,
        crate::app::state::StatusTone::Warning
    );
    assert!(
        controller
            .ui
            .status
            .text
            .contains("Recovery is still restoring")
    );
    Ok(())
}

#[test]
fn applying_pending_folder_rename_warns_when_retained_recovery_is_processing_the_folder() {
    let (mut controller, source) = dummy_controller();
    controller.library.sources.push(source.clone());
    controller.selection_state.ctx.selected_source = Some(source.id.clone());
    let target = source.root.join("old");
    std::fs::create_dir_all(&target).unwrap();
    write_test_wav(&target.join("clip.wav"), &[0.1, -0.1]);
    controller.set_wav_entries_for_tests(vec![sample_entry(
        "old/clip.wav",
        crate::sample_sources::Rating::NEUTRAL,
    )]);
    controller.rebuild_wav_lookup();
    controller.rebuild_browser_lists();
    controller.refresh_folder_browser_for_tests();
    controller.ui.sources.folders.inline_edit = Some(InlineFolderEdit {
        kind: InlineFolderEditKind::Rename {
            target: PathBuf::from("old"),
        },
        name: "new".into(),
        focus_requested: true,
        select_all_on_focus_requested: true,
    });
    controller.runtime.active_retained_delete_resolution = Some(ActiveRetainedDeleteResolution {
        entries: vec![RetainedDeleteBusyEntry {
            mode: RetainedDeleteResolutionMode::Restore,
            source_id: source.id.clone(),
            source_label: "source".into(),
            relative_path: PathBuf::from("old"),
        }],
    });

    assert!(controller.apply_pending_folder_rename());

    assert!(controller.ui.sources.folders.inline_edit.is_some());
    assert!(target.exists());
    assert!(!source.root.join("new").exists());
    assert_eq!(
        controller.ui.status.status_tone,
        crate::app::state::StatusTone::Warning
    );
    assert!(
        controller
            .ui
            .status
            .text
            .contains("Recovery is still restoring")
    );
}
