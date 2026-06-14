use super::{gui_state_for_span_tests, temp_gui_root, write_test_wav_i16};
use radiant::widgets::DragHandleMessage;
use radiant::{gui::types::Point, prelude as ui, runtime::NativeFileDrop};
use std::fs;

#[test]
fn native_file_hover_over_waveform_tracks_supported_state() {
    let root = temp_gui_root("wavecrate-native-file-hover");
    let wav = root.join("kick.wav");
    let txt = root.join("note.txt");
    write_test_wav_i16(&wav, &[0, 100]);
    fs::write(&txt, "not audio").expect("write text");
    let mut state = gui_state_for_span_tests();
    state.library.folder_browser =
        crate::native_app::test_support::state::FolderBrowserState::from_sample_sources(&[
            wavecrate::sample_sources::SampleSource::new(root.clone()),
        ]);
    let mut context = ui::UiUpdateContext::default();

    state.apply_native_file_drop(
        NativeFileDrop::hover(
            wav.clone(),
            Some(Point::new(8.0, 8.0)),
            Some(crate::native_app::test_support::waveform::WAVEFORM_WIDGET_ID),
        ),
        &mut context,
    );
    assert_eq!(
        state.ui.browser_interaction.native_file_drop_hover,
        Some(
            crate::native_app::test_support::state::NativeFileDropHover {
                path: wav.clone(),
                supported: true,
            }
        )
    );

    state.apply_native_file_drop(
        NativeFileDrop::hover(
            txt.clone(),
            Some(Point::new(8.0, 8.0)),
            Some(crate::native_app::test_support::waveform::WAVEFORM_WIDGET_ID),
        ),
        &mut context,
    );
    assert_eq!(
        state.ui.browser_interaction.native_file_drop_hover,
        Some(
            crate::native_app::test_support::state::NativeFileDropHover {
                path: txt,
                supported: false,
            }
        )
    );

    state.apply_native_file_drop(
        NativeFileDrop::cancel(
            Some(Point::new(8.0, 8.0)),
            Some(crate::native_app::test_support::waveform::WAVEFORM_WIDGET_ID),
        ),
        &mut context,
    );
    assert_eq!(state.ui.browser_interaction.native_file_drop_hover, None);
    let _ = fs::remove_dir_all(root);
}

#[test]
fn native_file_hover_without_widget_target_still_shows_waveform_drop_feedback() {
    let root = temp_gui_root("wavecrate-native-file-hover-targetless");
    let wav = root.join("kick.wav");
    write_test_wav_i16(&wav, &[0, 100]);
    let mut state = gui_state_for_span_tests();
    let mut context = ui::UiUpdateContext::default();

    state.apply_native_file_drop(
        NativeFileDrop::hover(wav.clone(), Some(Point::new(8.0, 8.0)), None),
        &mut context,
    );

    assert_eq!(
        state.ui.browser_interaction.native_file_drop_hover,
        Some(
            crate::native_app::test_support::state::NativeFileDropHover {
                path: wav,
                supported: true,
            }
        )
    );
    let _ = fs::remove_dir_all(root);
}

