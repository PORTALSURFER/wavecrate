use crate::native_app::test_support::{
    sample_browser::{DEFAULT_FOLDER_WIDTH, MAX_FOLDER_WIDTH, MIN_FOLDER_WIDTH},
    state::{FolderBrowserState, GuiMessage, NativeAppStateFixture},
};
use radiant::runtime::Command;
use radiant::{gui::types::Point, widgets::DragHandleMessage};
use std::fs;

#[test]
fn folder_browser_splitter_resizes_and_clamps_width() {
    let mut state = NativeAppStateFixture::default()
        .with_synthetic_waveform()
        .with_sample_status("")
        .build();
    state.resize_folder_browser(DragHandleMessage::started(Point::new(100.0, 0.0)));
    state.resize_folder_browser(DragHandleMessage::moved(Point::new(160.0, 0.0)));

    assert_eq!(
        state.ui.chrome.folder_panel.size(),
        DEFAULT_FOLDER_WIDTH + 60.0
    );

    state.resize_folder_browser(DragHandleMessage::moved(Point::new(900.0, 0.0)));
    assert_eq!(state.ui.chrome.folder_panel.size(), MAX_FOLDER_WIDTH);

    state.resize_folder_browser(DragHandleMessage::ended(Point::new(-900.0, 0.0)));
    assert_eq!(state.ui.chrome.folder_panel.size(), MIN_FOLDER_WIDTH);
    assert!(!state.ui.chrome.folder_panel.is_resizing());
}

#[test]
fn keyboard_folder_navigation_keeps_selected_folder_in_tree_view() {
    let tempdir = tempfile::tempdir().expect("create temp root");
    let root = tempdir.path().join("wavecrate-folder-keyboard-follow");
    fs::create_dir_all(&root).expect("create source root");
    for index in 0..20 {
        let folder = root.join(format!("folder_{index:02}"));
        fs::create_dir_all(&folder).expect("create folder");
        fs::write(folder.join("sample.wav"), []).expect("write sample");
    }
    let mut state = NativeAppStateFixture::default()
        .with_folder_browser(FolderBrowserState::from_root(root.clone()))
        .build();
    let mut context = radiant::prelude::UiUpdateContext::default();
    assert!(
        state.library.folder_browser.visible_folders().len() > 12,
        "fixture should produce a long visible folder tree"
    );

    for _ in 0..12 {
        state.navigate_browser(1, false, false, &mut context);
    }

    assert_eq!(
        state.library.folder_browser.selected_folder_id(),
        Some(root.join("folder_11").to_string_lossy().as_ref())
    );
    assert_eq!(
        last_fixed_row_scroll(context.into_command()),
        Some((12, 23.0, 2, 2, 1))
    );
}

#[test]
fn x_toggle_marks_focused_folder_without_sample_focus() {
    let tempdir = tempfile::tempdir().expect("create temp root");
    let root = tempdir.path().join("wavecrate-folder-x-toggle");
    let drums = root.join("drums");
    let loops = root.join("loops");
    fs::create_dir_all(&drums).expect("create drums folder");
    fs::create_dir_all(&loops).expect("create loops folder");
    let mut state = NativeAppStateFixture::default()
        .with_folder_browser(FolderBrowserState::from_root(root.clone()))
        .build();
    let mut context = radiant::prelude::UiUpdateContext::default();

    state.apply_message(GuiMessage::ToggleSelectedSampleAndAdvance, &mut context);

    let visible = state.library.folder_browser.visible_folders();
    let root_id = root.display().to_string();
    let drums_id = drums.display().to_string();
    let loops_id = loops.display().to_string();
    let root_row = visible
        .iter()
        .find(|folder| folder.id == root_id)
        .expect("root row should stay visible");
    let drums_row = visible
        .iter()
        .find(|folder| folder.id == drums_id)
        .expect("drums row should stay visible");
    assert!(root_row.selected);
    assert!(!root_row.focused);
    assert!(!drums_row.selected);
    assert!(drums_row.focused);
    assert!(state.ui.status.sample.contains("Marked"));

    state.apply_message(GuiMessage::ToggleSelectedSampleAndAdvance, &mut context);

    let visible = state.library.folder_browser.visible_folders();
    let drums_row = visible
        .iter()
        .find(|folder| folder.id == drums_id)
        .expect("drums row should stay visible");
    let loops_row = visible
        .iter()
        .find(|folder| folder.id == loops_id)
        .expect("loops row should stay visible");
    assert!(drums_row.selected);
    assert!(!drums_row.focused);
    assert!(!loops_row.selected);
    assert!(loops_row.focused);
    assert!(state.ui.status.sample.contains("2 selected"));
}

fn last_fixed_row_scroll(command: Command<GuiMessage>) -> Option<(usize, f32, usize, usize, i32)> {
    match command {
        Command::Batch(commands) => commands
            .into_iter()
            .filter_map(last_fixed_row_scroll)
            .last(),
        Command::ScrollFixedRowIntoView {
            row_index,
            row_stride,
            leading_context_rows,
            trailing_context_rows,
            direction,
            ..
        } => Some((
            row_index,
            row_stride,
            leading_context_rows,
            trailing_context_rows,
            direction,
        )),
        _ => None,
    }
}
