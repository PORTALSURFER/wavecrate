use super::*;
use crate::native_app::app::MetadataMessage;
use crate::native_app::app_chrome::view_models::sample_browser::prepare_sample_browser_view;
use crate::native_app::test_support::sample_browser::complete_starmap_layout_for_selected_source;
use crate::native_app::test_support::state::{FolderBrowserState, NativeAppStateFixture};
use std::{fs, sync::Arc, time::Duration};

#[test]
fn root_dispatch_routes_metadata_messages_to_metadata_owner() {
    let mut state = NativeAppState::load_default().expect("default state loads");

    state.apply_message(
        GuiMessage::Metadata(MetadataMessage::ToggleMetadataTagLibrary),
        &mut ui::UiUpdateContext::default(),
    );

    assert!(state.metadata.tag_library_open);
}

#[test]
fn frame_messages_use_frame_budget_slow_threshold() {
    assert_eq!(
        slow_ui_message_threshold(FRAME_MESSAGE_PROFILE_LABEL),
        Duration::from_micros(16_667)
    );
    assert_eq!(
        slow_ui_message_threshold("NavigateBrowser"),
        Duration::from_millis(4)
    );
}

#[test]
fn source_processing_progress_opens_the_shared_job_details_popover() {
    let mut state = NativeAppStateFixture::default().build();
    let source_id = state
        .library
        .folder_browser
        .defer_add_source_path(std::path::PathBuf::from("/tmp/progress-source"), false)
        .expect("source registered");
    state.background.source_processing_progress =
        Some(crate::native_app::app::SourceProcessingProgress {
            source_id: source_id.clone(),
            lifecycle_generation: 0,
            active: true,
            completed: 3,
            total: 10,
            stage: String::from("Preparing similarity"),
            detail: String::from("kick.wav"),
        });

    state.apply_message(
        GuiMessage::ToggleJobDetails,
        &mut ui::UiUpdateContext::default(),
    );

    assert!(state.ui.chrome.job_details_open);

    state.apply_message(
        GuiMessage::SourceProcessingProgress(crate::native_app::app::SourceProcessingProgress {
            source_id,
            lifecycle_generation: 0,
            active: false,
            completed: 10,
            total: 10,
            stage: String::from("Building similarity layout"),
            detail: String::from("Finalizing source"),
        }),
        &mut ui::UiUpdateContext::default(),
    );

    assert!(state.background.source_processing_progress.is_none());
    assert!(!state.ui.chrome.job_details_open);
}

#[test]
fn source_processing_progress_refreshes_the_retained_details_projection() {
    let mut state = NativeAppStateFixture::default().build();
    let source_id = state
        .library
        .folder_browser
        .defer_add_source_path(std::path::PathBuf::from("/tmp/progress-source"), false)
        .expect("source registered");
    let mut context = ui::UiUpdateContext::default();

    state.apply_message(
        GuiMessage::SourceProcessingProgress(crate::native_app::app::SourceProcessingProgress {
            source_id,
            lifecycle_generation: 0,
            active: true,
            completed: 4,
            total: 10,
            stage: String::from("Preparing similarity"),
            detail: String::from("snare.wav"),
        }),
        &mut context,
    );

    assert_eq!(
        context.into_command().repaint_scope(),
        Some(ui::RepaintScope::Projection),
        "background progress must refresh retained text and counters without user input"
    );
}

#[test]
fn late_progress_for_removed_source_is_ignored() {
    let mut state = NativeAppStateFixture::default().build();
    let mut context = ui::UiUpdateContext::default();

    state.apply_message(
        GuiMessage::SourceProcessingProgress(crate::native_app::app::SourceProcessingProgress {
            source_id: String::from("retired-source"),
            lifecycle_generation: 0,
            active: true,
            completed: 1,
            total: 10,
            stage: String::from("Preparing similarity"),
            detail: String::from("late.wav"),
        }),
        &mut context,
    );

    assert!(state.background.source_processing_progress.is_none());
}

