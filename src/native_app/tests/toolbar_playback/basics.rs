use super::*;

#[test]
fn toolbar_icon_assets_parse_and_paint_through_radiant_icon_button() {
    for icon in [
        crate::native_app::test_support::toolbar::ToolbarIcon::FocusLoaded,
        crate::native_app::test_support::toolbar::ToolbarIcon::Loop,
        crate::native_app::test_support::toolbar::ToolbarIcon::Random,
        crate::native_app::test_support::toolbar::ToolbarIcon::SimilarSections,
        crate::native_app::test_support::toolbar::ToolbarIcon::BeatGuides,
        crate::native_app::test_support::toolbar::ToolbarIcon::BeatGuideMinus,
        crate::native_app::test_support::toolbar::ToolbarIcon::BeatGuidePlus,
        crate::native_app::test_support::toolbar::ToolbarIcon::Play,
        crate::native_app::test_support::toolbar::ToolbarIcon::Stop,
    ] {
        assert_eq!(
            crate::native_app::test_support::toolbar::toolbar_icon_color(true, false),
            radiant::prelude::Rgba8::new(238, 238, 238, 255)
        );
        assert_eq!(
            crate::native_app::test_support::toolbar::toolbar_icon_color(true, true),
            radiant::prelude::Rgba8::new(255, 160, 82, 255)
        );
        assert_eq!(
            crate::native_app::test_support::toolbar::toolbar_icon_color(false, false),
            radiant::prelude::Rgba8::new(145, 145, 145, 255)
        );
        let mut primitives = Vec::new();
        crate::native_app::test_support::toolbar::toolbar_icon_glyph(icon, true, false)
            .append_paint(&mut primitives, 101, Rect::from_size(28.0, 24.0));
        assert!(
            primitives.iter().any(|primitive| primitive.svg().is_some()),
            "toolbar icon cache should produce a retained Radiant SVG"
        );
        let frame =
            crate::native_app::test_support::toolbar::toolbar_icon_button(101, icon, true, false)
                .view_frame_at_size_with_default_theme(Vector2::new(28.0, 24.0));
        assert!(
            frame.paint_plan.svgs().next().is_some(),
            "toolbar icon should paint as a retained Radiant SVG"
        );
    }
}

#[test]
fn toolbar_icon_button_routes_messages_through_radiant_builder() {
    for (icon, message) in [
        (
            crate::native_app::test_support::toolbar::ToolbarIcon::FocusLoaded,
            crate::native_app::test_support::state::GuiMessage::FocusLoadedFile,
        ),
        (
            crate::native_app::test_support::toolbar::ToolbarIcon::Loop,
            crate::native_app::test_support::state::GuiMessage::ToggleLoopPlayback,
        ),
        (
            crate::native_app::test_support::toolbar::ToolbarIcon::Random,
            crate::native_app::test_support::state::GuiMessage::PlayRandomSampleRange,
        ),
        (
            crate::native_app::test_support::toolbar::ToolbarIcon::SimilarSections,
            crate::native_app::test_support::state::GuiMessage::ToggleSimilarSections,
        ),
        (
            crate::native_app::test_support::toolbar::ToolbarIcon::BeatGuides,
            crate::native_app::test_support::state::GuiMessage::ToggleBeatGuides,
        ),
        (
            crate::native_app::test_support::toolbar::ToolbarIcon::BeatGuideMinus,
            crate::native_app::test_support::state::GuiMessage::AdjustBeatGuideCount(-1),
        ),
        (
            crate::native_app::test_support::toolbar::ToolbarIcon::BeatGuidePlus,
            crate::native_app::test_support::state::GuiMessage::AdjustBeatGuideCount(1),
        ),
    ] {
        assert_eq!(
            crate::native_app::test_support::toolbar::toolbar_icon_button(101, icon, true, false)
                .view_dispatch_widget_output(
                    101,
                    radiant::widgets::WidgetOutput::typed(
                        radiant::widgets::ButtonMessage::Activate
                    ),
                ),
            Some(message)
        );
    }

    assert_eq!(
        crate::native_app::test_support::toolbar::toolbar_icon_button(
            101,
            crate::native_app::test_support::toolbar::ToolbarIcon::Random,
            true,
            false,
        )
        .view_dispatch_widget_output(
            101,
            radiant::widgets::WidgetOutput::typed(
                radiant::widgets::ButtonMessage::ActivateWithModifiers {
                    modifiers: PointerModifiers {
                        command: true,
                        ..Default::default()
                    },
                },
            ),
        ),
        Some(
            crate::native_app::test_support::state::GuiMessage::ToggleStickyRandomSampleRangePlayback
        )
    );

    assert_eq!(
        crate::native_app::test_support::toolbar::toolbar_icon_button(
            101,
            crate::native_app::test_support::toolbar::ToolbarIcon::Random,
            true,
            false,
        )
        .view_dispatch_widget_output(
            101,
            radiant::widgets::WidgetOutput::typed(
                radiant::widgets::ButtonMessage::ActivateWithModifiers {
                    modifiers: PointerModifiers {
                        shift: true,
                        ..Default::default()
                    },
                },
            ),
        ),
        Some(crate::native_app::test_support::state::GuiMessage::PlayRandomListedSampleRange)
    );
}

