use super::{gui_state_for_span_tests, native_app_state_with_temp_sample, run_command_for_tests};
use crate::native_app::test_support::state::{FolderBrowserMessage, FolderBrowserState, view};
use radiant::{
    gui::types::{Point, Vector2},
    prelude::IntoView,
};
use std::fs;

#[test]
fn file_move_conflict_dialog_renders_resolution_choices() {
    let mut state = gui_state_for_span_tests();
    let source_root = tempfile::tempdir().expect("source root");
    let drums = source_root.path().join("drums");
    let loops = source_root.path().join("loops");
    fs::create_dir_all(&drums).expect("create drums folder");
    fs::create_dir_all(&loops).expect("create loops folder");
    let source = drums.join("kick.wav");
    let destination = loops.join("kick.wav");
    fs::write(&source, b"source").expect("write source");
    fs::write(&destination, b"destination").expect("write destination");
    state.library.folder_browser =
        FolderBrowserState::from_sample_sources(&[wavecrate::sample_sources::SampleSource::new(
            source_root.path().to_path_buf(),
        )]);
    state
        .library
        .folder_browser
        .apply_message(FolderBrowserMessage::ActivateFolder(
            drums.display().to_string(),
        ));
    state
        .library
        .folder_browser
        .select_file(source.display().to_string());
    state
        .library
        .folder_browser
        .begin_file_drag(source.display().to_string(), Point::new(4.0, 8.0));
    let mut context = radiant::prelude::UpdateContext::default();
    state.drop_browser_drag_on_folder(loops.display().to_string(), &mut context);
    run_command_for_tests(&mut state, context.into_command());

    let frame = view(&mut state).view_frame_at_size_with_default_theme(Vector2::new(900.0, 620.0));

    assert!(frame.paint_plan.contains_text("File Move Conflict"));
    assert!(frame.paint_plan.contains_text("Conflict 1 of 1"));
    assert!(frame.paint_plan.contains_text("kick.wav"));
    assert!(
        frame
            .paint_plan
            .contains_text("Apply to all remaining conflicts")
    );
    assert!(frame.paint_plan.contains_text("Overwrite"));
    assert!(frame.paint_plan.contains_text("Rename"));
    assert!(frame.paint_plan.contains_text("Skip"));
}

#[test]
fn activating_folder_queues_selected_folder_verify() {
    let mut state = gui_state_for_span_tests();
    let source_root = tempfile::tempdir().expect("source root");
    let drums = source_root.path().join("drums");
    fs::create_dir_all(&drums).expect("create drums folder");
    fs::write(drums.join("kick.wav"), b"sample").expect("write sample");
    state.library.folder_browser =
        FolderBrowserState::from_sample_sources(&[wavecrate::sample_sources::SampleSource::new(
            source_root.path().to_path_buf(),
        )]);
    let mut context = radiant::prelude::UpdateContext::default();

    state.apply_folder_browser_message(
        FolderBrowserMessage::ActivateFolder(drums.display().to_string()),
        &mut context,
    );

    assert!(
        state.background.folder_verify_task.active().is_some(),
        "activating a folder should schedule direct verification to reconcile stale rows"
    );
}

#[test]
fn activating_folder_replaces_pending_selected_folder_verify() {
    let mut state = gui_state_for_span_tests();
    let source_root = tempfile::tempdir().expect("source root");
    let drums = source_root.path().join("drums");
    let loops = source_root.path().join("loops");
    fs::create_dir_all(&drums).expect("create drums folder");
    fs::create_dir_all(&loops).expect("create loops folder");
    state.library.folder_browser =
        FolderBrowserState::from_sample_sources(&[wavecrate::sample_sources::SampleSource::new(
            source_root.path().to_path_buf(),
        )]);
    let mut context = radiant::prelude::UpdateContext::default();

    state.apply_folder_browser_message(
        FolderBrowserMessage::ActivateFolder(drums.display().to_string()),
        &mut context,
    );
    let first_ticket = state
        .background
        .folder_verify_task
        .active()
        .expect("first activation should queue verify");
    state.apply_folder_browser_message(
        FolderBrowserMessage::ActivateFolder(loops.display().to_string()),
        &mut context,
    );

    assert_ne!(
        state.background.folder_verify_task.active(),
        Some(first_ticket),
        "new folder activation should supersede an older pending verification"
    );
}

