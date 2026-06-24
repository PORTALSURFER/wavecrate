use super::*;

mod active_folder_cache;
mod fixture;
mod history;
mod keyboard_navigation;
mod normalization;
mod persisted_cache;
mod random_audition;
mod sample_loading;
mod tagged_playback;

use fixture::WaveformPlaybackScenario;

static WAVEFORM_CONFIG_BASE_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());

fn install_playback_runtime_for_tests(state: &mut NativeAppState) -> bool {
    if !test_audio_output_enabled() {
        return false;
    }
    let Ok(player) = wavecrate::audio::AudioPlayer::new() else {
        return false;
    };
    let output = player.output_details().clone();
    let Ok(runtime) = wavecrate::audio::PlaybackRuntime::spawn(
        player,
        wavecrate::audio::PlaybackRuntimeConfig::default(),
    ) else {
        return false;
    };
    state.audio.output_resolved = Some(output);
    state.audio.playback_runtime = Some(runtime.handle);
    state.audio.playback_events = Some(runtime.events);
    true
}

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
    state
        .waveform
        .cache
        .active_folder_warm_pending
        .push_back(old_path.clone());
    state.waveform.cache.active_folder_warm_current = Some(old_path.clone());

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
    assert_eq!(
        state.waveform.cache.active_folder_warm_current.as_deref(),
        Some(new_path.as_path())
    );
    assert_eq!(
        state
            .waveform
            .cache
            .active_folder_warm_pending
            .iter()
            .collect::<Vec<_>>(),
        vec![&new_path]
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
    assert!(
        super::super::waveform::cached_waveform_file_playback_ready_exists(&path),
        "playback-ready waveform cache marker was not written for {}",
        path.display()
    );
}

fn active_sample_load_ticket(state: &NativeAppState) -> Option<ui::TaskTicket> {
    state.active_sample_load_task()
}

fn active_sample_load_validation_ticket(state: &NativeAppState) -> Option<ui::TaskTicket> {
    state.background.sample_load_validation_task.active()
}

fn persisted_cache_warm_ticket(state: &NativeAppState) -> Option<ui::TaskTicket> {
    let key = state.waveform.cache.warm_key.as_ref()?;
    state.waveform.cache.warm_tasks.active(key)
}

fn active_folder_cache_warm_plan_ticket(state: &NativeAppState) -> Option<ui::TaskTicket> {
    state.waveform.cache.active_folder_warm_plan_task.active()
}

fn finish_active_folder_cache_warm_plan(
    state: &mut NativeAppState,
    context: &mut ui::UiUpdateContext<crate::native_app::test_support::state::GuiMessage>,
    folder_id: String,
    playback_ready: Vec<PathBuf>,
    pending: Vec<PathBuf>,
) {
    let ticket = active_folder_cache_warm_plan_ticket(state).expect("source warm plan task");
    state.apply_message(
        crate::native_app::test_support::state::GuiMessage::ActiveFolderCacheWarmPlanned(
            ui::TaskCompletion {
                ticket,
                output: crate::native_app::app::ActiveFolderCacheWarmPlanResult {
                    folder_id,
                    playback_ready,
                    pending,
                    cancelled: false,
                },
            },
        ),
        context,
    );
}

fn active_folder_cache_warm_ticket(state: &NativeAppState) -> Option<ui::TaskTicket> {
    let key = state.waveform.cache.active_folder_warm_key.as_ref()?;
    state.waveform.cache.active_folder_warm_tasks.active(key)
}

fn active_folder_cache_warm_completion_with_deferred(
    ticket: ui::TaskTicket,
    folder_id: String,
    loaded: Vec<(
        PathBuf,
        std::sync::Arc<crate::native_app::waveform::WaveformFile>,
    )>,
    deferred: Vec<PathBuf>,
    processed: usize,
    decoded_source: bool,
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
            playback_ready: Vec::new(),
            deferred,
            processed,
            decoded_source,
            cancelled,
        },
    }
}

