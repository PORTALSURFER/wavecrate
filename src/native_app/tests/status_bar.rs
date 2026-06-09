use super::gui_state_for_span_tests;
use crate::native_app::app_chrome::view_models::status_bar::StatusBarViewModel;
use crate::native_app::test_support::NativeAppState;
use radiant::{
    gui::types::{Point, Rect, Vector2},
    prelude::IntoView,
    widgets::WidgetInput,
};

#[test]
fn bottom_status_bar_reports_selected_sample_count() {
    let mut state = gui_state_for_span_tests();
    let source_root = tempfile::tempdir().expect("source root");
    let sample_path = source_root.path().join("selected-count.wav");
    std::fs::write(&sample_path, []).expect("sample file");
    state.folder_browser =
        crate::native_app::test_support::FolderBrowserState::from_sample_sources(&[
            wavecrate::sample_sources::SampleSource::new(source_root.path().to_path_buf()),
        ]);
    let empty_frame = crate::native_app::app_chrome::status_bar::bottom_status_bar(
        StatusBarViewModel::from_app_state(&state),
    )
    .view_frame_at_size_with_default_theme(Vector2::new(720.0, 30.0));
    assert!(empty_frame.paint_plan.contains_text("0 samples"));
    assert!(!empty_frame.paint_plan.contains_text("1 sample"));

    state
        .folder_browser
        .select_file(sample_path.display().to_string());
    let selected_frame = crate::native_app::app_chrome::status_bar::bottom_status_bar(
        StatusBarViewModel::from_app_state(&state),
    )
    .view_frame_at_size_with_default_theme(Vector2::new(720.0, 30.0));

    assert!(selected_frame.paint_plan.contains_text("1 sample"));
}

#[test]
fn bottom_status_progress_bar_paints_without_text_chrome() {
    let mut state = NativeAppState::load_default().expect("default state loads");
    state.folder_progress = Some(crate::native_app::test_support::FolderScanProgress {
        task_id: 7,
        source_id: String::from("assets"),
        label: String::from("Assets"),
        phase: String::from("Scanning"),
        completed: 2,
        total: 5,
        detail: String::from("kick.wav"),
    });
    let model = StatusBarViewModel::from_app_state(&state);
    let frame = crate::native_app::app_chrome::status_bar::worker_progress_bar(
        model.worker_progress,
        model.progress_tick,
    )
    .view_frame_at_size_with_default_theme(Vector2::new(180.0, 10.0));

    let fills = frame.paint_plan.fill_rects().count();
    assert_eq!(fills, 2);
    assert_eq!(frame.paint_plan.stroke_rects().count(), 0);
}

#[test]
fn bottom_status_bar_reports_normalization_progress() {
    let mut state = NativeAppState::load_default().expect("default state loads");
    state.background.normalization_progress =
        Some(crate::native_app::test_support::NormalizationProgress {
            task_id: 9,
            label: String::from("2 samples"),
            completed: 1,
            total: 2,
            detail: String::from("snare.wav"),
        });
    let frame = crate::native_app::app_chrome::status_bar::bottom_status_bar(
        StatusBarViewModel::from_app_state(&state),
    )
    .view_frame_at_size_with_default_theme(Vector2::new(720.0, 30.0));

    assert!(
        frame
            .paint_plan
            .contains_text("Normalizing 2 samples | 1/2 | snare.wav")
    );
}

#[test]
fn bottom_status_progress_bar_click_opens_job_details() {
    let bounds = Rect::from_size(180.0, 10.0);
    let mut progress = radiant::widgets::ProgressBarWidget::determinate(0.4).with_activation();
    assert_eq!(
        progress.handle_input(bounds, WidgetInput::primary_press(Point::new(90.0, 5.0)),),
        None
    );

    assert_eq!(
        progress.handle_input(bounds, WidgetInput::primary_release(Point::new(90.0, 5.0)),),
        Some(radiant::widgets::ProgressBarMessage::Activate)
    );
}

#[test]
fn bottom_status_progress_bar_shows_indeterminate_fill_for_unknown_totals() {
    let mut state = NativeAppState::load_default().expect("default state loads");
    state.background.progress_tick = 0.5;
    state.folder_progress = Some(crate::native_app::test_support::FolderScanProgress {
        task_id: 7,
        source_id: String::from("assets"),
        label: String::from("Assets"),
        phase: String::from("Scanning"),
        completed: 128,
        total: 0,
        detail: String::from("kick.wav"),
    });
    let model = StatusBarViewModel::from_app_state(&state);
    let frame = crate::native_app::app_chrome::status_bar::worker_progress_bar(
        model.worker_progress,
        model.progress_tick,
    )
    .view_frame_at_size_with_default_theme(Vector2::new(180.0, 10.0));

    let fills = frame.paint_plan.fill_rects().count();
    assert_eq!(fills, 2);
    assert_eq!(frame.paint_plan.stroke_rects().count(), 0);
}

#[test]
fn job_details_popover_reports_active_scan_progress() {
    let progress = crate::native_app::test_support::FolderScanProgress {
        task_id: 7,
        source_id: String::from("assets"),
        label: String::from("Assets"),
        phase: String::from("Scanning"),
        completed: 2,
        total: 5,
        detail: String::from("kick.wav"),
    };
    let frame = crate::native_app::app_chrome::status_bar::job_details_popover(&progress)
        .view_frame_at_size_with_default_theme(Vector2::new(360.0, 180.0));

    assert!(frame.paint_plan.contains_text("Job Details"));
    assert!(frame.paint_plan.contains_text("Type: Scanning"));
    assert!(frame.paint_plan.contains_text("Progress: 2/5"));
    assert!(frame.paint_plan.contains_text("Current: kick.wav"));
}

#[test]
fn status_bar_view_model_prioritizes_active_worker_progress() {
    let mut state = NativeAppState::load_default().expect("default state loads");
    state.sample_status = String::from("Ready");
    state.folder_progress = Some(crate::native_app::test_support::FolderScanProgress {
        task_id: 7,
        source_id: String::from("assets"),
        label: String::from("Assets"),
        phase: String::from("Scanning"),
        completed: 2,
        total: 5,
        detail: String::from("kick.wav"),
    });
    state.background.normalization_progress =
        Some(crate::native_app::test_support::NormalizationProgress {
            task_id: 9,
            label: String::from("2 samples"),
            completed: 1,
            total: 2,
            detail: String::from("snare.wav"),
        });

    let model = StatusBarViewModel::from_app_state(&state);

    assert_eq!(model.selected_sample_count, 0);
    assert_eq!(model.status_text, "Scanning Assets | 2/5 | kick.wav");
    assert_eq!(
        model.worker_progress.expect("worker progress"),
        crate::native_app::app_chrome::view_models::status_bar::WorkerProgressViewModel {
            completed: 2,
            total: 5,
        }
    );
}
