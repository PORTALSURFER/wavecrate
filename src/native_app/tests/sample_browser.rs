use radiant::{
    gui::types::{Point, Rect, Rgba8, Vector2},
    prelude::{IntoView, ThemeTokens, WidgetStyle, WidgetTone, dense_row_palette_from_style},
    runtime::{Command, Event, SurfaceFrame, SurfacePaintPlan, UiSurface},
    widgets::{PointerButton, PointerModifiers, Widget, WidgetInput, WidgetOutput},
};
use std::{collections::HashMap, fs, path::PathBuf};

use super::{
    native_app_state_with_temp_sample, native_runtime_for_tests, run_command_for_tests,
    write_sparse_test_wav_i16, write_test_wav_i16,
};

fn sample_hit_target(
    selected: bool,
    drag_active: bool,
    drag_source: bool,
    cached: bool,
) -> UiSurface<crate::native_app::test_support::state::GuiMessage> {
    crate::native_app::test_support::sample_browser::sample_file_hit_target(
        String::from("sample.wav"),
        selected,
        drag_active,
        drag_source,
        cached,
    )
}

fn sample_hit_target_input(
    target: &mut UiSurface<crate::native_app::test_support::state::GuiMessage>,
    bounds: Rect,
    input: WidgetInput,
) -> Option<WidgetOutput> {
    target.dispatch_widget_input(
        crate::native_app::test_support::sample_browser::SAMPLE_FILE_HIT_TARGET_TEST_ID,
        bounds,
        input,
    )
}

fn sample_hit_target_message(
    target: &UiSurface<crate::native_app::test_support::state::GuiMessage>,
    output: WidgetOutput,
) -> Option<crate::native_app::test_support::state::GuiMessage> {
    target.dispatch_widget_output(
        crate::native_app::test_support::sample_browser::SAMPLE_FILE_HIT_TARGET_TEST_ID,
        output,
    )
}

fn sample_hit_target_plan(
    target: &UiSurface<crate::native_app::test_support::state::GuiMessage>,
    bounds: Rect,
) -> SurfacePaintPlan {
    target
        .find_widget(
            crate::native_app::test_support::sample_browser::SAMPLE_FILE_HIT_TARGET_TEST_ID,
        )
        .expect("sample hit-target widget")
        .widget()
        .paint_plan_with_defaults(bounds)
}

fn sample_hit_target_widget(
    target: &UiSurface<crate::native_app::test_support::state::GuiMessage>,
) -> &dyn Widget {
    target
        .find_widget(
            crate::native_app::test_support::sample_browser::SAMPLE_FILE_HIT_TARGET_TEST_ID,
        )
        .expect("sample hit-target widget")
        .widget()
}

fn sync_sample_hit_target_from_previous(
    current: &mut UiSurface<crate::native_app::test_support::state::GuiMessage>,
    previous: &UiSurface<crate::native_app::test_support::state::GuiMessage>,
) {
    let previous = previous
        .find_widget(
            crate::native_app::test_support::sample_browser::SAMPLE_FILE_HIT_TARGET_TEST_ID,
        )
        .expect("previous sample hit-target widget")
        .widget();
    let current = current
        .find_widget_mut(
            crate::native_app::test_support::sample_browser::SAMPLE_FILE_HIT_TARGET_TEST_ID,
        )
        .expect("current sample hit-target widget")
        .widget_mut();
    current.synchronize_from_previous(previous);
}

fn folder_drop_target_fill() -> Rgba8 {
    dense_row_palette_from_style(
        &ThemeTokens::default(),
        WidgetStyle::subtle(WidgetTone::Accent),
    )
    .active_target
    .expect("sidebar dense-row active target fill")
}

fn text_center(frame: &SurfaceFrame, label: &str) -> Point {
    frame
        .paint_plan
        .text_runs()
        .find(|text| text.text.as_str() == label)
        .map(|text| text.rect.center())
        .unwrap_or_else(|| panic!("{label} should paint"))
}

fn prepare_sample_browser_view(state: &mut crate::native_app::test_support::state::NativeAppState) {
    crate::native_app::test_support::sample_browser::prepare_sample_browser_view(state);
}

fn last_fixed_sample_browser_row_scroll(
    command: Command<crate::native_app::test_support::state::GuiMessage>,
) -> Option<(usize, i32)> {
    match command {
        Command::Batch(commands) => commands
            .into_iter()
            .filter_map(last_fixed_sample_browser_row_scroll)
            .last(),
        Command::ScrollFixedRowIntoView {
            node_id,
            row_index,
            direction,
            ..
        } if node_id == crate::native_app::sample_library::sample_list::SAMPLE_BROWSER_LIST_ID => {
            Some((row_index, direction))
        }
        _ => None,
    }
}

mod column_headers;
mod column_reorder;
mod drag_drop;
mod row_activation;
mod rows;
mod similarity;

#[test]
fn recording_sample_last_played_updates_row_and_persists_source_history() {
    let source_root = tempfile::tempdir().expect("source root");
    let sample_path = source_root.path().join("played.wav");
    fs::write(&sample_path, []).expect("write sample");
    let sample_path_string = sample_path.display().to_string();
    let db = wavecrate::sample_sources::SourceDatabase::open(source_root.path()).expect("db");
    db.upsert_file(std::path::Path::new("played.wav"), 0, 1)
        .expect("upsert sample");

    let mut state = crate::native_app::test_support::state::NativeAppStateFixture::default()
        .with_folder_browser(
            crate::native_app::test_support::state::FolderBrowserState::from_sample_sources(&[
                wavecrate::sample_sources::SampleSource::new(source_root.path().to_path_buf()),
            ]),
        )
        .build();
    state
        .library
        .folder_browser
        .select_file(sample_path_string.clone());
    let projection_cache_before = state
        .library
        .folder_browser
        .selected_audio_projection_cache_len_for_tests();
    assert_eq!(
        projection_cache_before, 1,
        "selecting the visible file should lazily warm only the selected projection"
    );
    let mut context = radiant::prelude::UiUpdateContext::default();

    state.record_sample_last_played(sample_path_string.clone(), &mut context);

    assert_eq!(
        state
            .library
            .folder_browser
            .selected_audio_projection_cache_len_for_tests(),
        projection_cache_before,
        "last-played metadata should not invalidate navigation projections unless it changes ordering"
    );
    assert_eq!(
        state
            .library
            .folder_browser
            .selected_audio_files()
            .into_iter()
            .find(|file| file.id == sample_path_string)
            .map(|file| file.modified.as_str()),
        Some("Today")
    );

    let delayed = run_first_after(context.into_command()).expect("last played delayed command");
    let mut context = radiant::prelude::UiUpdateContext::default();
    state.apply_message(delayed, &mut context);
    let message = run_first_perform(context.into_command()).expect("last played persist command");
    state.apply_message(message, &mut radiant::prelude::UiUpdateContext::default());

    assert!(
        db.last_played_at_for_path(std::path::Path::new("played.wav"))
            .expect("read last played")
            .is_some()
    );
}