#[test]
fn native_file_drop_on_waveform_copies_into_selected_folder_and_queues_load() {
    let root = temp_gui_root("wavecrate-native-file-drop-root");
    let external_root = temp_gui_root("wavecrate-native-file-drop-external");
    let loops = root.join("loops");
    fs::create_dir_all(&loops).expect("create loops");
    let source = external_root.join("kick.wav");
    write_test_wav_i16(&source, &[0, 100, -100]);
    let mut state = gui_state_for_span_tests();
    state.library.folder_browser =
        crate::native_app::test_support::state::FolderBrowserState::from_sample_sources(&[
            wavecrate::sample_sources::SampleSource::new(root.clone()),
        ]);
    state.library.folder_browser.apply_message(
        crate::native_app::test_support::state::FolderBrowserMessage::ActivateFolder(
            loops.display().to_string(),
            Default::default(),
        ),
    );
    let mut context = ui::UiUpdateContext::default();

    state.apply_native_file_drop(
        NativeFileDrop::dropped(
            source,
            Some(Point::new(8.0, 8.0)),
            Some(crate::native_app::test_support::waveform::WAVEFORM_WIDGET_ID),
        ),
        &mut context,
    );

    let copied = loops.join("kick.wav");
    let copied_id = copied.display().to_string();
    assert!(copied.is_file());
    assert_eq!(
        state.library.folder_browser.selected_file_id(),
        Some(copied_id.as_str())
    );
    assert_eq!(state.waveform.load.label.as_deref(), Some("kick.wav"));
    assert!(
        state
            .background
            .deferred_sample_load_task
            .active()
            .is_some(),
        "native file import should debounce uncached sample loading before queueing decode work"
    );
    super::start_deferred_sample_load_for_tests(&mut state, copied_id, true, &mut context);
    assert!(state.background.sample_load_task.active().is_some());
    let _ = fs::remove_dir_all(root);
    let _ = fs::remove_dir_all(external_root);
}

#[test]
fn native_file_drop_without_widget_target_imports_into_selected_folder() {
    let root = temp_gui_root("wavecrate-native-file-drop-targetless-root");
    let external_root = temp_gui_root("wavecrate-native-file-drop-targetless-external");
    let loops = root.join("loops");
    fs::create_dir_all(&loops).expect("create loops");
    let source = external_root.join("kick.wav");
    write_test_wav_i16(&source, &[0, 100, -100]);
    let mut state = gui_state_for_span_tests();
    state.library.folder_browser =
        crate::native_app::test_support::state::FolderBrowserState::from_sample_sources(&[
            wavecrate::sample_sources::SampleSource::new(root.clone()),
        ]);
    state.library.folder_browser.apply_message(
        crate::native_app::test_support::state::FolderBrowserMessage::ActivateFolder(
            loops.display().to_string(),
            Default::default(),
        ),
    );
    let mut context = ui::UiUpdateContext::default();

    state.apply_native_file_drop(
        NativeFileDrop::dropped(source, Some(Point::new(8.0, 8.0)), None),
        &mut context,
    );

    let copied = loops.join("kick.wav");
    let copied_id = copied.display().to_string();
    assert!(copied.is_file());
    assert_eq!(
        state.library.folder_browser.selected_file_id(),
        Some(copied_id.as_str())
    );
    let _ = fs::remove_dir_all(root);
    let _ = fs::remove_dir_all(external_root);
}

#[test]
fn native_file_drop_from_selected_folder_cancels_instead_of_copying() {
    let root = temp_gui_root("wavecrate-native-file-drop-same-folder-root");
    let drums = root.join("drums");
    fs::create_dir_all(&drums).expect("create drums");
    let source = drums.join("kick.wav");
    write_test_wav_i16(&source, &[0, 100, -100]);
    let mut state = gui_state_for_span_tests();
    state.library.folder_browser =
        crate::native_app::test_support::state::FolderBrowserState::from_sample_sources(&[
            wavecrate::sample_sources::SampleSource::new(root.clone()),
        ]);
    state.library.folder_browser.apply_message(
        crate::native_app::test_support::state::FolderBrowserMessage::ActivateFolder(
            drums.display().to_string(),
            Default::default(),
        ),
    );
    let mut context = ui::UiUpdateContext::default();

    state.apply_native_file_drop(
        NativeFileDrop::dropped(source.clone(), Some(Point::new(8.0, 8.0)), None),
        &mut context,
    );

    assert!(source.is_file());
    assert!(!drums.join("kick_copy001.wav").exists());
    assert_eq!(state.ui.status.sample, "Drag cancelled");
    assert!(
        state
            .background
            .deferred_sample_load_task
            .active()
            .is_none()
    );
    let _ = fs::remove_dir_all(root);
}

