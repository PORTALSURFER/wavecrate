use super::*;
use crate::compat_app_contract::AutomationNodeSnapshot;

fn child<'a>(parent: &'a AutomationNodeSnapshot, id: &str) -> &'a AutomationNodeSnapshot {
    parent
        .children
        .iter()
        .find(|node| node.id.0 == id)
        .unwrap_or_else(|| panic!("missing automation child {id}"))
}

#[test]
fn waveform_motion_overlay_draws_slice_preview_overlays() {
    let layout = ShellLayout::build(Vector2::new(1280.0, 720.0));
    let style = StyleTokens::for_viewport_width(1280.0);
    let mut state = NativeShellState::new();
    let mut model = AppModel::default();
    let slice = crate::compat_app_contract::WaveformSlicePreviewModel {
        range: crate::compat_app_contract::NormalizedRangeModel::new(180, 420),
        selected: false,
        focused: false,
        marked_for_export: false,
        duplicate_cleanup_candidate: false,
        duplicate_cleanup_exempted: false,
    };
    model.waveform.slices.push(slice.clone());
    let motion = NativeMotionModel::from_app_model(&model);

    let expected_rect = compute_waveform_slice_preview_rects(
        layout.waveform_plot,
        &model.waveform.slices,
        model.waveform.view_start_micros,
        model.waveform.view_end_micros,
    )[0]
    .rect;

    let mut frame = NativeViewFrame::default();
    state.build_motion_overlay_into(&layout, &style, &motion, &mut frame);

    let fill = frame
        .primitives
        .iter()
        .find_map(|primitive| match primitive {
            Primitive::Rect(rect) if rect.rect == expected_rect => Some(rect.color),
            _ => None,
        })
        .expect("slice preview fill");

    assert_eq!(
        fill,
        translucent_overlay_color(style.bg_secondary, style.highlight_blue, 0.44)
    );

    let border_segments = frame
        .primitives
        .iter()
        .filter_map(|primitive| match primitive {
            Primitive::Rect(rect)
                if rect.color == blend_color(style.highlight_blue, style.text_primary, 0.18)
                    && rect.rect != expected_rect =>
            {
                Some(rect.rect)
            }
            _ => None,
        })
        .count();
    assert_eq!(border_segments, 4);
}

#[test]
fn waveform_motion_overlay_draws_selected_slice_preview_with_stronger_fill() {
    let layout = ShellLayout::build(Vector2::new(1280.0, 720.0));
    let style = StyleTokens::for_viewport_width(1280.0);
    let mut state = NativeShellState::new();
    let mut model = AppModel::default();
    let slice = crate::compat_app_contract::WaveformSlicePreviewModel {
        range: crate::compat_app_contract::NormalizedRangeModel::new(180, 420),
        selected: true,
        focused: false,
        marked_for_export: false,
        duplicate_cleanup_candidate: false,
        duplicate_cleanup_exempted: false,
    };
    model.waveform.slices.push(slice.clone());
    let motion = NativeMotionModel::from_app_model(&model);

    let expected_rect = compute_waveform_slice_preview_rects(
        layout.waveform_plot,
        &model.waveform.slices,
        model.waveform.view_start_micros,
        model.waveform.view_end_micros,
    )[0]
    .rect;

    let mut frame = NativeViewFrame::default();
    state.build_motion_overlay_into(&layout, &style, &motion, &mut frame);

    let fill = frame
        .primitives
        .iter()
        .find_map(|primitive| match primitive {
            Primitive::Rect(rect) if rect.rect == expected_rect => Some(rect.color),
            _ => None,
        })
        .expect("slice preview fill");

    assert_eq!(
        fill,
        translucent_overlay_color(style.surface_overlay, style.highlight_blue, 0.72)
    );
}

