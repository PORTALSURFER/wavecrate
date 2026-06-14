use super::{gui_state_for_span_tests, native_app_state_with_temp_sample, run_command_for_tests};
use crate::native_app::test_support::state::{
    FolderBrowserMessage, FolderBrowserState, GuiMessage, view,
};
use radiant::{
    gui::types::{Point, Vector2},
    prelude::IntoView,
};
use std::fs;
use wavecrate::sample_sources::Rating;

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
            Default::default(),
        ));
    state
        .library
        .folder_browser
        .select_file(source.display().to_string());
    state
        .library
        .folder_browser
        .begin_file_drag(source.display().to_string(), Point::new(4.0, 8.0));
    let mut context = radiant::prelude::UiUpdateContext::default();
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
    let mut context = radiant::prelude::UiUpdateContext::default();

    state.apply_folder_browser_message(
        FolderBrowserMessage::ActivateFolder(drums.display().to_string(), Default::default()),
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
    let mut context = radiant::prelude::UiUpdateContext::default();

    state.apply_folder_browser_message(
        FolderBrowserMessage::ActivateFolder(drums.display().to_string(), Default::default()),
        &mut context,
    );
    let first_ticket = state
        .background
        .folder_verify_task
        .active()
        .expect("first activation should queue verify");
    state.apply_folder_browser_message(
        FolderBrowserMessage::ActivateFolder(loops.display().to_string(), Default::default()),
        &mut context,
    );

    assert_ne!(
        state.background.folder_verify_task.active(),
        Some(first_ticket),
        "new folder activation should supersede an older pending verification"
    );
}

#[test]
fn context_new_folder_creates_child_and_starts_inline_rename() {
    let mut state = gui_state_for_span_tests();
    let source_root = tempfile::tempdir().expect("source root");
    let parent = source_root.path().join("drums");
    fs::create_dir_all(&parent).expect("create drums folder");
    state.library.folder_browser =
        FolderBrowserState::from_sample_sources(&[wavecrate::sample_sources::SampleSource::new(
            source_root.path().to_path_buf(),
        )]);
    let parent_id = parent.display().to_string();
    state.open_folder_context_menu(parent_id.clone(), Point::new(40.0, 120.0));

    let mut context = radiant::prelude::UiUpdateContext::default();
    state.apply_message(GuiMessage::CreateFolderAtContextTarget, &mut context);
    run_command_for_tests(&mut state, context.into_command());

    let created = parent.join("New Folder");
    let created_id = created.display().to_string();
    assert!(created.is_dir());
    assert_eq!(
        state.library.folder_browser.selected_folder_id(),
        Some(created_id.as_str())
    );
    assert_eq!(
        state
            .library
            .folder_browser
            .folder_expansion_for_tests(&parent_id),
        Some(true)
    );
    assert!(
        state
            .library
            .folder_browser
            .visible_folders()
            .into_iter()
            .any(|folder| {
                folder.id == created_id
                    && folder.selected
                    && folder.rename_draft.as_deref() == Some("New Folder")
                    && folder.rename_input_id.is_some()
            })
    );
    assert!(state.ui.status.sample.contains("Created folder New Folder"));
}

#[test]
fn context_new_folder_creates_root_child_from_source_context() {
    let mut state = gui_state_for_span_tests();
    let source_root = tempfile::tempdir().expect("source root");
    let source = wavecrate::sample_sources::SampleSource::new(source_root.path().to_path_buf());
    let source_id = source.id.as_str().to_string();
    state.library.folder_browser = FolderBrowserState::from_sample_sources(&[source]);
    state.open_source_context_menu(source_id, Point::new(40.0, 120.0));

    let mut context = radiant::prelude::UiUpdateContext::default();
    state.apply_message(GuiMessage::CreateFolderAtContextTarget, &mut context);
    run_command_for_tests(&mut state, context.into_command());

    let created = source_root.path().join("New Folder");
    let created_id = created.display().to_string();
    assert!(created.is_dir());
    assert_eq!(
        state.library.folder_browser.selected_folder_id(),
        Some(created_id.as_str())
    );
    assert!(
        state
            .library
            .folder_browser
            .visible_folders()
            .into_iter()
            .any(|folder| folder.id == created_id
                && folder.rename_draft.as_deref() == Some("New Folder"))
    );
}