#[test]
fn main_toolbar_does_not_paint_empty_spacer_border() {
    let state = NativeAppState::load_default().expect("default state loads");
    let frame = crate::native_app::test_support::toolbar::main_toolbar(&state)
        .view_frame_at_size_with_default_theme(Vector2::new(664.0, 34.0));

    assert!(
        !frame
            .paint_plan
            .contains_paint_rect_matching(|rect| rect.width() > 100.0 && rect.height() >= 20.0),
        "empty toolbar spacer should not paint or reserve a large visible rectangle"
    );
}

#[test]
fn apply_edit_mark_edits_button_appears_only_for_pending_effects() {
    let mut state = gui_state_for_span_tests();
    state
        .waveform
        .current
        .set_edit_selection_range(wavecrate::selection::SelectionRange::new(0.2, 0.6));

    let plain = crate::native_app::test_support::toolbar::main_toolbar(&state)
        .view_frame_at_size_with_default_theme(Vector2::new(720.0, 34.0));
    assert!(
        plain
            .paint_plan
            .first_widget_rect(
                crate::native_app::test_support::toolbar::TOOLBAR_APPLY_EDIT_MARK_EDITS_ID
            )
            .is_none(),
        "plain edit marks should not show an apply button"
    );

    state.waveform.current.set_edit_selection_range(
        wavecrate::selection::SelectionRange::new(0.2, 0.6).with_gain(0.5),
    );
    let edited = crate::native_app::test_support::toolbar::main_toolbar(&state)
        .view_frame_at_size_with_default_theme(Vector2::new(720.0, 34.0));
    assert!(
        edited
            .paint_plan
            .first_widget_rect(
                crate::native_app::test_support::toolbar::TOOLBAR_APPLY_EDIT_MARK_EDITS_ID
            )
            .is_some(),
        "pending edit mark effects should show an apply button"
    );
    assert_eq!(
        crate::native_app::test_support::toolbar::main_toolbar(&state).view_dispatch_widget_output(
            crate::native_app::test_support::toolbar::TOOLBAR_APPLY_EDIT_MARK_EDITS_ID,
            radiant::widgets::WidgetOutput::typed(radiant::widgets::ButtonMessage::Activate),
        ),
        Some(crate::native_app::test_support::state::GuiMessage::RequestApplyEditSelectionEffects)
    );
}

#[test]
fn random_toolbar_help_tooltip_paints_multiline_guidance() {
    let mut state = gui_state_for_span_tests();
    state.ui.chrome.help_tooltips_enabled = true;
    let bridge = radiant::runtime::DeclarativeOwnedRuntimeBridge::new(
        state,
        |state| crate::native_app::test_support::toolbar::main_toolbar(state).into_surface(),
        |_, _| {},
    );
    let mut runtime = SurfaceRuntime::new(bridge, Vector2::new(664.0, 80.0));
    let random = *runtime
        .layout()
        .rects
        .get(&crate::native_app::test_support::toolbar::TOOLBAR_RANDOM_ID)
        .expect("random toolbar button should lay out");

    runtime.dispatch_event(Event::pointer_move(random.center()));

    let frame = runtime.frame_with_default_theme();
    assert!(frame.paint_plan.contains_text("Random section playback"));
    assert!(
        frame
            .paint_plan
            .contains_text("Click: play a random section now.")
    );
    assert!(
        frame
            .paint_plan
            .text_label_strings()
            .iter()
            .any(|text| text.contains("Shift-click: pick a random"))
    );
    assert!(
        frame
            .paint_plan
            .text_label_strings()
            .iter()
            .any(|text| text.contains("Command-click: make Space use random"))
    );
    assert!(frame.paint_plan.contains_text("sections."));
}