#[test]
fn waveform_motion_overlay_draws_exempted_duplicate_preview_with_cool_accent() {
    let layout = ShellLayout::build(Vector2::new(1280.0, 720.0));
    let style = StyleTokens::for_viewport_width(1280.0);
    let mut state = NativeShellState::new();
    let mut model = AppModel::default();
    model
        .waveform
        .slices
        .push(crate::compat_app_contract::WaveformSlicePreviewModel {
            range: crate::compat_app_contract::NormalizedRangeModel::new(180, 420),
            selected: false,
            focused: false,
            marked_for_export: false,
            duplicate_cleanup_candidate: true,
            duplicate_cleanup_exempted: true,
        });
    let motion = NativeMotionModel::from_app_model(&model);
    let expected_rect = compute_waveform_slice_preview_rects(
        layout.waveform_plot,
        &model.waveform.slices,
        model.waveform.view_start_micros,
        model.waveform.view_end_micros,
    )[0]
    .rect;

    let mut frame = NativeViewFrame::default();
    state.build_motion_overlay_into(&layout, &style, &motion, &mut frame);

    let fill = frame
        .primitives
        .iter()
        .find_map(|primitive| match primitive {
            Primitive::Rect(rect) if rect.rect == expected_rect => Some(rect.color),
            _ => None,
        })
        .expect("slice preview fill");

    assert_eq!(
        fill,
        translucent_overlay_color(style.surface_overlay, style.accent_mint, 0.74)
    );
}

#[test]
fn waveform_automation_exposes_slice_toggle_and_detect_actions() {
    let layout = ShellLayout::build(Vector2::new(1280.0, 720.0));
    let style = StyleTokens::for_viewport_width(1280.0);
    let mut model = AppModel::default();
    model.waveform.loaded_label = Some(String::from("kick.wav"));
    model
        .waveform
        .slices
        .push(crate::compat_app_contract::WaveformSlicePreviewModel {
            range: crate::compat_app_contract::NormalizedRangeModel::new(180, 420),
            selected: true,
            focused: true,
            marked_for_export: true,
            duplicate_cleanup_candidate: false,
            duplicate_cleanup_exempted: false,
        });
    let mut state = NativeShellState::new();
    let node = state.automation_snapshot(&layout, &model);
    let waveform = child(&node.root, "waveform.panel");
    let region = child(waveform, "waveform.region");

    assert!(
        region
            .available_actions
            .contains(&String::from("detect_waveform_silence_slices"))
    );
    assert!(
        region
            .available_actions
            .contains(&String::from("detect_waveform_exact_duplicate_slices"))
    );
    assert!(
        region
            .available_actions
            .contains(&String::from("clean_waveform_exact_duplicate_slices"))
    );
    assert!(
        region
            .available_actions
            .contains(&String::from("move_waveform_slice_focus"))
    );
    assert!(
        region
            .available_actions
            .contains(&String::from("toggle_focused_waveform_slice_export_mark"))
    );
    let slice = child(waveform, "waveform.slice.000");
    assert_eq!(slice.selected, true);
    assert_eq!(slice.value.as_deref(), Some("180000-420000"));
    assert_eq!(slice.metadata.get("focused"), Some(&String::from("true")));
    assert_eq!(
        slice.metadata.get("marked_for_export"),
        Some(&String::from("true"))
    );
    assert_eq!(
        slice.metadata.get("edit_selected"),
        Some(&String::from("true"))
    );
    assert_eq!(
        slice.metadata.get("duplicate_cleanup_candidate"),
        Some(&String::from("false"))
    );
    assert!(
        slice
            .available_actions
            .contains(&String::from("toggle_waveform_slice_selection"))
    );

    let buttons = waveform_toolbar_buttons(
        &layout,
        &style,
        &NativeMotionModel::from_app_model(&model),
        false,
        None,
    );
    assert!(
        buttons.iter().any(|button| {
            button.label == "Silence Split"
                && button.action == Some(UiAction::DetectWaveformSilenceSlices)
        }),
        "silence split toolbar button should be present"
    );
    assert!(
        buttons.iter().any(|button| {
            button.label == "Exact Dedupe"
                && button.action == Some(UiAction::DetectWaveformExactDuplicateSlices)
        }),
        "exact dedupe toolbar button should be present"
    );
    assert!(
        buttons.iter().any(|button| {
            button.label == "Clean Dups"
                && button.action == Some(UiAction::CleanWaveformExactDuplicateSlices)
        }),
        "clean dups toolbar button should be present"
    );
}
