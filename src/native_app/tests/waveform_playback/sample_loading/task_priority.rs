use super::*;

#[test]
fn keyboard_and_mouse_uncached_selection_use_same_fast_debounce() {
    assert_eq!(
        crate::native_app::test_support::waveform::KEYBOARD_SAMPLE_LOAD_DEBOUNCE,
        crate::native_app::test_support::waveform::UNCACHED_SAMPLE_LOAD_DEBOUNCE,
        "keyboard navigation should not wait longer than mouse selection before audition loading"
    );
}

#[test]
fn uncached_selected_sample_load_uses_foreground_priority() {
    assert_eq!(
        crate::native_app::audio::sample_load_actions::foreground_sample_load_priority(),
        ui::TaskPriority::Interactive,
        "selected uncached audition loads must outrank background cache warming"
    );
}

#[test]
fn active_folder_cache_warm_uses_lower_priority_than_selected_sample_load() {
    assert_eq!(
        crate::native_app::audio::sample_load_actions::active_folder_cache_warm_priority(),
        ui::TaskPriority::BlockingIo
    );
    assert_ne!(
        crate::native_app::audio::sample_load_actions::foreground_sample_load_priority(),
        crate::native_app::audio::sample_load_actions::active_folder_cache_warm_priority(),
        "background folder warming must not share the foreground audition lane"
    );
}

#[test]
fn frame_queues_audio_output_warm_up_before_explicit_playback() {
    let mut state = gui_state_for_span_tests();
    assert!(state.audio.player.is_none());
    assert!(state.background.audio_open.active().is_none());

    let mut context = ui::UiUpdateContext::default();
    state.apply_message(
        crate::native_app::test_support::state::GuiMessage::Frame,
        &mut context,
    );

    assert!(
        state.background.audio_open.active().is_some(),
        "frame processing should begin audio output warm-up before the first explicit playback"
    );
}
