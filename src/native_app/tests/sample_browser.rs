use radiant::{
    gui::types::{Point, Rect, Rgba8, Vector2},
    prelude::{IntoView, ThemeTokens, WidgetStyle, WidgetTone, dense_row_palette_from_style},
    runtime::{Command, Event, SurfaceFrame},
    widgets::{PointerButton, PointerModifiers, Widget, WidgetInput},
};
use std::fs;

use super::{native_app_state_with_temp_sample, native_runtime_for_tests};

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

    let message = run_first_perform(context.into_command()).expect("last played persist command");
    state.apply_message(message, &mut radiant::prelude::UiUpdateContext::default());

    assert!(
        db.last_played_at_for_path(std::path::Path::new("played.wav"))
            .expect("read last played")
            .is_some()
    );
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
