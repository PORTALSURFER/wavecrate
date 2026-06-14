use super::*;

mod active_folder_cache;
mod fixture;
mod keyboard_navigation;
mod normalization;
mod persisted_cache;
mod random_audition;
mod sample_loading;
mod tagged_playback;

use fixture::WaveformPlaybackScenario;

static WAVEFORM_CONFIG_BASE_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());

fn set_waveform_test_config_base(
    path: PathBuf,
) -> (
    std::sync::MutexGuard<'static, ()>,
    wavecrate::app_dirs::ConfigBaseGuard,
) {
    let lock = WAVEFORM_CONFIG_BASE_LOCK
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    let guard = wavecrate::app_dirs::ConfigBaseGuard::set(path);
    (lock, guard)
}

#[test]
fn looped_waveform_click_resolves_to_full_sample_without_playmark() {
    let mut state = gui_state_for_span_tests();
    state.audio.loop_playback = true;

    let span = state.resolve_playback_span(0.45, 1.0, None);

    assert_eq!(span.start_ratio, 0.0);
    assert_eq!(span.end_ratio, 1.0);
    assert_eq!(span.offset_ratio, 0.45);
}

#[test]
fn looped_waveform_click_resolves_to_playmark_span_when_selected() {
    let mut scenario = WaveformPlaybackScenario::synthetic().with_looping();
    scenario.select_play_range(0.25, 0.60);

    let inside_span = scenario.state.resolve_playback_span(0.45, 1.0, None);
    assert_eq!(inside_span.start_ratio, 0.25);
    assert_eq!(inside_span.end_ratio, 0.60);
    assert_eq!(inside_span.offset_ratio, 0.45);

    let outside_span = scenario.state.resolve_playback_span(0.85, 1.0, None);
    assert_eq!(outside_span.start_ratio, 0.25);
    assert_eq!(outside_span.end_ratio, 0.60);
    assert_eq!(outside_span.offset_ratio, 0.25);
}

#[test]
fn file_rename_remaps_loaded_waveform_and_cache_without_reload() {
    let source_root = tempfile::tempdir().expect("source root");
    let old_path = source_root.path().join("loaded.wav");
    write_test_wav_i16(&old_path, &[0, 1024, -1024, 512]);
    let new_path = source_root.path().join("renamed.wav");

    let mut state = gui_state_for_span_tests();
    state.library.folder_browser =
        crate::native_app::test_support::state::FolderBrowserState::from_sample_sources(&[
            wavecrate::sample_sources::SampleSource::new(source_root.path().to_path_buf()),
        ]);
    state
        .library
        .folder_browser
        .select_file(old_path.display().to_string());
    state.waveform.current =
        crate::native_app::test_support::state::WaveformState::load_path(old_path.clone())
            .expect("sample loads");
    let loaded = state.waveform.current.clone();
    state.remember_waveform(&loaded);
    assert!(state.waveform.cache.entries.contains_key(&old_path));
    assert!(
        state
            .waveform
            .cache
            .cached_sample_paths
            .contains(&old_path.display().to_string())
    );

    state
        .library
        .folder_browser
        .begin_rename_selected()
        .expect("rename can start")
        .expect("rename input");
    submit_folder_browser_rename_for_tests(&mut state, "renamed");

    assert_eq!(state.waveform.current.path(), new_path);
    assert!(state.waveform.current.has_loaded_sample());
    assert!(state.waveform.cache.entries.contains_key(&new_path));
    assert!(!state.waveform.cache.entries.contains_key(&old_path));
    assert!(
        state
            .waveform
            .cache
            .cached_sample_paths
            .contains(&new_path.display().to_string())
    );
    assert!(
        !state
            .waveform
            .cache
            .cached_sample_paths
            .contains(&old_path.display().to_string())
    );
    assert!(
        state
            .background
            .deferred_sample_load_task
            .active()
            .is_none()
    );
    assert!(active_sample_load_ticket(&state).is_none());
    let new_id = new_path.display().to_string();
    assert_eq!(
        state.library.folder_browser.selected_file_id(),
        Some(new_id.as_str())
    );
}