#[test]
fn delete_selected_file_moves_it_to_configured_trash_folder() {
    let mut state = gui_state_for_span_tests();
    let source_root = tempfile::tempdir().expect("source root");
    let trash_root = tempfile::tempdir().expect("trash root");
    let keep = source_root.path().join("keep.wav");
    let delete = source_root.path().join("delete.wav");
    fs::write(&keep, []).expect("write keep wav");
    fs::write(&delete, []).expect("write delete wav");
    state.ui.settings.persisted.trash_folder = Some(trash_root.path().to_path_buf());
    state.library.folder_browser =
        FolderBrowserState::from_sample_sources(&[wavecrate::sample_sources::SampleSource::new(
            source_root.path().to_path_buf(),
        )]);
    state
        .library
        .folder_browser
        .select_file(delete.display().to_string());

    let mut context = radiant::prelude::UpdateContext::default();
    state.delete_selected_item(&mut context);
    run_command_for_tests(&mut state, context.into_command());

    assert!(!delete.exists());
    assert!(trash_root.path().join("delete.wav").exists());
    assert!(keep.exists());
    assert_eq!(state.library.folder_browser.selected_file_id(), None);
    assert!(
        state
            .library
            .folder_browser
            .selected_audio_files()
            .iter()
            .any(|file| file.name == "keep.wav")
    );
    assert!(
        !state
            .library
            .folder_browser
            .selected_audio_files()
            .iter()
            .any(|file| file.name == "delete.wav")
    );
    assert!(state.ui.status.sample.contains("Moved 1 file to trash"));
}

#[test]
fn delete_selected_folder_moves_it_to_configured_trash_folder() {
    let mut state = gui_state_for_span_tests();
    let source_root = tempfile::tempdir().expect("source root");
    let trash_root = tempfile::tempdir().expect("trash root");
    let drums = source_root.path().join("drums");
    let loops = source_root.path().join("loops");
    fs::create_dir_all(&drums).expect("create drums folder");
    fs::create_dir_all(&loops).expect("create loops folder");
    fs::write(drums.join("kick.wav"), []).expect("write kick wav");
    fs::write(loops.join("loop.wav"), []).expect("write loop wav");
    state.ui.settings.persisted.trash_folder = Some(trash_root.path().to_path_buf());
    state.library.folder_browser =
        FolderBrowserState::from_sample_sources(&[wavecrate::sample_sources::SampleSource::new(
            source_root.path().to_path_buf(),
        )]);
    state
        .library
        .folder_browser
        .apply_message(FolderBrowserMessage::ActivateFolder(
            drums.display().to_string(),
        ));

    let mut context = radiant::prelude::UpdateContext::default();
    state.delete_selected_item(&mut context);
    run_command_for_tests(&mut state, context.into_command());

    assert!(!drums.exists());
    assert!(trash_root.path().join("drums").join("kick.wav").exists());
    assert!(loops.exists());
    assert_eq!(state.library.folder_browser.selected_file_id(), None);
    state
        .library
        .folder_browser
        .apply_message(FolderBrowserMessage::ActivateFolder(
            loops.display().to_string(),
        ));
    assert!(
        state
            .library
            .folder_browser
            .selected_audio_files()
            .iter()
            .any(|file| file.name == "loop.wav")
    );
    assert!(state.ui.status.sample.contains("Moved drums to trash"));
}

#[test]
fn delete_selected_file_requires_configured_trash_folder() {
    let (mut state, _source_root, selected_file) = native_app_state_with_temp_sample("blocked.wav");

    let mut context = radiant::prelude::UpdateContext::default();
    state.delete_selected_item(&mut context);
    run_command_for_tests(&mut state, context.into_command());

    assert!(std::path::Path::new(&selected_file).exists());
    assert_eq!(
        state.library.folder_browser.selected_file_id(),
        Some(selected_file.as_str())
    );
    assert!(
        state
            .ui
            .status
            .sample
            .contains("Set a trash folder in Settings > General"),
        "{}",
        state.ui.status.sample
    );
}
