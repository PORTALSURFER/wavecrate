use super::*;

#[test]
fn transaction_list_modal_blocks_waveform_interaction_behind_it() {
    let mut state = gui_state_for_span_tests();
    state.ui.chrome.transaction_list_open = true;
    let mut runtime = native_runtime_for_tests(state, Vector2::new(900.0, 620.0));
    let rect = waveform_rect(&runtime);
    let point = Point::new(rect.min.x + rect.width() * 0.25, rect.center().y);

    assert_ne!(
        runtime.widget_at(point),
        Some(crate::native_app::test_support::WAVEFORM_WIDGET_ID)
    );
    assert_ne!(
        runtime.dispatch_event(Event::primary_press(point)),
        Some(crate::native_app::test_support::WAVEFORM_WIDGET_ID)
    );

    assert_eq!(
        runtime.bridge().state().waveform.current.play_mark_ratio(),
        None
    );
    assert_eq!(
        runtime.bridge().state().waveform.current.play_selection(),
        None
    );
}