#[test]
fn last_played_sort_invalidates_projection_when_history_changes() {
    let source_root = tempfile::tempdir().expect("source root");
    let first_path = source_root.path().join("first.wav");
    let second_path = source_root.path().join("second.wav");
    fs::write(&first_path, []).expect("write first sample");
    fs::write(&second_path, []).expect("write second sample");
    let first_path_string = first_path.display().to_string();

    let mut state = crate::native_app::test_support::state::NativeAppStateFixture::default()
        .with_folder_browser(
            crate::native_app::test_support::state::FolderBrowserState::from_sample_sources(&[
                wavecrate::sample_sources::SampleSource::new(source_root.path().to_path_buf()),
            ]),
        )
        .build();
    state
        .library
        .folder_browser
        .sort_file_column(String::from("modified"));
    let _ = state.library.folder_browser.selected_audio_files();
    assert!(
        state
            .library
            .folder_browser
            .selected_audio_projection_cache_len_for_tests()
            > 0,
        "history-sorted projection should be cached before playback"
    );

    state.record_sample_last_played(
        first_path_string,
        &mut radiant::prelude::UiUpdateContext::default(),
    );

    assert_eq!(
        state
            .library
            .folder_browser
            .selected_audio_projection_cache_len_for_tests(),
        0,
        "history-sorted projection must invalidate when last-played metadata changes ordering"
    );
}

#[test]
fn selecting_missing_sample_prunes_row_without_queueing_load() {
    let (mut state, source_root, selected_file) = native_app_state_with_temp_sample("missing.wav");
    fs::remove_file(source_root.path().join("missing.wav")).expect("remove sample");
    let mut context = radiant::prelude::UiUpdateContext::default();

    state.apply_message(
        crate::native_app::test_support::state::GuiMessage::SelectSampleWithModifiers {
            path: selected_file.clone(),
            modifiers: PointerModifiers::default(),
        },
        &mut context,
    );
    run_command_for_tests(&mut state, context.into_command());

    assert!(
        state
            .library
            .folder_browser
            .selected_audio_files()
            .is_empty(),
        "selecting a missing listed sample should remove it from the visible file list"
    );
    assert_eq!(state.library.folder_browser.selected_file_id(), None);
    assert!(
        state
            .background
            .deferred_sample_load_task
            .active()
            .is_none()
            && state.active_sample_load_task().is_none(),
        "missing sample selection should not queue a foreground load after validation"
    );
    assert!(state.ui.status.sample.contains("Removed missing"));
}

#[test]
fn map_mode_keyboard_navigation_centers_newly_selected_sample_node() {
    let source_root = tempfile::tempdir().expect("source root");
    let first = source_root.path().join("a.wav");
    let second = source_root.path().join("b.wav");
    write_test_wav_i16(&first, &[0, 100, -100]);
    write_test_wav_i16(&second, &[0, 120, -120]);
    let first_id = first.display().to_string();
    let second_id = second.display().to_string();
    let mut state = crate::native_app::test_support::state::NativeAppStateFixture::default()
        .with_folder_browser(
            crate::native_app::test_support::state::FolderBrowserState::from_sample_sources(&[
                wavecrate::sample_sources::SampleSource::new(source_root.path().to_path_buf()),
            ]),
        )
        .build();
    state.library.folder_browser.select_file(first_id.clone());
    state.ui.chrome.sample_browser_display = crate::native_app::app::SampleBrowserDisplayMode::Map;
    state.ui.chrome.starmap_viewport.zoom = 4.0;
    state
        .library
        .folder_browser
        .prepare_starmap_layout(&state.metadata.tags_by_file);
    let layout_request = state
        .library
        .folder_browser
        .take_starmap_layout_load_request(&state.metadata.tags_by_file)
        .expect("starmap layout request");
    state
        .library
        .folder_browser
        .apply_starmap_layout_load_result(wavecrate::sample_sources::StarmapLayoutLoadResult {
            signature: layout_request.signature,
            result: Ok(HashMap::from([
                (
                    first_id,
                    wavecrate::sample_sources::StarmapLayoutPoint {
                        x: 0.50,
                        y: 0.35,
                        cluster_id: None,
                    },
                ),
                (
                    second_id.clone(),
                    wavecrate::sample_sources::StarmapLayoutPoint {
                        x: 0.50,
                        y: 0.70,
                        cluster_id: None,
                    },
                ),
            ])),
        });

    state.apply_message(
        crate::native_app::test_support::state::GuiMessage::NavigateBrowser {
            delta: 1,
            extend: false,
            preserve_selection: false,
        },
        &mut radiant::prelude::UiUpdateContext::default(),
    );

    assert_eq!(
        state.library.folder_browser.selected_file_id(),
        Some(second_id.as_str())
    );
    let expected = state
        .library
        .folder_browser
        .selected_starmap_position(
            crate::native_app::sample_library::folder_browser::starmap::StarmapProjection {
                tags_by_file: &state.metadata.tags_by_file,
                instant_audition_sample_paths: &state.waveform.cache.instant_audition_sample_paths,
            },
        )
        .expect("selected sample should have a map position");
    assert_starmap_viewport_reveals(
        state.ui.chrome.starmap_viewport,
        expected,
        "map viewport should reveal the selected node after keyboard navigation",
    );
}

#[test]
fn leaving_starmap_mode_reveals_selected_list_row() {
    let source_root = tempfile::tempdir().expect("source root");
    let first = source_root.path().join("a.wav");
    let second = source_root.path().join("b.wav");
    let third = source_root.path().join("c.wav");
    write_test_wav_i16(&first, &[0, 100, -100]);
    write_test_wav_i16(&second, &[0, 120, -120]);
    write_test_wav_i16(&third, &[0, 140, -140]);
    let third_id = third.display().to_string();
    let mut state = crate::native_app::test_support::state::NativeAppStateFixture::default()
        .with_folder_browser(
            crate::native_app::test_support::state::FolderBrowserState::from_sample_sources(&[
                wavecrate::sample_sources::SampleSource::new(source_root.path().to_path_buf()),
            ]),
        )
        .build();
    state.library.folder_browser.select_file(third_id.clone());
    state.ui.chrome.sample_browser_display = crate::native_app::app::SampleBrowserDisplayMode::Map;
    let mut context = radiant::prelude::UiUpdateContext::default();

    state.apply_message(
        crate::native_app::test_support::state::GuiMessage::ToggleSampleBrowserMapView,
        &mut context,
    );

    assert_eq!(
        state.ui.chrome.sample_browser_display,
        crate::native_app::app::SampleBrowserDisplayMode::List
    );
    assert_eq!(
        state.library.folder_browser.selected_file_id(),
        Some(third_id.as_str())
    );
    assert_eq!(
        last_fixed_sample_browser_row_scroll(context.into_command()),
        Some((2, 0)),
        "switching back to list mode should reveal the selected row"
    );
}

