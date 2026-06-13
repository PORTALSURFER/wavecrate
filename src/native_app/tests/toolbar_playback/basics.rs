use super::*;

#[test]
fn toolbar_icon_assets_parse_and_paint_through_radiant_icon_button() {
    for icon in [
        crate::native_app::test_support::toolbar::ToolbarIcon::FocusLoaded,
        crate::native_app::test_support::toolbar::ToolbarIcon::Loop,
        crate::native_app::test_support::toolbar::ToolbarIcon::Random,
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
fn main_toolbar_view_model_projects_playback_state() {
    let mut state = NativeAppState::load_default().expect("default state loads");

    let empty = crate::native_app::test_support::toolbar::main_toolbar_projection(&state);
    assert_eq!(empty.random_available, state.random_playback_available());
    assert!(!empty.loop_playback);
    assert!(!empty.playing);

    state.audio.loop_playback = true;
    state.waveform.current =
        crate::native_app::test_support::state::WaveformState::synthetic_for_tests();
    state.waveform.current.start_playback(0.25);

    let loaded = crate::native_app::test_support::toolbar::main_toolbar_projection(&state);
    assert_eq!(loaded.random_available, state.random_playback_available());
    assert!(loaded.loop_playback);
    assert!(loaded.playing);
}
