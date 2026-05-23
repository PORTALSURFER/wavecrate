use super::{frame_has_text, selected_asset_file_path};
use crate::gui_app::GuiAppState;
use radiant::{
    gui::types::{Point, Rect, Vector2},
    prelude::IntoView,
    runtime::PaintPrimitive,
    widgets::{PointerButton, Widget, WidgetInput},
};

#[test]
fn bottom_status_bar_reports_selected_sample_count() {
    let mut state = GuiAppState::load_default().expect("default state loads");
    let empty_frame = radiant::runtime::UiSurface::new(
        crate::gui_app::status_bar::bottom_status_bar(&state).into_node(),
    )
    .frame(
        Rect::from_min_size(Point::new(0.0, 0.0), Vector2::new(720.0, 30.0)),
        &radiant::theme::ThemeTokens::default(),
    );
    assert!(frame_has_text(&empty_frame, "0 samples"));
    assert!(!frame_has_text(&empty_frame, "1 sample"));

    let sample_path = selected_asset_file_path(&state.folder_browser, "portal_SS_kick_003.wav");
    state.folder_browser.select_file(sample_path);
    let selected_frame = radiant::runtime::UiSurface::new(
        crate::gui_app::status_bar::bottom_status_bar(&state).into_node(),
    )
    .frame(
        Rect::from_min_size(Point::new(0.0, 0.0), Vector2::new(720.0, 30.0)),
        &radiant::theme::ThemeTokens::default(),
    );

    assert!(frame_has_text(&selected_frame, "1 sample"));
}

#[test]
fn bottom_status_progress_bar_paints_without_text_chrome() {
    let mut state = GuiAppState::load_default().expect("default state loads");
    state.folder_progress = Some(crate::gui_app::FolderScanProgress {
        task_id: 7,
        source_id: String::from("assets"),
        label: String::from("Assets"),
        phase: String::from("Scanning"),
        completed: 2,
        total: 5,
        detail: String::from("kick.wav"),
    });
    let frame = radiant::runtime::UiSurface::new(
        crate::gui_app::status_bar::worker_progress_bar(&state).into_node(),
    )
    .frame(
        Rect::from_min_size(Point::new(0.0, 0.0), Vector2::new(180.0, 10.0)),
        &radiant::theme::ThemeTokens::default(),
    );

    let fills = frame
        .paint_plan
        .primitives
        .iter()
        .filter(|primitive| matches!(primitive, PaintPrimitive::FillRect(_)))
        .count();
    assert_eq!(fills, 2);
    assert!(
        frame
            .paint_plan
            .primitives
            .iter()
            .all(|primitive| !matches!(primitive, PaintPrimitive::StrokeRect(_)))
    );
}

#[test]
fn bottom_status_progress_bar_click_opens_job_details() {
    let bounds = Rect::from_min_size(Point::new(0.0, 0.0), Vector2::new(180.0, 10.0));
    let mut progress = crate::gui_app::status_bar::StatusProgressBar::determinate(0.4);
    assert_eq!(
        progress.handle_input(
            bounds,
            WidgetInput::PointerPress {
                position: Point::new(90.0, 5.0),
                button: PointerButton::Primary,
                modifiers: Default::default(),
            },
        ),
        None
    );

    let output = progress
        .handle_input(
            bounds,
            WidgetInput::PointerRelease {
                position: Point::new(90.0, 5.0),
                button: PointerButton::Primary,
                modifiers: Default::default(),
            },
        )
        .expect("progress bar click should activate details");
    assert_eq!(
        output.typed_ref::<crate::gui_app::GuiMessage>(),
        Some(&crate::gui_app::GuiMessage::ToggleJobDetails)
    );
}

#[test]
fn bottom_status_progress_bar_shows_indeterminate_fill_for_unknown_totals() {
    let mut state = GuiAppState::load_default().expect("default state loads");
    state.progress_tick = 0.5;
    state.folder_progress = Some(crate::gui_app::FolderScanProgress {
        task_id: 7,
        source_id: String::from("assets"),
        label: String::from("Assets"),
        phase: String::from("Scanning"),
        completed: 128,
        total: 0,
        detail: String::from("kick.wav"),
    });
    let frame = radiant::runtime::UiSurface::new(
        crate::gui_app::status_bar::worker_progress_bar(&state).into_node(),
    )
    .frame(
        Rect::from_min_size(Point::new(0.0, 0.0), Vector2::new(180.0, 10.0)),
        &radiant::theme::ThemeTokens::default(),
    );

    let fills = frame
        .paint_plan
        .primitives
        .iter()
        .filter(|primitive| matches!(primitive, PaintPrimitive::FillRect(_)))
        .count();
    assert_eq!(fills, 2);
    assert!(
        frame
            .paint_plan
            .primitives
            .iter()
            .all(|primitive| !matches!(primitive, PaintPrimitive::StrokeRect(_)))
    );
}

#[test]
fn job_details_popover_reports_active_scan_progress() {
    let progress = crate::gui_app::FolderScanProgress {
        task_id: 7,
        source_id: String::from("assets"),
        label: String::from("Assets"),
        phase: String::from("Scanning"),
        completed: 2,
        total: 5,
        detail: String::from("kick.wav"),
    };
    let frame = radiant::runtime::UiSurface::new(
        crate::gui_app::status_bar::job_details_popover(&progress).into_node(),
    )
    .frame(
        Rect::from_min_size(Point::new(0.0, 0.0), Vector2::new(360.0, 180.0)),
        &radiant::theme::ThemeTokens::default(),
    );

    assert!(frame_has_text(&frame, "Job Details"));
    assert!(frame_has_text(&frame, "Type: Scanning"));
    assert!(frame_has_text(&frame, "Progress: 2/5"));
    assert!(frame_has_text(&frame, "Current: kick.wav"));
}
