use super::*;

#[test]
fn browser_header_omits_bucket_label() {
    let layout = ShellLayout::build(Vector2::new(1280.0, 720.0));
    let mut state = NativeShellState::new();
    let model = browser_model_with_rows(24, 8);
    let frame = state.build_frame(&layout, &model);
    assert!(!frame.text_runs.iter().any(|run| run.text == "Bucket"));
}

#[test]
fn static_segments_include_browser_rows_when_list_tab_is_active() {
    let layout = ShellLayout::build(Vector2::new(1280.0, 720.0));
    let style = style_for_layout(&layout);
    let mut state = NativeShellState::new();
    let model = browser_model_with_rows(120, 40);
    let mut segments = StaticFrameSegments::default();
    for segment in StaticFrameSegment::ALL {
        state.build_static_segment_with_style_into(
            &layout,
            &style,
            &model,
            None,
            segment,
            &mut segments,
        );
    }
    let rows_segment = segments.frame(StaticFrameSegment::BrowserRowsWindow);
    let map_segment = segments.frame(StaticFrameSegment::MapPanel);
    assert!(!rows_segment.primitives.is_empty());
    assert!(!rows_segment.text_runs.is_empty());
    assert!(map_segment.primitives.is_empty());
    assert!(state.browser_rows_cache_key.is_some());
    assert!(!state.browser_rows.is_empty());
}

#[test]
fn static_segments_include_map_panel_when_map_tab_is_active() {
    let layout = ShellLayout::build(Vector2::new(1280.0, 720.0));
    let style = style_for_layout(&layout);
    let mut state = NativeShellState::new();
    let mut model = browser_model_with_rows(120, 40);
    model.map.active = true;
    model.map.summary = String::from("Map summary");
    model.map.selected_item_id = Some(String::from("kick"));
    model.map.focused_item_id = Some(String::from("kick"));
    model.map.points = std::sync::Arc::from(vec![crate::compat_app_contract::MapPointModel {
        id: std::sync::Arc::<str>::from("kick"),
        x_milli: 512,
        y_milli: 480,
        cluster_id: Some(1),
    }]);
    let mut segments = StaticFrameSegments::default();
    for segment in StaticFrameSegment::ALL {
        state.build_static_segment_with_style_into(
            &layout,
            &style,
            &model,
            None,
            segment,
            &mut segments,
        );
    }
    let rows_segment = segments.frame(StaticFrameSegment::BrowserRowsWindow);
    let map_segment = segments.frame(StaticFrameSegment::MapPanel);
    assert!(rows_segment.primitives.is_empty());
    assert!(!map_segment.primitives.is_empty());
    assert!(!map_segment.text_runs.is_empty());
}