#[test]
fn context_new_folder_uses_collision_safe_name() {
    let mut state = gui_state_for_span_tests();
    let source_root = tempfile::tempdir().expect("source root");
    let parent = source_root.path().join("drums");
    fs::create_dir_all(parent.join("New Folder")).expect("create first collision");
    fs::create_dir_all(parent.join("New Folder 2")).expect("create second collision");
    state.library.folder_browser =
        FolderBrowserState::from_sample_sources(&[wavecrate::sample_sources::SampleSource::new(
            source_root.path().to_path_buf(),
        )]);
    state.open_folder_context_menu(parent.display().to_string(), Point::new(40.0, 120.0));

    let mut context = radiant::prelude::UiUpdateContext::default();
    state.apply_message(GuiMessage::CreateFolderAtContextTarget, &mut context);
    run_command_for_tests(&mut state, context.into_command());

    let created = parent.join("New Folder 3");
    let created_id = created.display().to_string();
    assert!(created.is_dir());
    assert_eq!(
        state.library.folder_browser.selected_folder_id(),
        Some(created_id.as_str())
    );
    assert!(
        state
            .ui
            .status
            .sample
            .contains("Created folder New Folder 3")
    );
}

#[test]
fn context_new_folder_missing_parent_reports_error_without_tree_corruption() {
    let mut state = gui_state_for_span_tests();
    let source_root = tempfile::tempdir().expect("source root");
    let parent = source_root.path().join("drums");
    fs::create_dir_all(&parent).expect("create drums folder");
    state.library.folder_browser =
        FolderBrowserState::from_sample_sources(&[wavecrate::sample_sources::SampleSource::new(
            source_root.path().to_path_buf(),
        )]);
    let parent_id = parent.display().to_string();
    state.open_folder_context_menu(parent_id.clone(), Point::new(40.0, 120.0));
    fs::remove_dir_all(&parent).expect("remove context target");

    let mut context = radiant::prelude::UiUpdateContext::default();
    state.apply_message(GuiMessage::CreateFolderAtContextTarget, &mut context);
    run_command_for_tests(&mut state, context.into_command());

    assert!(state.ui.status.sample.contains("parent folder"));
    assert!(state.ui.status.sample.contains("unavailable"));
    assert!(
        !state
            .library
            .folder_browser
            .visible_folders()
            .into_iter()
            .any(|folder| folder.name == "New Folder" && folder.id.starts_with(&parent_id))
    );
}

#[test]
fn context_delete_folder_requests_confirmation() {
    let mut state = gui_state_for_span_tests();
    let source_root = tempfile::tempdir().expect("source root");
    let folder = source_root.path().join("drums");
    fs::create_dir_all(&folder).expect("create drums folder");
    state.library.folder_browser =
        FolderBrowserState::from_sample_sources(&[wavecrate::sample_sources::SampleSource::new(
            source_root.path().to_path_buf(),
        )]);
    state.open_folder_context_menu(folder.display().to_string(), Point::new(40.0, 120.0));

    let mut context = radiant::prelude::UiUpdateContext::default();
    state.apply_message(GuiMessage::RequestDeleteContextFolder, &mut context);

    let pending = state
        .ui
        .browser_interaction
        .pending_folder_delete
        .as_ref()
        .expect("folder delete should wait for confirmation");
    assert_eq!(pending.path, folder);
    assert!(state.ui.browser_interaction.context_menu.is_none());

    let frame = view(&mut state).view_frame_at_size_with_default_theme(Vector2::new(900.0, 620.0));
    assert!(frame.paint_plan.contains_text("Delete Folder"));
    assert!(
        frame
            .paint_plan
            .contains_text("Move folder contents to the configured trash folder?")
    );
    assert!(folder.is_dir());
}

#[test]
fn context_delete_folder_confirmation_moves_folder_to_trash() {
    let mut state = gui_state_for_span_tests();
    let source_root = tempfile::tempdir().expect("source root");
    let trash_root = tempfile::tempdir().expect("trash root");
    let folder = source_root.path().join("drums");
    let sibling = source_root.path().join("loops");
    fs::create_dir_all(folder.join("nested")).expect("create nested folder");
    fs::create_dir_all(&sibling).expect("create sibling folder");
    fs::write(folder.join("nested").join("kick.wav"), []).expect("write nested sample");
    fs::write(sibling.join("loop.wav"), []).expect("write sibling sample");
    state.ui.settings.persisted.trash_folder = Some(trash_root.path().to_path_buf());
    state.library.folder_browser =
        FolderBrowserState::from_sample_sources(&[wavecrate::sample_sources::SampleSource::new(
            source_root.path().to_path_buf(),
        )]);
    let folder_id = folder.display().to_string();
    state
        .library
        .folder_browser
        .apply_message(FolderBrowserMessage::ActivateFolder(
            folder_id.clone(),
            Default::default(),
        ));
    state.open_folder_context_menu(folder_id.clone(), Point::new(40.0, 120.0));

    let mut context = radiant::prelude::UiUpdateContext::default();
    state.apply_message(GuiMessage::RequestDeleteContextFolder, &mut context);
    state.apply_message(GuiMessage::ConfirmContextFolderDelete, &mut context);
    run_command_for_tests(&mut state, context.into_command());

    assert!(!folder.exists());
    assert!(
        trash_root
            .path()
            .join("drums")
            .join("nested")
            .join("kick.wav")
            .exists()
    );
    assert!(sibling.exists());
    assert!(
        state
            .library
            .folder_browser
            .visible_folders()
            .into_iter()
            .all(|visible| visible.id != folder_id)
    );
    let source_root_id = source_root.path().display().to_string();
    assert_eq!(
        state.library.folder_browser.selected_folder_id(),
        Some(source_root_id.as_str())
    );
    assert!(state.ui.status.sample.contains("Moved drums to trash"));
}