#[test]
fn leaving_starmap_mode_materializes_selected_list_row_immediately() {
    let source_root = tempfile::tempdir().expect("source root");
    let files = (0..120)
        .map(|index| source_root.path().join(format!("sample_{index:03}.wav")))
        .collect::<Vec<_>>();
    for file in &files {
        write_test_wav_i16(file, &[0, 100, -100]);
    }
    let selected_index = 96;
    let selected_id = files[selected_index].display().to_string();
    let mut state = crate::native_app::test_support::state::NativeAppStateFixture::default()
        .with_folder_browser(
            crate::native_app::test_support::state::FolderBrowserState::from_sample_sources(&[
                wavecrate::sample_sources::SampleSource::new(source_root.path().to_path_buf()),
            ]),
        )
        .build();
    state
        .library
        .folder_browser
        .select_file(selected_id.clone());
    state.ui.chrome.sample_browser_display = crate::native_app::app::SampleBrowserDisplayMode::Map;
    let mut context = radiant::prelude::UiUpdateContext::default();

    state.apply_message(
        crate::native_app::test_support::state::GuiMessage::ToggleSampleBrowserMapView,
        &mut context,
    );

    assert_eq!(
        state.ui.chrome.sample_browser_display,
        crate::native_app::app::SampleBrowserDisplayMode::List
    );
    assert_eq!(
        state.library.folder_browser.selected_file_id(),
        Some(selected_id.as_str())
    );
    assert_eq!(
        last_fixed_sample_browser_row_scroll(context.into_command()),
        Some((selected_index, 0))
    );
    let window_start = state.library.folder_browser.file_view_start();
    assert!(
        window_start > 0 && window_start <= selected_index,
        "list window should be prepared around the selected sample immediately, start={}",
        window_start
    );
}

#[test]
fn map_audition_selection_is_revealed_when_returning_to_list_mode() {
    let source_root = tempfile::tempdir().expect("source root");
    let first = source_root.path().join("a.wav");
    let second = source_root.path().join("b.wav");
    let third = source_root.path().join("c.wav");
    write_test_wav_i16(&first, &[0, 100, -100]);
    write_test_wav_i16(&second, &[0, 120, -120]);
    write_test_wav_i16(&third, &[0, 140, -140]);
    let third_id = third.display().to_string();
    let mut state = crate::native_app::test_support::state::NativeAppStateFixture::default()
        .with_folder_browser(
            crate::native_app::test_support::state::FolderBrowserState::from_sample_sources(&[
                wavecrate::sample_sources::SampleSource::new(source_root.path().to_path_buf()),
            ]),
        )
        .build();
    state.ui.chrome.sample_browser_display = crate::native_app::app::SampleBrowserDisplayMode::Map;

    state.apply_message(
        crate::native_app::test_support::state::GuiMessage::BeginStarmapAuditionDrag {
            path: Some(third_id.clone()),
            position: Point::new(10.0, 10.0),
            modifiers: PointerModifiers::default(),
        },
        &mut radiant::prelude::UiUpdateContext::default(),
    );
    let mut context = radiant::prelude::UiUpdateContext::default();
    state.apply_message(
        crate::native_app::test_support::state::GuiMessage::ToggleSampleBrowserMapView,
        &mut context,
    );

    assert_eq!(
        state.ui.chrome.sample_browser_display,
        crate::native_app::app::SampleBrowserDisplayMode::List
    );
    assert_eq!(
        state.library.folder_browser.selected_file_id(),
        Some(third_id.as_str())
    );
    assert_eq!(
        last_fixed_sample_browser_row_scroll(context.into_command()),
        Some((2, 0)),
        "list mode should reveal the node selected by starmap audition"
    );
}

#[test]
fn copying_sample_selected_from_map_flashes_map_node_and_waveform() {
    let source_root = tempfile::tempdir().expect("source root");
    let first = source_root.path().join("a.wav");
    let second = source_root.path().join("b.wav");
    write_test_wav_i16(&first, &[0, 100, -100]);
    write_test_wav_i16(&second, &[0, 120, -120]);
    let first_id = first.display().to_string();
    let second_id = second.display().to_string();
    let mut state = crate::native_app::test_support::state::NativeAppStateFixture::default()
        .with_folder_browser(
            crate::native_app::test_support::state::FolderBrowserState::from_sample_sources(&[
                wavecrate::sample_sources::SampleSource::new(source_root.path().to_path_buf()),
            ]),
        )
        .build();
    state.library.folder_browser.select_file(first_id);
    state.ui.chrome.sample_browser_display = crate::native_app::app::SampleBrowserDisplayMode::Map;

    state.apply_message(
        crate::native_app::test_support::state::GuiMessage::BeginStarmapAuditionDrag {
            path: Some(second_id.clone()),
            position: Point::new(10.0, 10.0),
            modifiers: PointerModifiers::default(),
        },
        &mut radiant::prelude::UiUpdateContext::default(),
    );
    state.copy_selected_files(&mut radiant::prelude::UiUpdateContext::default());

    assert_eq!(
        state.library.folder_browser.selected_file_id(),
        Some(second_id.as_str()),
        "copy feedback must not disturb map-driven selection"
    );
    assert!(
        state.waveform.current.copy_flash_frames() > 0,
        "copying the map-selected sample should flash the waveform"
    );
    let items = state.library.folder_browser.starmap_projection(
        crate::native_app::sample_library::folder_browser::starmap::StarmapProjection {
            tags_by_file: &state.metadata.tags_by_file,
            instant_audition_sample_paths: &state.waveform.cache.instant_audition_sample_paths,
        },
    );
    assert!(
        items
            .iter()
            .any(|item| item.file_id == second_id && item.selected && item.copy_flash),
        "copying the map-selected sample should flash the selected map node"
    );
}