#[test]
fn folder_rename_remaps_loaded_waveform_and_cache_without_reload() {
    let source_root = tempfile::tempdir().expect("source root");
    let old_folder = source_root.path().join("drums");
    fs::create_dir_all(&old_folder).expect("create source folder");
    let old_path = old_folder.join("loaded.wav");
    write_test_wav_i16(&old_path, &[0, 1024, -1024, 512]);
    let new_folder = source_root.path().join("breaks");
    let new_path = new_folder.join("loaded.wav");

    let mut state = gui_state_for_span_tests();
    state.library.folder_browser =
        crate::native_app::test_support::state::FolderBrowserState::from_sample_sources(&[
            wavecrate::sample_sources::SampleSource::new(source_root.path().to_path_buf()),
        ]);
    let mut context = ui::UiUpdateContext::default();
    state.apply_message(
        crate::native_app::test_support::state::GuiMessage::FolderBrowser(
            crate::native_app::test_support::state::FolderBrowserMessage::ActivateFolder(
                old_folder.display().to_string(),
                Default::default(),
            ),
        ),
        &mut context,
    );
    state
        .library
        .folder_browser
        .select_file(old_path.display().to_string());
    state.waveform.current =
        crate::native_app::test_support::state::WaveformState::load_path(old_path.clone())
            .expect("sample loads");
    let loaded = state.waveform.current.clone();
    state.remember_waveform(&loaded);

    state.apply_message(
        crate::native_app::test_support::state::GuiMessage::FolderBrowser(
            crate::native_app::test_support::state::FolderBrowserMessage::ActivateFolder(
                old_folder.display().to_string(),
                Default::default(),
            ),
        ),
        &mut context,
    );
    state
        .library
        .folder_browser
        .begin_rename_selected()
        .expect("rename can start")
        .expect("rename input");
    submit_folder_browser_rename_for_tests(&mut state, "breaks");

    assert_eq!(state.waveform.current.path(), new_path);
    assert!(state.waveform.current.has_loaded_sample());
    assert!(state.waveform.cache.entries.contains_key(&new_path));
    assert!(!state.waveform.cache.entries.contains_key(&old_path));
    assert!(
        state
            .waveform
            .cache
            .cached_sample_paths
            .contains(&new_path.display().to_string())
    );
    assert!(
        !state
            .waveform
            .cache
            .cached_sample_paths
            .contains(&old_path.display().to_string())
    );
    assert!(
        state
            .background
            .deferred_sample_load_task
            .active()
            .is_none()
    );
    assert!(active_sample_load_ticket(&state).is_none());
}

fn wait_for_playback_ready_cache(sample_path: &str) {
    let path = PathBuf::from(sample_path);
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(2);
    while std::time::Instant::now() < deadline {
        if super::super::waveform::cached_waveform_file_playback_ready_exists(&path) {
            return;
        }
        std::thread::sleep(std::time::Duration::from_millis(10));
    }
}

fn active_sample_load_ticket(state: &NativeAppState) -> Option<ui::TaskTicket> {
    state.active_sample_load_task()
}

fn persisted_cache_warm_ticket(state: &NativeAppState) -> Option<ui::TaskTicket> {
    let key = state.waveform.cache.warm_key.as_ref()?;
    state.waveform.cache.warm_tasks.active(key)
}

fn active_folder_cache_warm_ticket(state: &NativeAppState) -> Option<ui::TaskTicket> {
    let key = state.waveform.cache.active_folder_warm_key.as_ref()?;
    state.waveform.cache.active_folder_warm_tasks.active(key)
}

fn active_folder_cache_warm_completion(
    ticket: ui::TaskTicket,
    folder_id: String,
    loaded: Vec<(
        PathBuf,
        std::sync::Arc<crate::native_app::waveform::WaveformFile>,
    )>,
    cancelled: bool,
) -> ui::KeyedTaskCompletion<ui::ResourceKey, crate::native_app::app::ActiveFolderCacheWarmResult> {
    ui::KeyedTaskCompletion {
        key: crate::native_app::audio::sample_load_actions::active_folder_cache_warm_resource_key(
            folder_id.as_str(),
        ),
        ticket,
        output: crate::native_app::app::ActiveFolderCacheWarmResult {
            folder_id,
            loaded,
            cancelled,
        },
    }
}

fn sample_load_completion(
    ticket: ui::TaskTicket,
    path: String,
    result: Result<crate::native_app::test_support::state::WaveformState, String>,
    autoplay: bool,
) -> ui::KeyedTaskCompletion<
    ui::ResourceKey,
    crate::native_app::test_support::state::SampleLoadResult,
> {
    ui::KeyedTaskCompletion {
        key: crate::native_app::audio::sample_load_actions::sample_resource_key(path.as_str()),
        ticket,
        output: crate::native_app::test_support::state::SampleLoadResult {
            path,
            result,
            autoplay,
        },
    }
}

fn sample_playback_ready_completion(
    ticket: ui::TaskTicket,
    path: String,
    audio: crate::native_app::waveform::WaveformPlaybackReady,
    autoplay: bool,
) -> ui::KeyedTaskCompletion<
    ui::ResourceKey,
    crate::native_app::test_support::state::SamplePlaybackReady,
> {
    ui::KeyedTaskCompletion {
        key: crate::native_app::audio::sample_load_actions::sample_resource_key(path.as_str()),
        ticket,
        output: crate::native_app::test_support::state::SamplePlaybackReady {
            path,
            audio,
            autoplay,
        },
    }
}