#[test]
fn context_delete_folder_missing_target_reconciles_tree_without_trash_move() {
    let mut state = gui_state_for_span_tests();
    let source_root = tempfile::tempdir().expect("source root");
    let folder = source_root.path().join("drums");
    fs::create_dir_all(&folder).expect("create drums folder");
    state.library.folder_browser =
        FolderBrowserState::from_sample_sources(&[wavecrate::sample_sources::SampleSource::new(
            source_root.path().to_path_buf(),
        )]);
    let folder_id = folder.display().to_string();
    state.open_folder_context_menu(folder_id.clone(), Point::new(40.0, 120.0));

    let mut context = radiant::prelude::UiUpdateContext::default();
    state.apply_message(GuiMessage::RequestDeleteContextFolder, &mut context);
    fs::remove_dir_all(&folder).expect("remove folder before confirmation");
    state.apply_message(GuiMessage::ConfirmContextFolderDelete, &mut context);

    assert!(
        state
            .library
            .folder_browser
            .visible_folders()
            .into_iter()
            .all(|visible| visible.id != folder_id)
    );
    assert!(state.ui.status.sample.contains("no longer exists"));
    assert!(
        state
            .ui
            .status
            .sample
            .contains("removed it from the browser")
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

    let mut context = radiant::prelude::UiUpdateContext::default();
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
fn third_negative_rating_does_not_auto_trash_selected_file() {
    let mut state = gui_state_for_span_tests();
    let source_root = tempfile::tempdir().expect("source root");
    let trash_root = tempfile::tempdir().expect("trash root");
    let sample = source_root.path().join("third.wav");
    fs::write(&sample, []).expect("write sample");
    state.ui.settings.persisted.trash_folder = Some(trash_root.path().to_path_buf());
    state.library.folder_browser =
        FolderBrowserState::from_sample_sources(&[wavecrate::sample_sources::SampleSource::new(
            source_root.path().to_path_buf(),
        )]);
    state
        .library
        .folder_browser
        .select_file(sample.display().to_string());
    assert!(
        state
            .library
            .folder_browser
            .set_file_rating_state(&sample, Rating::new(-2), false)
    );

    let mut context = radiant::prelude::UiUpdateContext::default();
    state.adjust_selected_rating(-1, &mut context);

    assert!(sample.exists());
    assert!(!trash_root.path().join("third.wav").exists());
    let selected = state.library.folder_browser.selected_audio_files();
    assert_eq!(selected.len(), 1);
    assert_eq!(selected[0].rating, Rating::TRASH_3);
    assert!(state.ui.status.sample.contains("Rated 1 sample"));
}

#[test]
fn rating_adjustment_survives_selected_file_rename() {
    let (mut state, source_root, selected_file) = native_app_state_with_temp_sample("kick.wav");
    state.ui.settings.persisted.controls.advance_after_rating = false;
    let mut context = radiant::prelude::UiUpdateContext::default();

    state.adjust_selected_rating(1, &mut context);
    state
        .library
        .folder_browser
        .begin_rename_selected()
        .expect("rename can start")
        .expect("rename input id");
    super::submit_folder_browser_rename_for_tests(&mut state, "snare");

    let renamed = source_root.path().join("snare.wav");
    let rows = state.library.folder_browser.selected_audio_files();
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0].id, renamed.display().to_string());
    assert_eq!(rows[0].rating, Rating::KEEP_1);
    assert!(!std::path::Path::new(&selected_file).exists());
    assert!(renamed.exists());
    let db = wavecrate::sample_sources::SourceDatabase::open(source_root.path()).expect("db");
    assert_eq!(
        db.tag_for_path(std::path::Path::new("snare.wav"))
            .expect("rating"),
        Some(Rating::KEEP_1)
    );
}