#[test]
fn starmap_drag_sweep_retargets_to_latest_hit_without_backlog() {
    let source_root = tempfile::tempdir().expect("source root");
    let first = source_root.path().join("a.wav");
    let second = source_root.path().join("b.wav");
    let third = source_root.path().join("c.wav");
    write_test_wav_i16(&first, &[0, 100, -100]);
    write_test_wav_i16(&second, &[0, 120, -120]);
    write_test_wav_i16(&third, &[0, 140, -140]);
    let first_id = first.display().to_string();
    let second_id = second.display().to_string();
    let third_id = third.display().to_string();
    let mut state = crate::native_app::test_support::state::NativeAppStateFixture::default()
        .with_folder_browser(
            crate::native_app::test_support::state::FolderBrowserState::from_sample_sources(&[
                wavecrate::sample_sources::SampleSource::new(source_root.path().to_path_buf()),
            ]),
        )
        .build();
    let mut context = radiant::prelude::UiUpdateContext::default();

    state.apply_message(
        crate::native_app::test_support::state::GuiMessage::BeginStarmapAuditionDrag {
            path: Some(first_id.clone()),
            position: Point::new(10.0, 10.0),
            modifiers: PointerModifiers::default(),
        },
        &mut context,
    );
    state.apply_message(
        crate::native_app::test_support::state::GuiMessage::UpdateStarmapAuditionDrag {
            paths: vec![second_id.clone(), third_id.clone()],
            position: Point::new(90.0, 10.0),
            modifiers: PointerModifiers::default(),
        },
        &mut context,
    );

    assert_eq!(
        state.library.folder_browser.selected_file_id(),
        Some(third_id.as_str()),
        "the latest crossed drag hit should become the immediate playback target"
    );
    assert_eq!(
        state
            .ui
            .chrome
            .starmap_audition_queue
            .active_file_id
            .as_deref(),
        Some(third_id.as_str())
    );
    assert_eq!(
        state
            .ui
            .chrome
            .starmap_audition_queue
            .queued_file_ids
            .iter()
            .cloned()
            .collect::<Vec<_>>(),
        Vec::<String>::new(),
        "drag sweeps should not create a delayed playback backlog"
    );
}

#[test]
fn starmap_drag_ready_descriptor_skips_foreground_sample_load_validation() {
    let source_root = tempfile::tempdir().expect("source root");
    let sample = source_root.path().join("large-ready.wav");
    write_sparse_test_wav_i16(&sample, 1, 700);
    let sample_id = sample.display().to_string();
    let cache_file = crate::native_app::waveform::PersistedPlaybackCacheFile::new(
        sample.with_extension("f32"),
        700,
    )
    .expect("playback cache file");
    let descriptor = crate::native_app::waveform::PersistedPlaybackDescriptor::new(
        sample.clone(),
        cache_file,
        48_000,
        1,
        700,
    )
    .expect("persisted playback descriptor");
    let mut state = crate::native_app::test_support::state::NativeAppStateFixture::default()
        .with_folder_browser(
            crate::native_app::test_support::state::FolderBrowserState::from_sample_sources(&[
                wavecrate::sample_sources::SampleSource::new(source_root.path().to_path_buf()),
            ]),
        )
        .build();
    state
        .waveform
        .cache
        .mark_sample_playback_descriptor_ready(descriptor);
    let mut context = radiant::prelude::UiUpdateContext::default();

    state.apply_message(
        crate::native_app::test_support::state::GuiMessage::BeginStarmapAuditionDrag {
            path: Some(sample_id.clone()),
            position: Point::new(10.0, 10.0),
            modifiers: PointerModifiers::default(),
        },
        &mut context,
    );
    let command = context.into_command();

    assert_eq!(
        state.library.folder_browser.selected_file_id(),
        Some(sample_id.as_str())
    );
    assert_eq!(
        state
            .ui
            .chrome
            .starmap_audition_queue
            .active_file_id
            .as_deref(),
        Some(sample_id.as_str())
    );
    assert_eq!(
        command.business_task_priority("gui-sample-load-validate"),
        None,
        "playback-ready starmap drag targets should not enter the foreground sample-load path"
    );
}

#[test]
fn starmap_drag_cold_wav_queues_preview_audition_not_sample_load_validation() {
    let source_root = tempfile::tempdir().expect("source root");
    let sample = source_root.path().join("large-cold.wav");
    write_sparse_test_wav_i16(&sample, 1, 700);
    let sample_id = sample.display().to_string();
    let mut state = crate::native_app::test_support::state::NativeAppStateFixture::default()
        .with_folder_browser(
            crate::native_app::test_support::state::FolderBrowserState::from_sample_sources(&[
                wavecrate::sample_sources::SampleSource::new(source_root.path().to_path_buf()),
            ]),
        )
        .build();
    let mut context = radiant::prelude::UiUpdateContext::default();

    state.apply_message(
        crate::native_app::test_support::state::GuiMessage::BeginStarmapAuditionDrag {
            path: Some(sample_id.clone()),
            position: Point::new(10.0, 10.0),
            modifiers: PointerModifiers::default(),
        },
        &mut context,
    );
    let command = context.into_command();

    assert_eq!(
        state.library.folder_browser.selected_file_id(),
        Some(sample_id.as_str())
    );
    assert_eq!(
        command.business_task_priority("gui-sample-load-validate"),
        None,
        "cold starmap drag audition should not need the normal foreground load path"
    );
    assert_eq!(
        command.business_task_priority("gui-preview-audition-decode"),
        Some(radiant::prelude::TaskPriority::Interactive),
        "cold starmap drag WAV targets should decode only a tiny preview head"
    );
}

#[test]
fn starmap_mode_frame_warms_preview_audition_heads() {
    let source_root = tempfile::tempdir().expect("source root");
    let sample = source_root.path().join("large-cold.wav");
    write_sparse_test_wav_i16(&sample, 1, 700);
    let mut state = crate::native_app::test_support::state::NativeAppStateFixture::default()
        .with_folder_browser(
            crate::native_app::test_support::state::FolderBrowserState::from_sample_sources(&[
                wavecrate::sample_sources::SampleSource::new(source_root.path().to_path_buf()),
            ]),
        )
        .build();
    state.ui.chrome.sample_browser_display =
        crate::native_app::app::SampleBrowserDisplayMode::Map;
    prepare_sample_browser_view(&mut state);
    let mut context = radiant::prelude::UiUpdateContext::default();

    state.apply_message(
        crate::native_app::test_support::state::GuiMessage::Frame,
        &mut context,
    );
    let command = context.into_command();

    assert_eq!(
        command.business_task_priority("gui-preview-audition-warm"),
        Some(radiant::prelude::TaskPriority::Background),
        "starmap mode should warm tiny preview heads before drag playback needs them"
    );
    assert!(
        state.background.preview_audition_warm_task.active().is_some(),
        "preview audition warm should be tracked as cancellable background work"
    );
}

