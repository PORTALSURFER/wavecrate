use super::super::support::*;
use crate::app::state::{InlineFolderEdit, InlineFolderEditKind};

#[test]
fn renaming_folder_updates_entries_and_tree() -> Result<(), String> {
    let (mut controller, source) = dummy_controller();
    controller.library.sources.push(source.clone());
    let folder = source.root.join("old");
    std::fs::create_dir_all(&folder).unwrap();
    write_test_wav(&folder.join("clip.wav"), &[0.1, -0.1]);
    controller.set_wav_entries_for_tests(vec![sample_entry(
        "old/clip.wav",
        crate::sample_sources::Rating::NEUTRAL,
    )]);
    controller.rebuild_wav_lookup();
    controller.rebuild_browser_lists();
    controller.refresh_folder_browser_for_tests();

    controller.rename_folder(Path::new("old"), "new")?;

    assert!(!folder.exists());
    assert!(source.root.join("new/clip.wav").is_file());
    assert_eq!(
        controller.wav_entry(0).unwrap().relative_path,
        PathBuf::from("new/clip.wav")
    );
    assert!(
        controller
            .ui
            .sources
            .folders
            .rows
            .iter()
            .any(|row| row.path == PathBuf::from("new"))
    );
    Ok(())
}

#[test]
fn start_folder_rename_creates_inline_edit_with_select_all() -> Result<(), String> {
    let (mut controller, source) = dummy_controller();
    controller.library.sources.push(source.clone());
    controller.selection_state.ctx.selected_source = Some(source.id.clone());
    let folder = source.root.join("folder");
    std::fs::create_dir_all(&folder).unwrap();
    write_test_wav(&folder.join("clip.wav"), &[0.1, -0.1]);
    controller.set_wav_entries_for_tests(vec![sample_entry(
        "folder/clip.wav",
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
        .position(|row| row.path == PathBuf::from("folder"))
        .unwrap();
    controller.focus_folder_row(focus_row);

    controller.start_folder_rename();

    let draft = controller.ui.sources.folders.inline_edit.as_ref().unwrap();
    assert!(matches!(
        draft.kind,
        InlineFolderEditKind::Rename { ref target } if target == &PathBuf::from("folder")
    ));
    assert_eq!(draft.name, "folder");
    assert!(draft.focus_requested);
    assert!(draft.select_all_on_focus_requested);
    Ok(())
}

#[test]
fn start_folder_rename_rejects_root_folder() {
    let (mut controller, source) = dummy_controller();
    controller.library.sources.push(source.clone());
    controller.selection_state.ctx.selected_source = Some(source.id.clone());
    controller.refresh_folder_browser_for_tests();
    controller.focus_folder_row(0);

    controller.start_folder_rename();

    assert!(controller.ui.sources.folders.inline_edit.is_none());
    assert_eq!(
        controller.ui.status.status_tone,
        crate::app::state::StatusTone::Info
    );
    assert!(
        controller
            .ui
            .status
            .text
            .contains("Root folder cannot be renamed")
    );
}

#[test]
fn cancelling_folder_rename_clears_inline_edit() {
    let (mut controller, _source) = dummy_controller();
    controller.ui.sources.folders.inline_edit = Some(InlineFolderEdit {
        kind: InlineFolderEditKind::Rename {
            target: PathBuf::from("folder"),
        },
        name: "folder".into(),
        focus_requested: true,
        select_all_on_focus_requested: true,
    });

    controller.cancel_folder_rename();

    assert!(controller.ui.sources.folders.inline_edit.is_none());
}

#[test]
fn applying_pending_folder_rename_updates_tree_and_focus() -> Result<(), String> {
    let (mut controller, source) = dummy_controller();
    controller.library.sources.push(source.clone());
    controller.selection_state.ctx.selected_source = Some(source.id.clone());
    let folder = source.root.join("old");
    std::fs::create_dir_all(&folder).unwrap();
    write_test_wav(&folder.join("clip.wav"), &[0.1, -0.1]);
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

    assert!(controller.apply_pending_folder_rename());

    assert!(controller.ui.sources.folders.inline_edit.is_none());
    let focused = controller
        .ui
        .sources
        .folders
        .focused
        .expect("focused row after rename");
    assert_eq!(
        controller.ui.sources.folders.rows[focused].path,
        PathBuf::from("new")
    );
    assert!(source.root.join("new/clip.wav").is_file());
    Ok(())
}