#[test]
fn late_progress_from_previous_readded_source_epoch_is_ignored() {
    let mut state = NativeAppStateFixture::default().build();
    let source_id = state
        .library
        .folder_browser
        .defer_add_source_path(
            std::path::PathBuf::from("/tmp/readded-progress-source"),
            false,
        )
        .expect("source registered");
    state
        .background
        .source_lifecycle_generations
        .insert(source_id.clone(), 2);
    let mut context = ui::UiUpdateContext::default();

    state.apply_message(
        GuiMessage::SourceProcessingProgress(crate::native_app::app::SourceProcessingProgress {
            source_id: source_id.clone(),
            lifecycle_generation: 1,
            active: true,
            completed: 1,
            total: 10,
            stage: String::from("Preparing similarity"),
            detail: String::from("old-epoch.wav"),
        }),
        &mut context,
    );
    assert!(state.background.source_processing_progress.is_none());

    state.apply_message(
        GuiMessage::SourceProcessingProgress(crate::native_app::app::SourceProcessingProgress {
            source_id,
            lifecycle_generation: 2,
            active: true,
            completed: 2,
            total: 10,
            stage: String::from("Preparing similarity"),
            detail: String::from("current-epoch.wav"),
        }),
        &mut context,
    );
    assert_eq!(
        state
            .background
            .source_processing_progress
            .as_ref()
            .map(|progress| progress.lifecycle_generation),
        Some(2)
    );
}

#[test]
fn idle_map_frame_reuses_prepared_sample_browser_projection() {
    let (_root, mut state, _sample_id) = prepared_map_state("idle-map-frame.wav");
    let prepared = state
        .library
        .folder_browser
        .cached_starmap_projection()
        .expect("prepared starmap projection");

    state.handle_message(GuiMessage::Frame, &mut ui::UiUpdateContext::default());

    let after_frame = state
        .library
        .folder_browser
        .cached_starmap_projection()
        .expect("starmap projection remains prepared");
    assert!(
        Arc::ptr_eq(&prepared, &after_frame),
        "paint-only frame traffic must not rebuild an unchanged starmap projection"
    );
}

#[test]
fn map_frame_rebuilds_projection_when_copy_flash_expires() {
    let (_root, mut state, sample_id) = prepared_map_state("copy-flash-map-frame.wav");
    state
        .library
        .folder_browser
        .flash_copied_file_paths([sample_id.as_str()]);
    prepare_sample_browser_view(&mut state);
    let flashed = state
        .library
        .folder_browser
        .cached_starmap_projection()
        .expect("prepared flashed starmap projection");
    assert!(flashed[0].copy_flash);

    while state.library.folder_browser.copy_flash_frames() > 1 {
        state.handle_message(GuiMessage::Frame, &mut ui::UiUpdateContext::default());
        let during_flash = state
            .library
            .folder_browser
            .cached_starmap_projection()
            .expect("starmap projection remains prepared during flash");
        assert!(Arc::ptr_eq(&flashed, &during_flash));
    }
    state.handle_message(GuiMessage::Frame, &mut ui::UiUpdateContext::default());

    let after_flash = state
        .library
        .folder_browser
        .cached_starmap_projection()
        .expect("starmap projection refreshed after flash");
    assert!(!Arc::ptr_eq(&flashed, &after_flash));
    assert!(!after_flash[0].copy_flash);
}

fn prepared_map_state(file_name: &str) -> (tempfile::TempDir, NativeAppState, String) {
    let root = tempfile::tempdir().expect("source root");
    let sample = root.path().join(file_name);
    fs::write(&sample, [0_u8; 8]).expect("write sample");
    let sample_id = sample.to_string_lossy().into_owned();
    let mut state = NativeAppStateFixture::default()
        .with_folder_browser(FolderBrowserState::from_root(root.path().to_path_buf()))
        .build();
    state.ui.chrome.sample_browser_display = crate::native_app::app::SampleBrowserDisplayMode::Map;
    complete_starmap_layout_for_selected_source(&mut state);
    prepare_sample_browser_view(&mut state);
    (root, state, sample_id)
}