#[test]
fn main_toolbar_view_model_projects_playback_state() {
    let mut state = NativeAppState::load_default().expect("default state loads");

    let empty = crate::native_app::test_support::toolbar::main_toolbar_projection(&state);
    assert_eq!(empty.random_available, state.random_playback_available());
    assert!(!empty.sticky_random_sample_range_playback);
    assert!(!empty.similar_sections_available);
    assert!(!empty.similar_sections_enabled);
    assert!(!empty.loop_playback);
    assert!(!empty.playing);
    assert!(!empty.beat_guides_enabled);
    assert_eq!(empty.beat_guide_count, 4);
    assert!(empty.can_decrement_beat_guide_count);
    assert!(empty.can_increment_beat_guide_count);
    assert!(!empty.pending_edit_mark_edits);

    state.audio.loop_playback = true;
    state.ui.chrome.sticky_random_sample_range_playback = true;
    state.ui.chrome.beat_guides_enabled = true;
    state.ui.chrome.beat_guide_count = 8;
    state.waveform.current =
        crate::native_app::test_support::state::WaveformState::synthetic_for_tests();
    state.waveform.current.start_playback(0.25);
    state.waveform.current.set_edit_selection_range(
        wavecrate::selection::SelectionRange::new(0.2, 0.6).with_gain(0.5),
    );
    state.waveform.current.set_play_selection_range(0.1, 0.2);

    let loaded = crate::native_app::test_support::toolbar::main_toolbar_projection(&state);
    assert_eq!(loaded.random_available, state.random_playback_available());
    assert!(loaded.sticky_random_sample_range_playback);
    assert!(loaded.similar_sections_available);
    assert!(!loaded.similar_sections_enabled);
    assert!(loaded.loop_playback);
    assert!(loaded.playing);
    assert!(loaded.beat_guides_enabled);
    assert_eq!(loaded.beat_guide_count, 8);
    assert!(loaded.pending_edit_mark_edits);
}

#[test]
fn sticky_random_toolbar_message_updates_space_playback_mode() {
    let mut state = NativeAppState::load_default().expect("default state loads");
    let mut context = radiant::prelude::UiUpdateContext::default();

    assert!(!state.ui.chrome.sticky_random_sample_range_playback);

    state.apply_message(
        GuiMessage::ToggleStickyRandomSampleRangePlayback,
        &mut context,
    );

    assert!(state.ui.chrome.sticky_random_sample_range_playback);
    assert_eq!(
        state.ui.status.sample,
        "Sticky random playback on: Space plays random sample sections"
    );

    state.apply_message(
        GuiMessage::ToggleStickyRandomSampleRangePlayback,
        &mut context,
    );

    assert!(!state.ui.chrome.sticky_random_sample_range_playback);
    assert_eq!(
        state.ui.status.sample,
        "Sticky random playback off: Space plays selected samples"
    );
}

#[test]
fn beat_guide_toolbar_messages_update_chrome_state() {
    let mut state = NativeAppState::load_default().expect("default state loads");
    let mut context = radiant::prelude::UiUpdateContext::default();

    state.apply_message(GuiMessage::ToggleBeatGuides, &mut context);
    assert!(state.ui.chrome.beat_guides_enabled);

    state.apply_message(GuiMessage::AdjustBeatGuideCount(3), &mut context);
    assert_eq!(state.ui.chrome.beat_guide_count, 7);

    state.apply_message(GuiMessage::AdjustBeatGuideCount(-100), &mut context);
    assert_eq!(state.ui.chrome.beat_guide_count, 1);

    state.apply_message(GuiMessage::AdjustBeatGuideCount(100), &mut context);
    assert_eq!(state.ui.chrome.beat_guide_count, 64);
}
