use radiant::{
    gui::types::{Point, Rect, Rgba8, Vector2},
    prelude::{IntoView, ThemeTokens, WidgetStyle, WidgetTone, dense_row_palette_from_style},
    runtime::{Command, Event, SurfaceFrame},
    widgets::{PointerButton, PointerModifiers, Widget, WidgetInput},
};
use std::fs;

use super::{native_app_state_with_temp_sample, native_runtime_for_tests, run_command_for_tests};

fn sample_hit_target(
    selected: bool,
    drag_active: bool,
    drag_source: bool,
    cached: bool,
) -> crate::native_app::test_support::sample_browser::SampleFileHitTarget {
    crate::native_app::test_support::sample_browser::sample_file_hit_target(
        String::from("sample.wav"),
        selected,
        drag_active,
        drag_source,
        cached,
    )
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