#[test]
fn starmap_mode_frame_does_not_duplicate_scheduled_preview_audition_heads() {
    let source_root = tempfile::tempdir().expect("source root");
    let sample = source_root.path().join("large-cold.wav");
    write_sparse_test_wav_i16(&sample, 1, 700);
    let sample_id = sample.display().to_string();
    let mut state = crate::native_app::test_support::state::NativeAppStateFixture::default()
        .with_folder_browser(
            crate::native_app::test_support::state::FolderBrowserState::from_sample_sources(&[
                wavecrate::sample_sources::SampleSource::new(source_root.path().to_path_buf()),
            ]),
        )
        .build();
    state.ui.chrome.sample_browser_display =
        crate::native_app::app::SampleBrowserDisplayMode::Map;
    prepare_sample_browser_view(&mut state);
    state
        .waveform
        .cache
        .mark_preview_audition_warm_scheduled(std::slice::from_ref(&sample_id));
    let mut context = radiant::prelude::UiUpdateContext::default();

    state.apply_message(
        crate::native_app::test_support::state::GuiMessage::Frame,
        &mut context,
    );
    let command = context.into_command();

    assert_eq!(
        command.business_task_priority("gui-preview-audition-warm"),
        None,
        "preview warming should not rediscover a path already queued by a prior frame"
    );
}

#[test]
fn starmap_mode_frame_does_not_warm_preview_audition_heads_while_playback_active() {
    let source_root = tempfile::tempdir().expect("source root");
    let sample = source_root.path().join("large-cold.wav");
    write_sparse_test_wav_i16(&sample, 1, 700);
    let sample_id = sample.display().to_string();
    let mut state = crate::native_app::test_support::state::NativeAppStateFixture::default()
        .with_folder_browser(
            crate::native_app::test_support::state::FolderBrowserState::from_sample_sources(&[
                wavecrate::sample_sources::SampleSource::new(source_root.path().to_path_buf()),
            ]),
        )
        .build();
    state.ui.chrome.sample_browser_display =
        crate::native_app::app::SampleBrowserDisplayMode::Map;
    state.audio.early_sample_playback_path = Some(sample_id);
    prepare_sample_browser_view(&mut state);
    let mut context = radiant::prelude::UiUpdateContext::default();

    state.apply_message(
        crate::native_app::test_support::state::GuiMessage::Frame,
        &mut context,
    );
    let command = context.into_command();

    assert_eq!(
        command.business_task_priority("gui-preview-audition-warm"),
        None,
        "preview warming should yield while playback is active"
    );
    assert_eq!(
        state.background.preview_audition_warm_task.active(),
        None,
        "preview warming should not leave a tracked background task while playback is active"
    );
}

#[test]
fn starmap_drag_finish_cancels_cold_preview_audition_decode() {
    let source_root = tempfile::tempdir().expect("source root");
    let sample = source_root.path().join("large-cold.wav");
    write_sparse_test_wav_i16(&sample, 1, 700);
    let sample_id = sample.display().to_string();
    let mut state = crate::native_app::test_support::state::NativeAppStateFixture::default()
        .with_folder_browser(
            crate::native_app::test_support::state::FolderBrowserState::from_sample_sources(&[
                wavecrate::sample_sources::SampleSource::new(source_root.path().to_path_buf()),
            ]),
        )
        .build();
    let mut context = radiant::prelude::UiUpdateContext::default();

    state.apply_message(
        crate::native_app::test_support::state::GuiMessage::BeginStarmapAuditionDrag {
            path: Some(sample_id),
            position: Point::new(10.0, 10.0),
            modifiers: PointerModifiers::default(),
        },
        &mut context,
    );
    assert!(
        state.background.preview_audition_task.active().is_some(),
        "cold drag targets should start preview decode while the drag is active"
    );

    state.apply_message(
        crate::native_app::test_support::state::GuiMessage::FinishStarmapAuditionDrag,
        &mut context,
    );

    assert_eq!(state.background.preview_audition_task.active(), None);
    assert_eq!(state.ui.chrome.starmap_audition_queue, Default::default());
}

#[test]
fn starmap_audition_promotion_only_loads_latest_stable_target() {
    let source_root = tempfile::tempdir().expect("source root");
    let first = source_root.path().join("first.wav");
    let second = source_root.path().join("second.wav");
    write_sparse_test_wav_i16(&first, 1, 700);
    write_sparse_test_wav_i16(&second, 1, 700);
    let first_id = first.display().to_string();
    let second_id = second.display().to_string();
    let mut state = crate::native_app::test_support::state::NativeAppStateFixture::default()
        .with_folder_browser(
            crate::native_app::test_support::state::FolderBrowserState::from_sample_sources(&[
                wavecrate::sample_sources::SampleSource::new(source_root.path().to_path_buf()),
            ]),
        )
        .build();
    state
        .library
        .folder_browser
        .select_file(second_id.clone());
    let mut context = radiant::prelude::UiUpdateContext::default();

    state.schedule_starmap_audition_promotion(first_id, &mut context);
    state.schedule_starmap_audition_promotion(second_id.clone(), &mut context);
    let delayed = run_after_commands(context.into_command());
    assert_eq!(delayed.len(), 2);

    let mut stale_context = radiant::prelude::UiUpdateContext::default();
    state.apply_message(delayed[0].clone(), &mut stale_context);
    assert_eq!(
        stale_context
            .into_command()
            .business_task_priority("gui-sample-load-validate"),
        None,
        "stale starmap promotion tickets must not start full sample loads"
    );

    let mut latest_context = radiant::prelude::UiUpdateContext::default();
    state.apply_message(delayed[1].clone(), &mut latest_context);
    assert_eq!(
        latest_context
            .into_command()
            .business_task_priority("gui-sample-load-validate"),
        Some(radiant::prelude::TaskPriority::Interactive),
        "latest stable starmap target should promote to the normal full load path"
    );
}

#[test]
fn starmap_drag_retriggers_sample_after_sweeping_away_and_back() {
    let source_root = tempfile::tempdir().expect("source root");
    let first = source_root.path().join("a.wav");
    let second = source_root.path().join("b.wav");
    write_test_wav_i16(&first, &[0, 100, -100]);
    write_test_wav_i16(&second, &[0, 120, -120]);
    let first_id = first.display().to_string();
    let second_id = second.display().to_string();
    let mut state = crate::native_app::test_support::state::NativeAppStateFixture::default()
        .with_folder_browser(
            crate::native_app::test_support::state::FolderBrowserState::from_sample_sources(&[
                wavecrate::sample_sources::SampleSource::new(source_root.path().to_path_buf()),
            ]),
        )
        .build();
    let mut context = radiant::prelude::UiUpdateContext::default();

    state.apply_message(
        crate::native_app::test_support::state::GuiMessage::BeginStarmapAuditionDrag {
            path: Some(first_id.clone()),
            position: Point::new(10.0, 10.0),
            modifiers: PointerModifiers::default(),
        },
        &mut context,
    );
    state.apply_message(
        crate::native_app::test_support::state::GuiMessage::UpdateStarmapAuditionDrag {
            paths: vec![second_id.clone()],
            position: Point::new(90.0, 10.0),
            modifiers: PointerModifiers::default(),
        },
        &mut context,
    );
    state.apply_message(
        crate::native_app::test_support::state::GuiMessage::UpdateStarmapAuditionDrag {
            paths: vec![first_id.clone()],
            position: Point::new(92.0, 10.0),
            modifiers: PointerModifiers::default(),
        },
        &mut context,
    );

    assert_eq!(
        state
            .ui
            .chrome
            .starmap_audition_queue
            .active_file_id
            .as_deref(),
        Some(first_id.as_str())
    );
    assert_eq!(
        state.library.folder_browser.selected_file_id(),
        Some(first_id.as_str()),
        "returning to a node should make it the immediate playback target again"
    );
    assert_eq!(
        state
            .ui
            .chrome
            .starmap_audition_queue
            .queued_file_ids
            .iter()
            .cloned()
            .collect::<Vec<_>>(),
        Vec::<String>::new(),
        "dragging away from a sample and back should not leave a delayed replay queued"
    );
}

