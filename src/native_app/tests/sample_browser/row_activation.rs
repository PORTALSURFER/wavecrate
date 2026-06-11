use super::*;

#[test]
fn sample_row_hit_target_survives_frame_refresh_between_press_and_release() {
    let bounds = Rect::from_size(160.0, 22.0);
    let mut hit_target = sample_hit_target(false, false, false, false);

    assert_eq!(
        hit_target.handle_input(bounds, WidgetInput::primary_press(Point::new(24.0, 10.0)),),
        None
    );

    let mut refreshed_hit_target = sample_hit_target(false, false, false, false);
    refreshed_hit_target.synchronize_from_previous(&hit_target);
    let output = refreshed_hit_target
        .handle_input(
            bounds,
            WidgetInput::pointer_release(
                Point::new(24.0, 10.0),
                PointerButton::Primary,
                PointerModifiers {
                    command: true,
                    shift: true,
                    ..Default::default()
                },
            ),
        )
        .expect("sample row should activate after a frame refresh");

    assert_eq!(
        output.typed_cloned::<crate::native_app::test_support::GuiMessage>(),
        Some(
            crate::native_app::test_support::GuiMessage::SelectSampleWithModifiers {
                path: String::from("sample.wav"),
                modifiers: PointerModifiers {
                    command: true,
                    shift: true,
                    ..Default::default()
                },
            }
        )
    );
    assert!(!refreshed_hit_target.common().is_pressed());
}