#[test]
fn native_file_drop_from_active_browser_drag_cancels_instead_of_copying() {
    let root = temp_gui_root("wavecrate-native-file-drop-internal-root");
    let drums = root.join("drums");
    let loops = root.join("loops");
    fs::create_dir_all(&drums).expect("create drums");
    fs::create_dir_all(&loops).expect("create loops");
    let source = drums.join("kick.wav");
    write_test_wav_i16(&source, &[0, 100, -100]);
    let mut state = gui_state_for_span_tests();
    state.library.folder_browser =
        crate::native_app::test_support::state::FolderBrowserState::from_sample_sources(&[
            wavecrate::sample_sources::SampleSource::new(root.clone()),
        ]);
    state.library.folder_browser.apply_message(
        crate::native_app::test_support::state::FolderBrowserMessage::ActivateFolder(
            drums.display().to_string(),
            Default::default(),
        ),
    );
    state
        .library
        .folder_browser
        .begin_file_drag(source.display().to_string(), Point::new(4.0, 8.0));
    let mut context = ui::UiUpdateContext::default();

    state.apply_native_file_drop(
        NativeFileDrop::dropped(source.clone(), Some(Point::new(8.0, 8.0)), None),
        &mut context,
    );

    assert!(source.is_file());
    assert!(!drums.join("kick_copy001.wav").exists());
    assert!(!loops.join("kick.wav").exists());
    assert!(!state.library.folder_browser.drag_active());
    assert_eq!(state.ui.status.sample, "Drag cancelled");
    let _ = fs::remove_dir_all(root);
}

#[test]
fn native_file_drop_after_internal_browser_drag_release_cancels_instead_of_copying() {
    let root = temp_gui_root("wavecrate-native-file-drop-internal-release-root");
    let drums = root.join("drums");
    let loops = root.join("loops");
    fs::create_dir_all(&drums).expect("create drums");
    fs::create_dir_all(&loops).expect("create loops");
    let source = drums.join("kick.wav");
    write_test_wav_i16(&source, &[0, 100, -100]);
    let source_id = source.display().to_string();
    let mut state = gui_state_for_span_tests();
    state.library.folder_browser =
        crate::native_app::test_support::state::FolderBrowserState::from_sample_sources(&[
            wavecrate::sample_sources::SampleSource::new(root.clone()),
        ]);
    state.library.folder_browser.apply_message(
        crate::native_app::test_support::state::FolderBrowserMessage::ActivateFolder(
            drums.display().to_string(),
            Default::default(),
        ),
    );
    state.library.folder_browser.select_file(source_id.clone());
    let mut context = ui::UiUpdateContext::default();

    state.apply_message(
        crate::native_app::test_support::state::GuiMessage::DragSampleFile {
            path: source_id.clone(),
            drag: DragHandleMessage::started(Point::new(4.0, 8.0)),
        },
        &mut context,
    );
    state.library.folder_browser.apply_message(
        crate::native_app::test_support::state::FolderBrowserMessage::HoverDropTarget(
            loops.display().to_string(),
            Point::new(16.0, 18.0),
        ),
    );
    state.apply_message(
        crate::native_app::test_support::state::GuiMessage::DragSampleFile {
            path: source_id,
            drag: DragHandleMessage::ended(Point::new(80.0, 40.0)),
        },
        &mut context,
    );

    state.apply_native_file_drop(
        NativeFileDrop::hover(source.clone(), Some(Point::new(80.0, 40.0)), None),
        &mut context,
    );
    assert_eq!(state.ui.browser_interaction.native_file_drop_hover, None);
    assert_eq!(state.ui.status.sample, "Drag cancelled");

    state.apply_native_file_drop(
        NativeFileDrop::dropped(source.clone(), Some(Point::new(80.0, 40.0)), None),
        &mut context,
    );

    assert!(source.is_file());
    assert!(!drums.join("kick_copy001.wav").exists());
    assert!(!loops.join("kick.wav").exists());
    assert!(!state.library.folder_browser.drag_active());
    assert_eq!(state.ui.status.sample, "Drag cancelled");
    let _ = fs::remove_dir_all(root);
}