#[test]
fn starmap_drag_finish_clears_active_drag_audition_state() {
    let source_root = tempfile::tempdir().expect("source root");
    let first = source_root.path().join("a.wav");
    let second = source_root.path().join("b.wav");
    let third = source_root.path().join("c.wav");
    write_test_wav_i16(&first, &[0, 100, -100]);
    write_test_wav_i16(&second, &[0, 120, -120]);
    write_test_wav_i16(&third, &[0, 140, -140]);
    let first_id = first.display().to_string();
    let second_id = second.display().to_string();
    let third_id = third.display().to_string();
    let mut state = crate::native_app::test_support::state::NativeAppStateFixture::default()
        .with_folder_browser(
            crate::native_app::test_support::state::FolderBrowserState::from_sample_sources(&[
                wavecrate::sample_sources::SampleSource::new(source_root.path().to_path_buf()),
            ]),
        )
        .build();
    let mut context = radiant::prelude::UiUpdateContext::default();

    state.apply_message(
        crate::native_app::test_support::state::GuiMessage::BeginStarmapAuditionDrag {
            path: Some(first_id.clone()),
            position: Point::new(10.0, 10.0),
            modifiers: PointerModifiers::default(),
        },
        &mut context,
    );
    state.apply_message(
        crate::native_app::test_support::state::GuiMessage::UpdateStarmapAuditionDrag {
            paths: vec![second_id.clone(), third_id.clone()],
            position: Point::new(90.0, 10.0),
            modifiers: PointerModifiers::default(),
        },
        &mut context,
    );

    assert_eq!(
        state
            .ui
            .chrome
            .starmap_audition_queue
            .active_file_id
            .as_deref(),
        Some(third_id.as_str())
    );
    assert_eq!(
        state
            .ui
            .chrome
            .starmap_audition_queue
            .queued_file_ids
            .iter()
            .cloned()
            .collect::<Vec<_>>(),
        Vec::<String>::new(),
        "drag updates should not leave swept follow-up targets queued behind the current pointer"
    );

    state.apply_message(
        crate::native_app::test_support::state::GuiMessage::FinishStarmapAuditionDrag,
        &mut context,
    );

    assert_eq!(state.ui.chrome.starmap_audition_drag, None);
    assert_eq!(
        state.ui.chrome.starmap_audition_queue,
        Default::default(),
        "releasing the drag should not leave active or queued audition state behind"
    );
}

#[test]
fn starmap_drag_update_after_finish_is_ignored() {
    let source_root = tempfile::tempdir().expect("source root");
    let first = source_root.path().join("a.wav");
    let second = source_root.path().join("b.wav");
    write_test_wav_i16(&first, &[0, 100, -100]);
    write_test_wav_i16(&second, &[0, 120, -120]);
    let first_id = first.display().to_string();
    let second_id = second.display().to_string();
    let mut state = crate::native_app::test_support::state::NativeAppStateFixture::default()
        .with_folder_browser(
            crate::native_app::test_support::state::FolderBrowserState::from_sample_sources(&[
                wavecrate::sample_sources::SampleSource::new(source_root.path().to_path_buf()),
            ]),
        )
        .build();
    let mut context = radiant::prelude::UiUpdateContext::default();

    state.apply_message(
        crate::native_app::test_support::state::GuiMessage::BeginStarmapAuditionDrag {
            path: Some(first_id.clone()),
            position: Point::new(10.0, 10.0),
            modifiers: PointerModifiers::default(),
        },
        &mut context,
    );
    state.apply_message(
        crate::native_app::test_support::state::GuiMessage::FinishStarmapAuditionDrag,
        &mut context,
    );
    state.apply_message(
        crate::native_app::test_support::state::GuiMessage::UpdateStarmapAuditionDrag {
            paths: vec![second_id],
            position: Point::new(90.0, 10.0),
            modifiers: PointerModifiers::default(),
        },
        &mut context,
    );

    assert_eq!(
        state.library.folder_browser.selected_file_id(),
        Some(first_id.as_str()),
        "late pointer updates after release should not restart drag-play audition"
    );
    assert_eq!(state.ui.chrome.starmap_audition_queue, Default::default());
}

#[test]
fn starmap_drag_audition_ignores_multi_select_modifiers() {
    let source_root = tempfile::tempdir().expect("source root");
    let first = source_root.path().join("a.wav");
    let second = source_root.path().join("b.wav");
    write_test_wav_i16(&first, &[0, 100, -100]);
    write_test_wav_i16(&second, &[0, 120, -120]);
    let first_id = first.display().to_string();
    let second_id = second.display().to_string();
    let mut state = crate::native_app::test_support::state::NativeAppStateFixture::default()
        .with_folder_browser(
            crate::native_app::test_support::state::FolderBrowserState::from_sample_sources(&[
                wavecrate::sample_sources::SampleSource::new(source_root.path().to_path_buf()),
            ]),
        )
        .build();
    let shift = PointerModifiers {
        shift: true,
        ..PointerModifiers::default()
    };
    state.library.folder_browser.select_file(first_id.clone());

    state.apply_message(
        crate::native_app::test_support::state::GuiMessage::BeginStarmapAuditionDrag {
            path: Some(second_id.clone()),
            position: Point::new(90.0, 10.0),
            modifiers: shift,
        },
        &mut radiant::prelude::UiUpdateContext::default(),
    );

    assert_eq!(
        state.library.folder_browser.selected_file_id(),
        Some(second_id.as_str())
    );
    assert_eq!(
        state.library.folder_browser.selected_audio_file_count(),
        1,
        "map audition should replace the focused sample instead of range-selecting swept nodes"
    );
}

