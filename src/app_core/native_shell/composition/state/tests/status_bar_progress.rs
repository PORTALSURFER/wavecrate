use super::*;

#[test]
fn indeterminate_scan_progress_renders_scan_label_and_file_counter() {
    let layout = ShellLayout::build(Vector2::new(1280.0, 720.0));
    let style = StyleTokens::for_viewport_width(1280.0);
    let mut state = NativeShellState::new();
    state.tick_with_style(0.35, &style);
    let model = AppModel {
        progress_overlay: crate::compat_app_contract::ProgressOverlayModel {
            visible: true,
            modal: false,
            title: String::from("Scanning source"),
            detail: Some(String::from("drums/kick.wav")),
            completed: 432,
            total: 0,
            cancelable: true,
            cancel_requested: false,
        },
        ..AppModel::default()
    };

    let frame = state.build_frame_with_style(&layout, &style, &model);

    assert!(
        frame
            .text_runs
            .iter()
            .any(|run| run.text.contains("Scanning source")),
        "status bar should show the scan label"
    );
    assert!(
        frame.text_runs.iter().any(|run| run.text == "432 files"),
        "status bar should show the scanned-file counter"
    );
    assert!(
        frame.text_runs.iter().any(|run| run.text == "col: 2/3"),
        "status bar should keep the right-side status text visible"
    );
    assert!(
        frame.primitives.iter().any(|primitive| matches!(
            primitive,
            Primitive::Rect(rect)
                if rect.rect.min.x >= layout.status_progress_segment.min.x
                    && rect.rect.max.x <= layout.status_progress_segment.max.x
                    && rect.color == blend_color(style.accent_mint, style.text_primary, 0.18)
        )),
        "status bar should render an indeterminate progress fill"
    );
}

#[test]
fn determinate_analysis_progress_keeps_fraction_counter() {
    let layout = ShellLayout::build(Vector2::new(1280.0, 720.0));
    let style = StyleTokens::for_viewport_width(1280.0);
    let mut state = NativeShellState::new();
    let model = AppModel {
        progress_overlay: crate::compat_app_contract::ProgressOverlayModel {
            visible: true,
            modal: false,
            title: String::from("Analyzing samples"),
            detail: Some(String::from("Jobs 2/5 • Samples 3/8")),
            completed: 2,
            total: 5,
            cancelable: true,
            cancel_requested: false,
        },
        ..AppModel::default()
    };

    let frame = state.build_frame_with_style(&layout, &style, &model);

    assert!(
        frame
            .text_runs
            .iter()
            .any(|run| run.text.contains("Analyzing samples")),
        "status bar should show the analysis label"
    );
    assert!(
        frame.text_runs.iter().any(|run| run.text == "2/5"),
        "status bar should keep determinate counters"
    );
    assert!(
        frame.text_runs.iter().any(|run| run.text == "col: 2/3"),
        "status bar should keep the right-side status text visible"
    );
}

#[test]
fn status_bar_text_cache_reuses_and_invalidates_on_progress_changes() {
    let layout = ShellLayout::build(Vector2::new(1280.0, 720.0));
    let style = StyleTokens::for_viewport_width(1280.0);
    let mut state = NativeShellState::new();
    let mut model = AppModel {
        progress_overlay: crate::compat_app_contract::ProgressOverlayModel {
            visible: true,
            modal: false,
            title: String::from("Analyzing samples"),
            detail: Some(String::from("Jobs 2/5 • Samples 3/8")),
            completed: 2,
            total: 5,
            cancelable: true,
            cancel_requested: false,
        },
        ..AppModel::default()
    };
    let mut segments = StaticFrameSegments::default();

    state.build_static_segment_with_style_into(
        &layout,
        &style,
        &model,
        None,
        StaticFrameSegment::StatusBar,
        &mut segments,
    );
    let first = state.status_bar_text_frame_counts();
    assert_eq!(first.lookup_count, 1);
    assert_eq!(first.cache_hit_count, 0);
    assert_eq!(first.cache_miss_count, 1);

    state.build_static_segment_with_style_into(
        &layout,
        &style,
        &model,
        None,
        StaticFrameSegment::StatusBar,
        &mut segments,
    );
    let second = state.status_bar_text_frame_counts();
    assert_eq!(second.lookup_count, 1);
    assert_eq!(second.cache_hit_count, 1);
    assert_eq!(second.cache_miss_count, 0);

    model.progress_overlay.completed = 3;
    state.build_static_segment_with_style_into(
        &layout,
        &style,
        &model,
        None,
        StaticFrameSegment::StatusBar,
        &mut segments,
    );
    let third = state.status_bar_text_frame_counts();
    assert_eq!(third.lookup_count, 1);
    assert_eq!(third.cache_hit_count, 0);
    assert_eq!(third.cache_miss_count, 1);
}

#[test]
fn status_bar_text_cache_invalidates_when_transport_state_changes() {
    let layout = ShellLayout::build(Vector2::new(1280.0, 720.0));
    let style = StyleTokens::for_viewport_width(1280.0);
    let mut state = NativeShellState::new();
    let mut model = AppModel::default();
    let mut segments = StaticFrameSegments::default();

    model.transport_running = false;
    model.status.left = String::from("Transport: stopped | Selected column: 2");
    state.build_static_segment_with_style_into(
        &layout,
        &style,
        &model,
        None,
        StaticFrameSegment::StatusBar,
        &mut segments,
    );
    let stopped = segments.frame(StaticFrameSegment::StatusBar);
    assert!(
        stopped
            .text_runs
            .iter()
            .any(|run| run.text == "Transport: stopped | Selected column: 2"),
        "status bar should report stopped transport in the default left label"
    );
    let first = state.status_bar_text_frame_counts();
    assert_eq!(first.lookup_count, 1);
    assert_eq!(first.cache_hit_count, 0);
    assert_eq!(first.cache_miss_count, 1);

    segments = StaticFrameSegments::default();
    model.transport_running = true;
    model.status.left = String::from("Transport: running | Selected column: 2");
    state.build_static_segment_with_style_into(
        &layout,
        &style,
        &model,
        None,
        StaticFrameSegment::StatusBar,
        &mut segments,
    );
    let running = segments.frame(StaticFrameSegment::StatusBar);
    assert!(
        running
            .text_runs
            .iter()
            .any(|run| run.text == "Transport: running | Selected column: 2"),
        "status bar should rebuild when transport state changes"
    );
    let second = state.status_bar_text_frame_counts();
    assert_eq!(second.lookup_count, 1);
    assert_eq!(second.cache_hit_count, 0);
    assert_eq!(second.cache_miss_count, 1);
}
