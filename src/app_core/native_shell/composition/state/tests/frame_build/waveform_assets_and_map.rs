use super::*;
#[test]
fn waveform_title_uses_primary_text_hierarchy_color() {
    let layout = ShellLayout::build(Vector2::new(1280.0, 720.0));
    let style = style_for_layout(&layout);
    let mut state = NativeShellState::new();
    let mut model = AppModel::default();
    model.waveform.loaded_label = Some(String::from("WaveTitle"));
    let frame = state.build_frame(&layout, &model);
    assert!(
        frame
            .text_runs
            .iter()
            .any(|run| run.text == "WaveTitle" && run.color == style.text_primary)
    );
}

#[test]
fn waveform_image_data_emits_textured_waveform_primitive() {
    let layout = ShellLayout::build(Vector2::new(1280.0, 720.0));
    let mut state = NativeShellState::new();
    let mut model = AppModel::default();
    model.waveform.waveform_image = Some(std::sync::Arc::new(
        ImageRgba::new(1, 1, vec![11, 22, 33, 255]).unwrap(),
    ));
    let frame = state.build_frame(&layout, &model);
    let has_waveform_image = frame
        .primitives
        .iter()
        .any(|primitive| matches!(primitive, Primitive::Image(image) if image.rect == layout.waveform_plot));
    assert!(has_waveform_image);
}

#[test]
fn waveform_image_data_preserves_distinct_colors_in_texture_payload() {
    let layout = ShellLayout::build(Vector2::new(1280.0, 720.0));
    let mut state = NativeShellState::new();
    let mut model = AppModel::default();
    model.waveform.waveform_image = Some(std::sync::Arc::new(
        ImageRgba::new(
            1,
            2,
            vec![
                11, 22, 33, 255, // top pixel
                99, 88, 77, 255, // bottom pixel
            ],
        )
        .unwrap(),
    ));
    let frame = state.build_frame(&layout, &model);
    let (top_color_present, bottom_color_present) = frame
        .primitives
        .iter()
        .find_map(|primitive| match primitive {
            Primitive::Image(image) if image.rect == layout.waveform_plot => Some((
                image.image.pixels.get(0..4) == Some(&[11, 22, 33, 255]),
                image.image.pixels.get(4..8) == Some(&[99, 88, 77, 255]),
            )),
            _ => None,
        })
        .unwrap_or((false, false));
    assert!(top_color_present);
    assert!(bottom_color_present);
}

#[test]
fn waveform_image_transparent_pixels_do_not_emit_texture_primitive() {
    let layout = ShellLayout::build(Vector2::new(1280.0, 720.0));
    let mut state = NativeShellState::new();
    let mut model = AppModel::default();
    model.waveform.waveform_image = Some(std::sync::Arc::new(
        ImageRgba::new(1, 1, vec![11, 22, 33, 0]).unwrap(),
    ));
    let frame = state.build_frame(&layout, &model);
    let has_waveform_image = frame
        .primitives
        .iter()
        .any(|primitive| matches!(primitive, Primitive::Image(image) if image.rect == layout.waveform_plot));
    assert!(!has_waveform_image);
}

#[test]
fn waveform_loading_motion_overlay_draws_neutral_waveform_placeholder() {
    let layout = ShellLayout::build(Vector2::new(1280.0, 720.0));
    let style = style_for_layout(&layout);
    let mut state = NativeShellState::new();
    let mut model = AppModel::default();
    model.waveform.loading = true;
    let motion = NativeMotionModel::from_app_model(&model);
    let mut overlay = NativeViewFrame::default();

    state.build_motion_overlay_into(&layout, &style, &motion, &mut overlay);

    assert!(overlay.primitives.iter().any(|primitive| {
        matches!(
            primitive,
            Primitive::Rect(rect)
                if rect.rect == layout.waveform_plot && rect.color == style.surface_base
        )
    }));

    let placeholder_rects = overlay
        .primitives
        .iter()
        .filter_map(|primitive| match primitive {
            Primitive::Rect(rect)
                if rect.rect != layout.waveform_plot
                    && rect.rect.min.x >= layout.waveform_plot.min.x
                    && rect.rect.max.x <= layout.waveform_plot.max.x
                    && rect.rect.min.y >= layout.waveform_plot.min.y
                    && rect.rect.max.y <= layout.waveform_plot.max.y =>
            {
                Some(*rect)
            }
            _ => None,
        })
        .collect::<Vec<_>>();

    assert!(
        placeholder_rects.len() >= 10,
        "loading placeholder should emit a multi-column waveform silhouette"
    );
    assert!(
        placeholder_rects
            .iter()
            .all(|rect| rect.color != style.accent_warning),
        "loading placeholder should avoid warning-accent colors"
    );
    assert!(
        placeholder_rects
            .iter()
            .all(|rect| rect.rect.width() < layout.waveform_plot.width() * 0.08),
        "loading placeholder should avoid the previous wide loading bars"
    );
}

#[test]
fn map_header_prefers_projected_legend_selection_and_viewport_copy() {
    let layout = ShellLayout::build(Vector2::new(1280.0, 720.0));
    let mut state = NativeShellState::new();
    let mut model = AppModel::default();
    model.map.active = true;
    model.map.legend_label = String::from("Render: points");
    model.map.selection_label = String::from("Selection: kick_24.wav");
    model.map.hover_label = String::from("Hover: kick_hover.wav");
    model.map.cluster_label = String::from("Clusters: 7");
    model.map.viewport_label = String::from("zoom 1.75x | pan (12, -8)");
    model.map.summary = String::from("248 points");

    let frame = state.build_frame(&layout, &model);
    assert!(
        frame
            .text_runs
            .iter()
            .any(|run| run.text.contains("Render: points"))
    );
    assert!(
        frame
            .text_runs
            .iter()
            .any(|run| run.text.contains("Selection: kick_24.wav"))
    );
    assert!(
        frame
            .text_runs
            .iter()
            .any(|run| run.text.contains("Clusters: 7"))
    );
    assert!(
        frame
            .text_runs
            .iter()
            .any(|run| run.text.contains("zoom 1.75x | pan (12, -8)"))
    );
    assert!(
        frame
            .text_runs
            .iter()
            .any(|run| run.text.contains("248 points"))
    );
}

#[test]
fn map_header_metadata_stays_within_header_band() {
    let layout = ShellLayout::build(Vector2::new(820.0, 520.0));
    let mut state = NativeShellState::new();
    let mut model = AppModel::default();
    model.map.active = true;
    model.map.legend_label = String::from("Render: points");
    model.map.selection_label = String::from("Selection: very_long_sample_name.wav");
    model.map.cluster_label = String::from("Clusters: 42");

    let frame = state.build_frame(&layout, &model);
    let header_runs = frame
        .text_runs
        .iter()
        .filter(|run| run.text.contains("Render:") || run.text.contains("Selection:"))
        .collect::<Vec<_>>();
    assert!(!header_runs.is_empty());
    for run in header_runs {
        assert_text_run_inside_band(run, layout.browser_table_header);
    }
}