#[test]
fn starmap_audition_hit_preserves_current_viewport() {
    let source_root = tempfile::tempdir().expect("source root");
    let first = source_root.path().join("a.wav");
    let second = source_root.path().join("b.wav");
    write_test_wav_i16(&first, &[0, 100, -100]);
    write_test_wav_i16(&second, &[0, 120, -120]);
    let second_id = second.display().to_string();
    let mut state = crate::native_app::test_support::state::NativeAppStateFixture::default()
        .with_folder_browser(
            crate::native_app::test_support::state::FolderBrowserState::from_sample_sources(&[
                wavecrate::sample_sources::SampleSource::new(source_root.path().to_path_buf()),
            ]),
        )
        .build();
    state.ui.chrome.sample_browser_display = crate::native_app::app::SampleBrowserDisplayMode::Map;
    state.ui.chrome.starmap_viewport.zoom = 4.0;
    state.ui.chrome.starmap_viewport.center_x = 0.1;
    state.ui.chrome.starmap_viewport.center_y = 0.9;
    let initial_viewport = state.ui.chrome.starmap_viewport;

    state.apply_message(
        crate::native_app::test_support::state::GuiMessage::BeginStarmapAuditionDrag {
            path: Some(second_id.clone()),
            position: Point::new(10.0, 10.0),
            modifiers: PointerModifiers::default(),
        },
        &mut radiant::prelude::UiUpdateContext::default(),
    );

    assert_eq!(
        state.library.folder_browser.selected_file_id(),
        Some(second_id.as_str())
    );
    assert_eq!(
        state.ui.chrome.starmap_viewport, initial_viewport,
        "map audition should select/play without panning the viewport",
    );
}

#[test]
fn starmap_drag_update_selects_next_hit_immediately() {
    let source_root = tempfile::tempdir().expect("source root");
    let first = source_root.path().join("a.wav");
    let second = source_root.path().join("b.wav");
    write_test_wav_i16(&first, &[0, 100, -100]);
    write_test_wav_i16(&second, &[0, 120, -120]);
    let first_id = first.display().to_string();
    let second_id = second.display().to_string();
    let mut state = crate::native_app::test_support::state::NativeAppStateFixture::default()
        .with_folder_browser(
            crate::native_app::test_support::state::FolderBrowserState::from_sample_sources(&[
                wavecrate::sample_sources::SampleSource::new(source_root.path().to_path_buf()),
            ]),
        )
        .build();
    state.ui.chrome.sample_browser_display = crate::native_app::app::SampleBrowserDisplayMode::Map;
    state.ui.chrome.starmap_viewport.zoom = 4.0;
    state.ui.chrome.starmap_viewport.center_x = 0.12;
    state.ui.chrome.starmap_viewport.center_y = 0.88;
    let initial_viewport = state.ui.chrome.starmap_viewport;
    let mut context = radiant::prelude::UiUpdateContext::default();
    state.apply_message(
        crate::native_app::test_support::state::GuiMessage::BeginStarmapAuditionDrag {
            path: Some(first_id.clone()),
            position: Point::new(10.0, 10.0),
            modifiers: PointerModifiers::default(),
        },
        &mut context,
    );
    state.apply_message(
        crate::native_app::test_support::state::GuiMessage::UpdateStarmapAuditionDrag {
            paths: vec![second_id.clone()],
            position: Point::new(90.0, 10.0),
            modifiers: PointerModifiers::default(),
        },
        &mut context,
    );

    assert_eq!(
        state.library.folder_browser.selected_file_id(),
        Some(second_id.as_str())
    );
    assert_eq!(
        state.ui.chrome.starmap_viewport, initial_viewport,
        "queued drag-play audition should not pan to the selected node"
    );
    assert_eq!(
        state
            .ui
            .chrome
            .starmap_audition_queue
            .active_file_id
            .as_deref(),
        Some(second_id.as_str())
    );
    assert!(
        state
            .ui
            .chrome
            .starmap_audition_queue
            .queued_file_ids
            .is_empty()
    );
}

#[test]
fn starmap_drag_replacement_ignores_stale_advance_ticket() {
    let source_root = tempfile::tempdir().expect("source root");
    let first = source_root.path().join("a.wav");
    let second = source_root.path().join("b.wav");
    write_test_wav_i16(&first, &[0, 100, -100]);
    write_test_wav_i16(&second, &[0, 120, -120]);
    let first_id = first.display().to_string();
    let second_id = second.display().to_string();
    let mut state = crate::native_app::test_support::state::NativeAppStateFixture::default()
        .with_folder_browser(
            crate::native_app::test_support::state::FolderBrowserState::from_sample_sources(&[
                wavecrate::sample_sources::SampleSource::new(source_root.path().to_path_buf()),
            ]),
        )
        .build();
    let mut context = radiant::prelude::UiUpdateContext::default();

    state.apply_message(
        crate::native_app::test_support::state::GuiMessage::BeginStarmapAuditionDrag {
            path: Some(first_id),
            position: Point::new(10.0, 10.0),
            modifiers: PointerModifiers::default(),
        },
        &mut context,
    );
    let stale_advance = state.background.starmap_audition_advance_task.begin();
    state.apply_message(
        crate::native_app::test_support::state::GuiMessage::UpdateStarmapAuditionDrag {
            paths: vec![second_id.clone()],
            position: Point::new(90.0, 10.0),
            modifiers: PointerModifiers::default(),
        },
        &mut context,
    );
    state.apply_message(
        crate::native_app::test_support::state::GuiMessage::AdvanceStarmapAudition {
            ticket: stale_advance,
        },
        &mut context,
    );

    assert_eq!(
        state.library.folder_browser.selected_file_id(),
        Some(second_id.as_str()),
        "stale delayed advances must not undo the latest drag target"
    );
    assert_eq!(
        state
            .ui
            .chrome
            .starmap_audition_queue
            .active_file_id
            .as_deref(),
        Some(second_id.as_str())
    );
}

#[test]
fn starmap_audition_queue_clears_after_last_hit_finishes() {
    let source_root = tempfile::tempdir().expect("source root");
    let first = source_root.path().join("a.wav");
    write_test_wav_i16(&first, &[0, 100, -100]);
    let first_id = first.display().to_string();
    let mut state = crate::native_app::test_support::state::NativeAppStateFixture::default()
        .with_folder_browser(
            crate::native_app::test_support::state::FolderBrowserState::from_sample_sources(&[
                wavecrate::sample_sources::SampleSource::new(source_root.path().to_path_buf()),
            ]),
        )
        .build();
    state.apply_message(
        crate::native_app::test_support::state::GuiMessage::BeginStarmapAuditionDrag {
            path: Some(first_id),
            position: Point::new(10.0, 10.0),
            modifiers: PointerModifiers::default(),
        },
        &mut radiant::prelude::UiUpdateContext::default(),
    );
    state.apply_message(
        crate::native_app::test_support::state::GuiMessage::FinishStarmapAuditionDrag,
        &mut radiant::prelude::UiUpdateContext::default(),
    );

    state.schedule_next_starmap_audition_hit(&mut radiant::prelude::UiUpdateContext::default());

    assert_eq!(state.ui.chrome.starmap_audition_drag, None);
    assert_eq!(state.ui.chrome.starmap_audition_queue, Default::default());
}