fn platform_copy_file_path_count(
    command: radiant::runtime::Command<crate::native_app::test_support::state::GuiMessage>,
) -> Option<usize> {
    match command {
        radiant::runtime::Command::PlatformRequest {
            request: ui::PlatformRequest::CopyFilePaths(paths),
            ..
        } => Some(paths.len()),
        radiant::runtime::Command::Batch(commands) => {
            commands.into_iter().find_map(platform_copy_file_path_count)
        }
        _ => None,
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
    assert!(scenario.state.audio.loop_playback);
    let progress = scenario
        .state
        .waveform
        .current
        .playhead_ratio()
        .expect("playback progress");
    assert!(
        (0.24..=0.35).contains(&progress),
        "spacebar playback should start inside the playmark selection, got {progress}"
    );
}

#[test]
fn playmark_selection_copy_uses_interactive_handoff_worker() {
    let mut scenario =
        WaveformPlaybackScenario::with_temp_wav("playmark-copy.wav", &[0, 1024, -1024, 512]);
    load_selected_sample_into_waveform(&mut scenario);
    scenario.select_play_range(0.25, 0.60);
    let warm_cancel = ui::CancellationToken::new();
    scenario.state.waveform.cache.active_folder_warm_cancel = Some(warm_cancel.clone());

    let mut context = ui::UiUpdateContext::default();
    scenario.state.copy_selected_files(&mut context);

    assert!(warm_cancel.is_cancelled());
    assert!(
        scenario
            .state
            .waveform
            .cache
            .active_folder_warm_cancel
            .is_none()
    );
    assert_eq!(
        context
            .into_command()
            .business_task_priority("gui-copy-waveform-selection"),
        Some(ui::TaskPriority::Interactive),
        "playmark clipboard staging must not queue behind cache warm workers"
    );
}

#[test]
fn playmark_extraction_tags_new_file_as_one_shot_by_default() {
    let mut scenario = WaveformPlaybackScenario::with_temp_wav(
        "playmark-extract-one-shot.wav",
        &[0, 1024, -1024, 512],
    );
    load_selected_sample_into_waveform(&mut scenario);
    scenario.select_play_range(0.25, 0.60);

    let extracted = run_playmark_extraction(&mut scenario);

    assert_eq!(
        scenario
            .state
            .metadata
            .tags_by_file
            .get(&extracted.to_string_lossy().to_string()),
        Some(&vec![String::from("one-shot")])
    );
}

#[test]
fn playmark_extraction_tags_new_file_as_loop_when_looping_at_request_time() {
    let mut scenario = WaveformPlaybackScenario::with_temp_wav(
        "playmark-extract-loop.wav",
        &[0, 1024, -1024, 512],
    );
    load_selected_sample_into_waveform(&mut scenario);
    scenario.select_play_range(0.25, 0.60);
    scenario.state.audio.loop_playback = true;

    let mut context = ui::UiUpdateContext::default();
    scenario.state.apply_message(
        crate::native_app::test_support::state::GuiMessage::ExtractPlaymarkedRange,
        &mut context,
    );
    scenario.state.audio.loop_playback = false;
    let extracted = extraction_path_for_loaded_sample(&scenario);
    run_command_for_tests(&mut scenario.state, context.into_command());

    assert_eq!(
        scenario
            .state
            .metadata
            .tags_by_file
            .get(&extracted.to_string_lossy().to_string()),
        Some(&vec![String::from("loop")])
    );
}

#[test]
fn playmark_selection_copy_flashes_on_submit_and_ready() {
    let mut scenario =
        WaveformPlaybackScenario::with_temp_wav("playmark-copy-ready.wav", &[0, 1024, -1024, 512]);
    load_selected_sample_into_waveform(&mut scenario);
    scenario.select_play_range(0.25, 0.60);

    let mut context = ui::UiUpdateContext::default();
    scenario.state.copy_selected_files(&mut context);

    assert!(
        scenario
            .state
            .waveform
            .current
            .play_selection_flash_active()
    );
    drain_play_selection_flash(&mut scenario.state);
    assert!(
        !scenario
            .state
            .waveform
            .current
            .play_selection_flash_active()
    );

    let source_path = scenario.state.waveform.current.path();
    let selection = scenario
        .state
        .waveform
        .current
        .play_selection()
        .expect("play selection");
    scenario.state.finish_waveform_selection_copy(
        source_path,
        selection,
        std::time::Instant::now(),
        Ok(PathBuf::from("/tmp/wavecrate-staged-clip.wav")),
    );

    assert!(
        scenario
            .state
            .waveform
            .current
            .play_selection_flash_active()
    );
}

#[test]
fn playmark_selection_copy_ready_flash_ignores_stale_range() {
    let mut scenario =
        WaveformPlaybackScenario::with_temp_wav("playmark-copy-stale.wav", &[0, 1024, -1024, 512]);
    load_selected_sample_into_waveform(&mut scenario);
    scenario.select_play_range(0.25, 0.60);
    let source_path = scenario.state.waveform.current.path();
    let copied_selection = scenario
        .state
        .waveform
        .current
        .play_selection()
        .expect("play selection");
    drain_play_selection_flash(&mut scenario.state);

    scenario
        .state
        .waveform
        .current
        .set_play_selection_range(0.10, 0.20);
    scenario.state.finish_waveform_selection_copy(
        source_path,
        copied_selection,
        std::time::Instant::now(),
        Ok(PathBuf::from("/tmp/wavecrate-staged-clip.wav")),
    );

    assert!(
        !scenario
            .state
            .waveform
            .current
            .play_selection_flash_active()
    );
}

#[test]
fn playmark_selection_clip_ready_queues_platform_clipboard_handoff() {
    let mut scenario =
        WaveformPlaybackScenario::with_temp_wav("playmark-copy-platform.wav", &[0, 1024, -1024]);
    load_selected_sample_into_waveform(&mut scenario);
    scenario.select_play_range(0.25, 0.60);
    let source_path = scenario.state.waveform.current.path();
    let selection = scenario
        .state
        .waveform
        .current
        .play_selection()
        .expect("play selection");
    let staged_path = PathBuf::from("/tmp/wavecrate-staged-clip.wav");

    let mut context = ui::UiUpdateContext::default();
    scenario.state.finish_waveform_selection_clip_staged(
        source_path,
        selection,
        std::time::Instant::now(),
        Ok(staged_path),
        &mut context,
    );

    assert_eq!(
        platform_copy_file_path_count(context.into_command()),
        Some(1),
        "staged waveform clips should use Radiant's typed file-path clipboard service"
    );
}

#[test]
fn whole_file_copy_uses_radiant_platform_clipboard_handoff() {
    let mut scenario = WaveformPlaybackScenario::with_temp_wav("whole-file-copy.wav", &[0, 1024]);

    let mut context = ui::UiUpdateContext::default();
    scenario.state.copy_selected_files(&mut context);

    assert_eq!(
        platform_copy_file_path_count(context.into_command()),
        Some(1),
        "whole-file clipboard handoff should use Radiant's typed platform service"
    );
}

#[test]
fn playmark_selection_change_undoes_and_redoes_through_transactions() {
    let mut scenario = WaveformPlaybackScenario::synthetic();

    scenario.select_play_range(0.20, 0.50);

    assert_play_selection_state(&scenario.state, Some((0.20, 0.50)), Some(0.20));
    assert_eq!(
        scenario.state.waveform.current.marked_play_ranges().len(),
        1
    );
    assert_eq!(scenario.state.transactions.history.list_items().len(), 1);

    apply_transaction_message(
        &mut scenario.state,
        crate::native_app::test_support::state::GuiMessage::UndoTransaction,
    );

    assert_play_selection_state(&scenario.state, None, None);
    assert!(
        scenario
            .state
            .waveform
            .current
            .marked_play_ranges()
            .is_empty()
    );
    assert!(scenario.state.transactions.history.can_redo());

    apply_transaction_message(
        &mut scenario.state,
        crate::native_app::test_support::state::GuiMessage::RedoTransaction,
    );

    assert_play_selection_state(&scenario.state, Some((0.20, 0.50)), Some(0.20));
    assert_eq!(
        scenario.state.waveform.current.marked_play_ranges().len(),
        1
    );
}

#[test]
fn playmark_resize_registers_one_undoable_selection_change() {
    let mut scenario = WaveformPlaybackScenario::synthetic();
    scenario.select_play_range(0.20, 0.40);

    scenario.begin_play_range_end_resize(0.40);
    scenario.update_play_range_drag(0.60);
    scenario.update_play_range_drag(0.70);
    scenario.finish_play_range_drag(0.70);

    assert_play_selection_state(&scenario.state, Some((0.20, 0.70)), Some(0.20));
    assert_eq!(
        scenario.state.transactions.history.list_items().len(),
        2,
        "the initial selection and the completed resize should be separate transaction entries"
    );

    apply_transaction_message(
        &mut scenario.state,
        crate::native_app::test_support::state::GuiMessage::UndoTransaction,
    );

    assert_play_selection_state(&scenario.state, Some((0.20, 0.40)), Some(0.20));

    apply_transaction_message(
        &mut scenario.state,
        crate::native_app::test_support::state::GuiMessage::RedoTransaction,
    );

    assert_play_selection_state(&scenario.state, Some((0.20, 0.70)), Some(0.20));
}

fn load_selected_sample_into_waveform(scenario: &mut WaveformPlaybackScenario) {
    let selected_file = scenario
        .state
        .library
        .folder_browser
        .selected_file_id()
        .map(ToOwned::to_owned)
        .expect("scenario should have a selected sample");
    scenario.state.waveform.current =
        crate::native_app::test_support::state::WaveformState::load_path(PathBuf::from(
            selected_file,
        ))
        .expect("test sample loads");
}

fn apply_transaction_message(
    state: &mut NativeAppState,
    message: crate::native_app::test_support::state::GuiMessage,
) {
    state.apply_message(message, &mut ui::UiUpdateContext::default());
}

fn assert_play_selection_state(
    state: &NativeAppState,
    expected_selection: Option<(f32, f32)>,
    expected_mark: Option<f32>,
) {
    match (state.waveform.current.play_mark_ratio(), expected_mark) {
        (None, None) => {}
        (Some(actual), Some(expected)) => {
            assert!(
                (actual - expected).abs() < 0.001,
                "play mark was {actual}, expected {expected}"
            );
        }
        (actual, expected) => panic!("expected play mark {expected:?}, got {actual:?}"),
    }

    match (state.waveform.current.play_selection(), expected_selection) {
        (None, None) => {}
        (Some(selection), Some((expected_start, expected_end))) => {
            assert!(
                (selection.start() - expected_start).abs() < 0.001,
                "selection start was {}, expected {expected_start}",
                selection.start()
            );
            assert!(
                (selection.end() - expected_end).abs() < 0.001,
                "selection end was {}, expected {expected_end}",
                selection.end()
            );
        }
        (actual, expected) => panic!("expected play selection {expected:?}, got {actual:?}"),
    }
}

fn drain_play_selection_flash(state: &mut NativeAppState) {
    for _ in 0..32 {
        state
            .waveform
            .current
            .apply_interaction(WaveformInteraction::Frame);
    }
}

fn run_playmark_extraction(scenario: &mut WaveformPlaybackScenario) -> PathBuf {
    let extracted = extraction_path_for_loaded_sample(scenario);
    let mut context = ui::UiUpdateContext::default();
    scenario.state.apply_message(
        crate::native_app::test_support::state::GuiMessage::ExtractPlaymarkedRange,
        &mut context,
    );
    run_command_for_tests(&mut scenario.state, context.into_command());
    extracted
}

fn extraction_path_for_loaded_sample(scenario: &WaveformPlaybackScenario) -> PathBuf {
    let source_path = scenario.state.waveform.current.path();
    let stem = source_path
        .file_stem()
        .map(|stem| stem.to_string_lossy())
        .expect("test sample should have a stem");
    source_path.with_file_name(format!("{stem}_extraction.wav"))
}

#[test]
fn looped_playback_retargets_when_playmark_selection_is_created_and_resized() {
    let Some(mut scenario) = WaveformPlaybackScenario::default_loaded_with_player() else {
        return;
    };
    scenario.start_full_sample_loop();
    assert_waveform_progress_inside_span(&scenario.state, 0.0, 1.0);

    scenario.begin_play_range(0.25);
    scenario.update_play_range_drag(0.60);

    assert_playback_span_state(&scenario.state, 0.25, 0.60);
    assert_waveform_progress_inside_span(&scenario.state, 0.25, 0.60);
    assert!(scenario.state.audio.loop_playback);
    scenario.finish_play_range_drag(0.60);

    scenario.begin_play_range_start_resize(0.25);
    scenario.update_play_range_drag(0.10);

    assert_playback_span_state(&scenario.state, 0.10, 0.60);
    assert_waveform_progress_inside_span(&scenario.state, 0.10, 0.60);
    scenario.finish_play_range_drag(0.10);
}

#[test]
fn looped_playback_retarget_keeps_current_cycle_when_playhead_still_fits() {
    let Some(mut scenario) = WaveformPlaybackScenario::default_loaded_with_player() else {
        return;
    };
    scenario.start_full_sample_loop();
    scenario.select_play_range(0.20, 0.60);
    scenario.begin_play_range_end_resize(0.60);

    scenario.state.waveform.current.set_playhead_ratio(0.50);
    let playback_start_id = pending_runtime_playback_start_id(&scenario.state);
    scenario.update_play_range_drag(0.80);

    assert_eq!(
        pending_runtime_playback_start_id(&scenario.state),
        playback_start_id
    );
    assert_playback_span_state(&scenario.state, 0.20, 0.80);
    assert_waveform_progress_near(&scenario.state, 0.50);

    scenario.state.waveform.current.set_playhead_ratio(0.50);
    let playback_start_id = pending_runtime_playback_start_id(&scenario.state);
    scenario.update_play_range_drag(0.65);

    assert_eq!(
        pending_runtime_playback_start_id(&scenario.state),
        playback_start_id
    );
    assert_playback_span_state(&scenario.state, 0.20, 0.65);
    assert_waveform_progress_near(&scenario.state, 0.50);
    scenario.finish_play_range_drag(0.65);
}

#[test]
fn looped_playback_retarget_restarts_when_playhead_is_past_new_end() {
    let Some(mut scenario) = WaveformPlaybackScenario::default_loaded_with_player() else {
        return;
    };
    scenario.start_full_sample_loop();
    scenario.select_play_range(0.20, 0.80);
    scenario.begin_play_range_end_resize(0.80);

    scenario.state.waveform.current.set_playhead_ratio(0.70);
    let playback_start_id = pending_runtime_playback_start_id(&scenario.state);
    scenario.update_play_range_drag(0.55);

    assert_eq!(
        pending_runtime_playback_start_id(&scenario.state),
        playback_start_id
    );
    assert_playback_span_state(&scenario.state, 0.20, 0.55);
    assert_waveform_progress_near(&scenario.state, 0.20);
    scenario.finish_play_range_drag(0.55);
}

#[test]
fn one_shot_playback_retargets_when_playmark_selection_is_created_and_resized() {
    let Some(mut scenario) = WaveformPlaybackScenario::default_loaded_with_player() else {
        return;
    };
    scenario
        .state
        .start_playback_current_span(0.0, 1.0)
        .expect("full sample playback starts");
    assert_waveform_progress_inside_span(&scenario.state, 0.0, 1.0);

    scenario.begin_play_range(0.25);
    scenario.update_play_range_drag(0.60);

    assert_playback_span_state(&scenario.state, 0.25, 0.60);
    assert_waveform_progress_near(&scenario.state, 0.25);
    assert!(!scenario.state.audio.loop_playback);
    scenario.finish_play_range_drag(0.60);

    scenario.begin_play_range_start_resize(0.25);
    scenario.update_play_range_drag(0.10);

    assert_playback_span_state(&scenario.state, 0.10, 0.60);
    assert_waveform_progress_near(&scenario.state, 0.25);
    scenario.finish_play_range_drag(0.10);
}

#[test]
fn one_shot_playback_retarget_keeps_current_pass_when_playhead_still_fits() {
    let Some(mut scenario) = WaveformPlaybackScenario::default_loaded_with_player() else {
        return;
    };
    scenario.select_play_range(0.20, 0.60);
    scenario
        .state
        .start_playback_current_span(0.20, 0.60)
        .expect("playmark playback starts");
    scenario.begin_play_range_end_resize(0.60);

    scenario.state.waveform.current.set_playhead_ratio(0.50);
    let playback_start_id = pending_runtime_playback_start_id(&scenario.state);
    scenario.update_play_range_drag(0.80);

    assert_eq!(
        pending_runtime_playback_start_id(&scenario.state),
        playback_start_id
    );
    assert_playback_span_state(&scenario.state, 0.20, 0.80);
    assert_waveform_progress_near(&scenario.state, 0.50);

    scenario.state.waveform.current.set_playhead_ratio(0.50);
    let playback_start_id = pending_runtime_playback_start_id(&scenario.state);
    scenario.update_play_range_drag(0.65);

    assert_eq!(
        pending_runtime_playback_start_id(&scenario.state),
        playback_start_id
    );
    assert_playback_span_state(&scenario.state, 0.20, 0.65);
    assert_waveform_progress_near(&scenario.state, 0.50);
    scenario.finish_play_range_drag(0.65);
}

#[test]
fn one_shot_playback_retarget_restarts_when_playhead_is_past_new_end() {
    let Some(mut scenario) = WaveformPlaybackScenario::default_loaded_with_player() else {
        return;
    };
    scenario.select_play_range(0.20, 0.80);
    scenario
        .state
        .start_playback_current_span(0.20, 0.80)
        .expect("playmark playback starts");
    scenario.begin_play_range_end_resize(0.80);

    scenario.state.waveform.current.set_playhead_ratio(0.70);
    let playback_start_id = pending_runtime_playback_start_id(&scenario.state);
    scenario.update_play_range_drag(0.55);

    assert_eq!(
        pending_runtime_playback_start_id(&scenario.state),
        playback_start_id
    );
    assert_playback_span_state(&scenario.state, 0.20, 0.55);
    assert_waveform_progress_near(&scenario.state, 0.20);
    scenario.finish_play_range_drag(0.55);
}

fn pending_runtime_playback_start_id(state: &NativeAppState) -> Option<u64> {
    state
        .audio
        .pending_runtime_start
        .as_ref()
        .map(|pending| pending.id.get())
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

fn assert_waveform_progress_inside_span(state: &NativeAppState, start: f32, end: f32) {
    let progress = state
        .waveform
        .current
        .playhead_ratio()
        .expect("waveform progress should be available");
    assert!(
        progress >= start - 0.02 && progress <= end + 0.02,
        "progress {progress}, expected inside {start}..={end}"
    );
}

fn assert_waveform_progress_near(state: &NativeAppState, expected: f32) {
    let progress = state
        .waveform
        .current
        .playhead_ratio()
        .expect("waveform progress should be available");
    assert!(
        (progress - expected).abs() < 0.02,
        "progress {progress}, expected near {expected}"
    );
}
