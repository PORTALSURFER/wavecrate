use crate::native_app::{
    sample_library::context_menu_target::target_available,
    test_support::{
        context_menu::{BrowserContextMenu, BrowserContextTargetKind, WaveformContextMenu},
        state::{FolderBrowserMessage, GuiMessage, NativeAppState, default_gui_shortcuts},
    },
};
use radiant::{gui::types::Point, prelude as ui};
use std::path::PathBuf;

fn sample_context_menu(path: impl Into<PathBuf>) -> BrowserContextMenu {
    BrowserContextMenu {
        kind: BrowserContextTargetKind::Sample,
        path: path.into(),
        source_id: None,
        source_role: wavecrate::sample_sources::SourceRole::Normal,
        source_removable: false,
        folder_locked: false,
        folder_lock_inherited: false,
        metadata_tag: None,
        collection: None,
        sample_missing: false,
        sample_keep_locked: false,
        anchor: Point::new(12.0, 24.0),
        title: String::from("kick.wav"),
    }
}

#[test]
fn context_menu_escape_shortcut_closes_context_menu() {
    let mut state = NativeAppState::load_default().expect("default state loads");
    state.ui.browser_interaction.context_menu = Some(sample_context_menu("C:\\samples\\kick.wav"));

    let resolution = default_gui_shortcuts(&state).resolve(ui::KeyPress::new(ui::KeyCode::Escape));

    assert_eq!(resolution.action, Some(GuiMessage::CloseContextMenu));
    assert!(resolution.handled);
}

#[test]
fn context_menu_w_shortcut_closes_context_menu() {
    let mut state = NativeAppState::load_default().expect("default state loads");
    state.ui.browser_interaction.context_menu = Some(sample_context_menu("C:\\samples\\kick.wav"));

    let resolution = default_gui_shortcuts(&state).resolve(ui::KeyPress::new(ui::KeyCode::W));

    assert_eq!(resolution.action, Some(GuiMessage::CloseContextMenu));
    assert!(resolution.handled);
}

#[test]
fn waveform_context_menu_escape_shortcut_closes_context_menu() {
    let mut state = NativeAppState::load_default().expect("default state loads");
    state.ui.browser_interaction.waveform_context_menu = Some(WaveformContextMenu {
        anchor: Point::new(12.0, 24.0),
        title: String::from("Playmark Selection"),
        extract_to_harvest_destination: false,
    });

    let resolution = default_gui_shortcuts(&state).resolve(ui::KeyPress::new(ui::KeyCode::Escape));

    assert_eq!(resolution.action, Some(GuiMessage::CloseContextMenu));
    assert!(resolution.handled);
}

#[test]
fn waveform_context_menu_w_shortcut_closes_context_menu() {
    let mut state = NativeAppState::load_default().expect("default state loads");
    state.ui.browser_interaction.waveform_context_menu = Some(WaveformContextMenu {
        anchor: Point::new(12.0, 24.0),
        title: String::from("Playmark Selection"),
        extract_to_harvest_destination: false,
    });

    let resolution = default_gui_shortcuts(&state).resolve(ui::KeyPress::new(ui::KeyCode::W));

    assert_eq!(resolution.action, Some(GuiMessage::CloseContextMenu));
    assert!(resolution.handled);
}

#[test]
fn close_context_menu_message_clears_waveform_context_menu() {
    let mut state = NativeAppState::load_default().expect("default state loads");
    state.ui.browser_interaction.waveform_context_menu = Some(WaveformContextMenu {
        anchor: Point::new(12.0, 24.0),
        title: String::from("Playmark Selection"),
        extract_to_harvest_destination: false,
    });

    state.apply_message(
        GuiMessage::CloseContextMenu,
        &mut ui::UiUpdateContext::default(),
    );

    assert_eq!(state.ui.browser_interaction.waveform_context_menu, None);
}

#[test]
fn context_menu_escape_takes_priority_over_collection_focus_escape() {
    let mut state = NativeAppState::load_default().expect("default state loads");
    let collection = wavecrate::sample_sources::SampleCollection::new(0).expect("collection");
    state
        .library
        .folder_browser
        .apply_message(FolderBrowserMessage::ActivateCollection(collection));
    state.ui.browser_interaction.context_menu = Some(sample_context_menu("C:\\samples\\kick.wav"));

    let resolution = default_gui_shortcuts(&state).resolve(ui::KeyPress::new(ui::KeyCode::Escape));

    assert_eq!(resolution.action, Some(GuiMessage::CloseContextMenu));
    assert!(resolution.handled);
}

#[test]
fn context_menu_availability_does_not_probe_disk() {
    let root = std::env::temp_dir().join(format!(
        "wavecrate-context-menu-{}",
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("clock")
            .as_nanos()
    ));
    std::fs::create_dir_all(&root).expect("create temp root");
    let sample = root.join("kick.wav");
    std::fs::write(&sample, [0_u8; 8]).expect("write sample");

    assert!(target_available(&BrowserContextTargetKind::Source, &root));
    assert!(target_available(&BrowserContextTargetKind::Folder, &root));
    assert!(target_available(&BrowserContextTargetKind::Sample, &sample));
    assert!(target_available(&BrowserContextTargetKind::Sample, &root));
    assert!(target_available(&BrowserContextTargetKind::Folder, &sample));

    std::fs::remove_file(&sample).expect("remove sample");
    assert!(target_available(&BrowserContextTargetKind::Sample, &sample));
    let _ = std::fs::remove_dir_all(root);
}

#[test]
fn stale_context_menu_copy_path_defers_missing_file_to_platform_completion() {
    let mut state = NativeAppState::load_default().expect("default state loads");
    let missing = std::env::temp_dir().join("wavecrate-missing-context-sample.wav");
    state.ui.browser_interaction.context_menu = Some(sample_context_menu(missing.clone()));

    let mut context = ui::UiUpdateContext::default();
    state.copy_context_path(&mut context);

    state.finish_context_path_copy(
        BrowserContextTargetKind::Sample,
        missing,
        Err(String::from("Sample file is missing")),
    );
    assert_eq!(
        state.ui.status.sample,
        "Copy path failed: Sample file is missing"
    );
    assert_eq!(state.ui.browser_interaction.context_menu, None);
}

#[test]
fn context_path_copy_completion_updates_status() {
    let mut state = NativeAppState::load_default().expect("default state loads");

    state.finish_context_path_copy(
        BrowserContextTargetKind::Sample,
        PathBuf::from("C:\\samples\\kick.wav"),
        Ok(ui::PlatformResponse::Completed),
    );
    assert_eq!(state.ui.status.sample, "Copied path");

    state.finish_context_path_copy(
        BrowserContextTargetKind::Sample,
        PathBuf::from("C:\\samples\\kick.wav"),
        Err(String::from("clipboard unavailable")),
    );
    assert_eq!(
        state.ui.status.sample,
        "Copy path failed: clipboard unavailable"
    );
}

#[test]
fn context_target_open_completion_updates_status() {
    let mut state = NativeAppState::load_default().expect("default state loads");

    state.finish_context_target_open(
        BrowserContextTargetKind::Sample,
        PathBuf::from("C:\\samples\\kick.wav"),
        Ok(ui::PlatformResponse::Completed),
    );
    assert_eq!(state.ui.status.sample, "Revealed sample");

    state.finish_context_target_open(
        BrowserContextTargetKind::Folder,
        PathBuf::from("C:\\samples"),
        Err(String::from("shell unavailable")),
    );
    assert_eq!(state.ui.status.sample, "shell unavailable");
}
