use radiant::{
    gui::types::{Point, Rect, Rgba8, Vector2},
    prelude::{IntoView, ThemeTokens, WidgetStyle, WidgetTone, dense_row_palette_from_style},
    runtime::{Command, Event, SurfaceFrame, SurfacePaintPlan, UiSurface},
    widgets::{PointerButton, PointerModifiers, Widget, WidgetInput, WidgetOutput},
};
use std::{fs, path::PathBuf};

use super::{
    native_app_state_with_temp_sample, native_runtime_for_tests, run_command_for_tests,
    write_test_wav_i16,
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
    let mut context = radiant::prelude::UiUpdateContext::default();

    state.record_sample_last_played(sample_path_string.clone(), &mut context);

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
    state.library.folder_browser.select_file(first_id);
    state.ui.chrome.sample_browser_display = crate::native_app::app::SampleBrowserDisplayMode::Map;
    state.ui.chrome.sample_map_viewport.zoom = 4.0;

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
        .selected_sample_map_position(
            crate::native_app::sample_library::folder_browser::sample_map::SampleMapProjection {
                tags_by_file: &state.metadata.tags_by_file,
            },
        )
        .expect("selected sample should have a map position");
    assert!(
        (state.ui.chrome.sample_map_viewport.center_x - expected.0).abs() < 0.001,
        "map viewport should center selected x position"
    );
    assert!(
        (state.ui.chrome.sample_map_viewport.center_y - expected.1).abs() < 0.001,
        "map viewport should center selected y position"
    );
}

#[test]
fn leaving_sample_map_mode_reveals_selected_list_row() {
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
fn sample_map_drag_queues_every_swept_hit_without_collapsing_to_last_sample() {
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
        crate::native_app::test_support::state::GuiMessage::BeginSampleMapAuditionDrag {
            path: Some(first_id.clone()),
            position: Point::new(10.0, 10.0),
            modifiers: PointerModifiers::default(),
        },
        &mut context,
    );
    state.apply_message(
        crate::native_app::test_support::state::GuiMessage::UpdateSampleMapAuditionDrag {
            paths: vec![second_id.clone(), third_id.clone()],
            position: Point::new(90.0, 10.0),
            modifiers: PointerModifiers::default(),
        },
        &mut context,
    );

    assert_eq!(
        state.library.folder_browser.selected_file_id(),
        Some(first_id.as_str()),
        "the first hit should start immediately instead of being cancelled by later swept hits"
    );
    assert_eq!(
        state
            .ui
            .chrome
            .sample_map_audition_queue
            .active_file_id
            .as_deref(),
        Some(first_id.as_str())
    );
    assert_eq!(
        state
            .ui
            .chrome
            .sample_map_audition_queue
            .queued_file_ids
            .iter()
            .cloned()
            .collect::<Vec<_>>(),
        vec![second_id, third_id]
    );
}

#[test]
fn sample_map_audition_advance_selects_next_queued_hit() {
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
        crate::native_app::test_support::state::GuiMessage::BeginSampleMapAuditionDrag {
            path: Some(first_id.clone()),
            position: Point::new(10.0, 10.0),
            modifiers: PointerModifiers::default(),
        },
        &mut context,
    );
    state.apply_message(
        crate::native_app::test_support::state::GuiMessage::UpdateSampleMapAuditionDrag {
            paths: vec![second_id.clone()],
            position: Point::new(90.0, 10.0),
            modifiers: PointerModifiers::default(),
        },
        &mut context,
    );

    let mut advance_context = radiant::prelude::UiUpdateContext::default();
    state.schedule_next_sample_map_audition_hit(&mut advance_context);
    let advance = run_first_after(advance_context.into_command()).expect("queued map advance");
    state.apply_message(advance, &mut radiant::prelude::UiUpdateContext::default());

    assert_eq!(
        state.library.folder_browser.selected_file_id(),
        Some(second_id.as_str())
    );
    assert_eq!(
        state
            .ui
            .chrome
            .sample_map_audition_queue
            .active_file_id
            .as_deref(),
        Some(second_id.as_str())
    );
    assert!(
        state
            .ui
            .chrome
            .sample_map_audition_queue
            .queued_file_ids
            .is_empty()
    );
}

#[test]
fn sample_map_audition_queue_clears_after_last_hit_finishes() {
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
        crate::native_app::test_support::state::GuiMessage::BeginSampleMapAuditionDrag {
            path: Some(first_id),
            position: Point::new(10.0, 10.0),
            modifiers: PointerModifiers::default(),
        },
        &mut radiant::prelude::UiUpdateContext::default(),
    );
    state.apply_message(
        crate::native_app::test_support::state::GuiMessage::FinishSampleMapAuditionDrag,
        &mut radiant::prelude::UiUpdateContext::default(),
    );

    state.schedule_next_sample_map_audition_hit(&mut radiant::prelude::UiUpdateContext::default());

    assert_eq!(state.ui.chrome.sample_map_audition_drag, None);
    assert_eq!(
        state.ui.chrome.sample_map_audition_queue,
        Default::default()
    );
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

fn run_first_perform(
    command: Command<crate::native_app::test_support::state::GuiMessage>,
) -> Option<crate::native_app::test_support::state::GuiMessage> {
    match command {
        Command::Perform { work, .. } => Some(work()),
        Command::Batch(commands) => commands.into_iter().find_map(run_first_perform),
        _ => None,
    }
}