#[test]
fn rating_adjustment_survives_selected_file_move() {
    let mut state = gui_state_for_span_tests();
    let source_root = tempfile::tempdir().expect("source root");
    let drums = source_root.path().join("drums");
    let loops = source_root.path().join("loops");
    fs::create_dir_all(&drums).expect("create drums folder");
    fs::create_dir_all(&loops).expect("create loops folder");
    let kick = drums.join("kick.wav");
    fs::write(&kick, []).expect("write kick");
    state.ui.settings.persisted.controls.advance_after_rating = false;
    state.library.folder_browser =
        FolderBrowserState::from_sample_sources(&[wavecrate::sample_sources::SampleSource::new(
            source_root.path().to_path_buf(),
        )]);
    state
        .library
        .folder_browser
        .apply_message(FolderBrowserMessage::ActivateFolder(
            drums.display().to_string(),
            Default::default(),
        ));
    state
        .library
        .folder_browser
        .select_file(kick.display().to_string());

    let mut context = radiant::prelude::UiUpdateContext::default();
    state.adjust_selected_rating(1, &mut context);
    state
        .library
        .folder_browser
        .begin_file_drag(kick.display().to_string(), Point::new(4.0, 8.0));
    state.drop_browser_drag_on_folder(loops.display().to_string(), &mut context);
    run_command_for_tests(&mut state, context.into_command());

    let moved = loops.join("kick.wav");
    state
        .library
        .folder_browser
        .apply_message(FolderBrowserMessage::ActivateFolder(
            loops.display().to_string(),
            Default::default(),
        ));
    let rows = state.library.folder_browser.selected_audio_files();
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0].id, moved.display().to_string());
    assert_eq!(rows[0].rating, Rating::KEEP_1);
    assert!(!kick.exists());
    assert!(moved.exists());
    let db = wavecrate::sample_sources::SourceDatabase::open(source_root.path()).expect("db");
    assert_eq!(
        db.tag_for_path(std::path::Path::new("loops/kick.wav"))
            .expect("rating"),
        Some(Rating::KEEP_1)
    );
}

#[test]
fn fourth_negative_rating_moves_selected_file_to_trash() {
    let mut state = gui_state_for_span_tests();
    let source_root = tempfile::tempdir().expect("source root");
    let trash_root = tempfile::tempdir().expect("trash root");
    let keep = source_root.path().join("keep.wav");
    let sample = source_root.path().join("fourth.wav");
    fs::write(&keep, []).expect("write keep wav");
    fs::write(&sample, []).expect("write sample");
    state.ui.settings.persisted.trash_folder = Some(trash_root.path().to_path_buf());
    state.library.folder_browser =
        FolderBrowserState::from_sample_sources(&[wavecrate::sample_sources::SampleSource::new(
            source_root.path().to_path_buf(),
        )]);
    state
        .library
        .folder_browser
        .select_file(sample.display().to_string());
    assert!(
        state
            .library
            .folder_browser
            .set_file_rating_state(&sample, Rating::TRASH_3, false)
    );

    let mut context = radiant::prelude::UiUpdateContext::default();
    state.adjust_selected_rating(-1, &mut context);
    assert!(
        state.ui.status.sample.contains("fourth negative rating"),
        "{}",
        state.ui.status.sample
    );
    run_command_for_tests(&mut state, context.into_command());

    assert!(!sample.exists());
    assert!(trash_root.path().join("fourth.wav").exists());
    assert!(keep.exists());
    assert_eq!(state.library.folder_browser.selected_file_id(), None);
    assert!(
        !state
            .library
            .folder_browser
            .selected_audio_files()
            .iter()
            .any(|file| file.name == "fourth.wav")
    );
    assert!(
        state
            .ui
            .status
            .sample
            .contains("Moved 1 file to trash after fourth negative rating")
    );
}

#[test]
fn fourth_negative_rating_keeps_file_available_when_trash_move_fails() {
    let mut state = gui_state_for_span_tests();
    let source_root = tempfile::tempdir().expect("source root");
    let sample = source_root.path().join("blocked.wav");
    fs::write(&sample, []).expect("write sample");
    state.library.folder_browser =
        FolderBrowserState::from_sample_sources(&[wavecrate::sample_sources::SampleSource::new(
            source_root.path().to_path_buf(),
        )]);
    state
        .library
        .folder_browser
        .select_file(sample.display().to_string());
    assert!(
        state
            .library
            .folder_browser
            .set_file_rating_state(&sample, Rating::TRASH_3, false)
    );

    let mut context = radiant::prelude::UiUpdateContext::default();
    state.adjust_selected_rating(-1, &mut context);
    run_command_for_tests(&mut state, context.into_command());

    assert!(sample.exists());
    let expected_selected = sample.display().to_string();
    assert_eq!(
        state.library.folder_browser.selected_file_id(),
        Some(expected_selected.as_str())
    );
    assert_eq!(
        state.library.folder_browser.selected_audio_files()[0].rating,
        Rating::TRASH_3
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
            Default::default(),
        ));

    let mut context = radiant::prelude::UiUpdateContext::default();
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
            Default::default(),
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

    let mut context = radiant::prelude::UiUpdateContext::default();
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