#[test]
fn rapid_last_played_records_only_latest_delayed_persist() {
    let source_root = tempfile::tempdir().expect("source root");
    let first_path = source_root.path().join("first.wav");
    let second_path = source_root.path().join("second.wav");
    fs::write(&first_path, []).expect("write first sample");
    fs::write(&second_path, []).expect("write second sample");
    let first_path_string = first_path.display().to_string();
    let second_path_string = second_path.display().to_string();

    let mut state = crate::native_app::test_support::state::NativeAppStateFixture::default()
        .with_folder_browser(
            crate::native_app::test_support::state::FolderBrowserState::from_sample_sources(&[
                wavecrate::sample_sources::SampleSource::new(source_root.path().to_path_buf()),
            ]),
        )
        .build();
    let mut context = radiant::prelude::UiUpdateContext::default();

    state
        .library
        .folder_browser
        .select_file(first_path_string.clone());
    state.record_sample_last_played(first_path_string, &mut context);
    state
        .library
        .folder_browser
        .select_file(second_path_string.clone());
    state.record_sample_last_played(second_path_string.clone(), &mut context);

    let delayed = run_after_commands(context.into_command());
    assert_eq!(delayed.len(), 2);
    let mut stale_context = radiant::prelude::UiUpdateContext::default();
    state.apply_message(delayed[0].clone(), &mut stale_context);
    assert!(
        matches!(stale_context.into_command(), Command::None),
        "stale delayed last-played writes should not schedule disk work"
    );

    let mut latest_context = radiant::prelude::UiUpdateContext::default();
    state.apply_message(delayed[1].clone(), &mut latest_context);
    let message =
        run_first_perform(latest_context.into_command()).expect("latest last played persist");

    assert!(matches!(
        message,
        crate::native_app::test_support::state::GuiMessage::LastPlayedPersisted(result)
            if result.file_id == second_path_string
    ));
}

#[test]
fn active_playback_defers_last_played_disk_persist_until_idle() {
    let source_root = tempfile::tempdir().expect("source root");
    let sample_path = source_root.path().join("sample.wav");
    fs::write(&sample_path, [1_u8, 2, 3, 4]).expect("write sample");
    let sample_path_string = sample_path.display().to_string();

    let mut state = crate::native_app::test_support::state::NativeAppStateFixture::default()
        .with_folder_browser(
            crate::native_app::test_support::state::FolderBrowserState::from_sample_sources(&[
                wavecrate::sample_sources::SampleSource::new(source_root.path().to_path_buf()),
            ]),
        )
        .build();
    let mut context = radiant::prelude::UiUpdateContext::default();

    state
        .library
        .folder_browser
        .select_file(sample_path_string.clone());
    state.record_sample_last_played(sample_path_string.clone(), &mut context);
    state.waveform.current.start_playback(0.25);

    let delayed = run_first_after(context.into_command()).expect("last played delayed command");
    let mut active_context = radiant::prelude::UiUpdateContext::default();
    state.apply_message(delayed, &mut active_context);

    let (retry_messages, perform_count) =
        collect_after_commands_and_perform_count(active_context.into_command());
    assert_eq!(
        perform_count, 0,
        "active playback should defer last-played persistence before DB work starts"
    );
    assert_eq!(
        retry_messages.len(),
        1,
        "active playback should reschedule last-played persistence"
    );

    state.waveform.current.stop_playback();
    let retry = retry_messages
        .into_iter()
        .next()
        .expect("retry last played persist");
    let mut idle_context = radiant::prelude::UiUpdateContext::default();
    state.apply_message(retry, &mut idle_context);
    let message = run_first_perform(idle_context.into_command())
        .expect("idle last played should persist to disk");

    assert!(matches!(
        message,
        crate::native_app::test_support::state::GuiMessage::LastPlayedPersisted(result)
            if result.file_id == sample_path_string
    ));
}

fn run_first_after(
    command: Command<crate::native_app::test_support::state::GuiMessage>,
) -> Option<crate::native_app::test_support::state::GuiMessage> {
    run_after_commands(command).into_iter().next()
}

fn run_after_commands(
    command: Command<crate::native_app::test_support::state::GuiMessage>,
) -> Vec<crate::native_app::test_support::state::GuiMessage> {
    match command {
        Command::After { message, .. } => vec![message],
        Command::Batch(commands) => commands.into_iter().flat_map(run_after_commands).collect(),
        _ => Vec::new(),
    }
}

fn collect_after_commands_and_perform_count(
    command: Command<crate::native_app::test_support::state::GuiMessage>,
) -> (
    Vec<crate::native_app::test_support::state::GuiMessage>,
    usize,
) {
    match command {
        Command::After { message, .. } => (vec![message], 0),
        Command::Perform { .. } => (Vec::new(), 1),
        Command::Batch(commands) => commands
            .into_iter()
            .map(collect_after_commands_and_perform_count)
            .fold((Vec::new(), 0), |mut collected, (messages, count)| {
                collected.0.extend(messages);
                collected.1 += count;
                collected
            }),
        _ => (Vec::new(), 0),
    }
}

fn run_first_perform(
    command: Command<crate::native_app::test_support::state::GuiMessage>,
) -> Option<crate::native_app::test_support::state::GuiMessage> {
    match command {
        Command::Perform { work, .. } => Some(work()),
        Command::Batch(commands) => commands.into_iter().find_map(run_first_perform),
        _ => None,
    }
}

fn assert_starmap_viewport_reveals(
    viewport: crate::native_app::app::StarmapViewport,
    position: (f32, f32),
    message: &str,
) {
    let half_span = 0.5 / viewport.zoom.max(1.0);
    let min_x = viewport.center_x - half_span;
    let max_x = viewport.center_x + half_span;
    let min_y = viewport.center_y - half_span;
    let max_y = viewport.center_y + half_span;
    assert!(
        position.0 >= min_x - 0.001
            && position.0 <= max_x + 0.001
            && position.1 >= min_y - 0.001
            && position.1 <= max_y + 0.001,
        "{message}: position {:?} outside x=[{min_x:.3}, {max_x:.3}] y=[{min_y:.3}, {max_y:.3}]",
        position
    );
}