#[test]
fn selecting_another_sample_cancels_metadata_tag_entry() {
    let source_root = tempfile::tempdir().expect("source root");
    let first_path = source_root.path().join("first.wav");
    let second_path = source_root.path().join("second.wav");
    fs::write(&first_path, []).expect("first sample");
    fs::write(&second_path, []).expect("second sample");

    let mut state = gui_state_for_span_tests();
    state.library.folder_browser =
        crate::native_app::test_support::state::FolderBrowserState::from_sample_sources(&[
            wavecrate::sample_sources::SampleSource::new(source_root.path().to_path_buf()),
        ]);
    let first_file = first_path.display().to_string();
    let second_file = second_path.display().to_string();
    state.library.folder_browser.select_file(first_file.clone());
    state.metadata.tag_draft = String::from("ki");
    state.metadata.tag_tokens = vec![String::from("warm")];
    state.metadata.tag_input_mode =
        crate::native_app::test_support::waveform::MetadataTagInputMode::Category {
            pending_tag: String::from("new-tag"),
        };
    state.metadata.tag_completion_cycle.select("ki", 2, 4);

    state.select_sample_with_modifiers(
        second_file.clone(),
        PointerModifiers::default(),
        &mut ui::UiUpdateContext::default(),
    );

    assert_eq!(
        state.library.folder_browser.selected_file_id(),
        Some(second_file.as_str())
    );
    assert!(state.metadata.tag_draft.is_empty());
    assert!(state.metadata.tag_tokens.is_empty());
    assert_eq!(
        state.metadata.tag_input_mode,
        crate::native_app::test_support::waveform::MetadataTagInputMode::Tag
    );
    assert_eq!(state.metadata.tag_completion_cycle.query_key(), None);
    assert_eq!(state.metadata.tag_completion_cycle.stored_index(), 0);
    assert_eq!(state.pending_metadata_tag_category_tag(), None);
    assert!(!state.metadata_tag_completion_active());
}

#[test]
fn play_selected_sample_uses_active_playmark_selection_span() {
    let Some(mut scenario) = WaveformPlaybackScenario::default_loaded_with_player() else {
        return;
    };
    scenario.select_play_range(0.25, 0.60);
    scenario.state.audio.loop_playback = true;

    scenario.play_selected_sample();

    assert!(scenario.state.waveform.current.is_playing());
    assert_eq!(
        scenario.state.waveform.current.play_mark_ratio(),
        Some(0.25)
    );
    assert_eq!(
        scenario.state.audio.current_playback_span,
        Some((0.25, 0.6))
    );
    assert!(
        scenario
            .state
            .audio
            .player
            .as_ref()
            .is_some_and(|player| player.is_looping())
    );
    let progress = scenario
        .state
        .audio
        .player
        .as_ref()
        .and_then(|player| player.progress())
        .expect("playback progress");
    assert!(
        (0.24..=0.35).contains(&progress),
        "spacebar playback should start inside the playmark selection, got {progress}"
    );
}

#[test]
fn looped_playback_retargets_when_playmark_selection_is_created_and_resized() {
    let Some(mut scenario) = WaveformPlaybackScenario::default_loaded_with_player() else {
        return;
    };
    scenario.start_full_sample_loop();
    assert_player_progress_inside_span(&scenario.state, 0.0, 1.0);

    scenario.select_play_range(0.25, 0.60);

    assert_playback_span_state(&scenario.state, 0.25, 0.60);
    assert_player_progress_inside_span(&scenario.state, 0.25, 0.60);
    assert!(
        scenario
            .state
            .audio
            .player
            .as_ref()
            .is_some_and(|player| player.is_looping())
    );

    scenario.resize_play_range_start(0.25, 0.10);

    assert_playback_span_state(&scenario.state, 0.10, 0.60);
    assert_player_progress_inside_span(&scenario.state, 0.10, 0.60);
}

fn assert_playback_span_state(state: &NativeAppState, expected_start: f32, expected_end: f32) {
    let (start, end) = state
        .audio
        .current_playback_span
        .expect("current playback span should be set");
    assert!(
        (start - expected_start).abs() < 0.001,
        "start {start}, expected {expected_start}"
    );
    assert!(
        (end - expected_end).abs() < 0.001,
        "end {end}, expected {expected_end}"
    );
}

fn assert_player_progress_inside_span(state: &NativeAppState, start: f32, end: f32) {
    let progress = state
        .audio
        .player
        .as_ref()
        .and_then(|player| player.progress())
        .expect("audio player progress should be available");
    assert!(
        progress >= start - 0.02 && progress <= end + 0.02,
        "progress {progress}, expected inside {start}..={end}"
    );
}
