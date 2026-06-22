use super::*;

#[test]
fn sample_row_hit_target_survives_frame_refresh_between_press_and_release() {
    let bounds = Rect::from_size(160.0, 22.0);
    let mut hit_target = sample_hit_target(false, false, false, false);

    assert_eq!(
        sample_hit_target_input(
            &mut hit_target,
            bounds,
            WidgetInput::primary_press(Point::new(24.0, 10.0)),
        ),
        None
    );

    let mut refreshed_hit_target = sample_hit_target(false, false, false, false);
    sync_sample_hit_target_from_previous(&mut refreshed_hit_target, &hit_target);
    let output = sample_hit_target_input(
        &mut refreshed_hit_target,
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
        sample_hit_target_message(&refreshed_hit_target, output),
        Some(
            crate::native_app::test_support::state::GuiMessage::SelectSampleWithModifiers {
                path: String::from("sample.wav"),
                modifiers: PointerModifiers {
                    command: true,
                    shift: true,
                    ..Default::default()
                },
            }
        )
    );
    assert!(
        !sample_hit_target_widget(&refreshed_hit_target)
            .common()
            .is_pressed()
    );
}
