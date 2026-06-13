use super::*;

#[test]
fn audio_settings_snapshot_uses_cached_device_options() {
    let mut state = gui_state_for_span_tests();
    state.audio.hosts = vec![crate::native_app::test_support::audio::AudioHostSummary {
        id: String::from("cached-host"),
        label: String::from("Cached Host"),
        is_default: true,
    }];

    let host_ids = crate::native_app::test_support::settings::audio_settings_host_ids(&state);

    assert_eq!(host_ids, vec![String::from("cached-host")]);
}

#[test]
fn audio_engine_detail_distinguishes_selected_host_from_runtime_fallback() {
    let mut state = gui_state_for_span_tests();
    state.audio.output_config.host = Some(String::from("asio"));
    state.audio.hosts = vec![
        crate::native_app::test_support::audio::AudioHostSummary {
            id: String::from("wasapi"),
            label: String::from("WASAPI"),
            is_default: true,
        },
        crate::native_app::test_support::audio::AudioHostSummary {
            id: String::from("asio"),
            label: String::from("ASIO"),
            is_default: false,
        },
    ];
    state.audio.output_resolved = Some(crate::native_app::test_support::audio::ResolvedOutput {
        host_id: String::from("wasapi"),
        device_name: String::from("Studio"),
        sample_rate: 48_000,
        buffer_size_frames: None,
        channel_count: 2,
        used_fallback: true,
    });

    assert_eq!(
        state.audio_engine_detail_label(),
        "ASIO selected | using WASAPI | Studio | 48 kHz"
    );
}

#[test]
fn audio_engine_pill_prefers_runtime_sample_rate() {
    let mut state = gui_state_for_span_tests();
    state.audio.output_config.sample_rate = Some(44_100);
    state.audio.output_resolved = Some(crate::native_app::test_support::audio::ResolvedOutput {
        host_id: String::from("wasapi"),
        device_name: String::from("Studio"),
        sample_rate: 48_000,
        buffer_size_frames: None,
        channel_count: 2,
        used_fallback: false,
    });

    assert_eq!(state.audio_engine_pill_label(), "48 kHz");
}

#[test]
fn audio_engine_pill_shows_no_audio_before_runtime_resolves() {
    let mut state = gui_state_for_span_tests();
    state.audio.output_config.sample_rate = Some(44_100);

    assert_eq!(state.audio_engine_pill_label(), "no audio");
}

#[test]
fn audio_engine_pill_uses_warning_style_without_runtime_output() {
    let mut state = gui_state_for_span_tests();
    state.audio.output_config.sample_rate = Some(44_100);

    assert_eq!(
        state.audio_engine_pill_style(),
        WidgetStyle::subtle(WidgetTone::Warning)
    );
}

#[test]
fn audio_engine_pill_uses_neutral_style_with_runtime_output() {
    let mut state = gui_state_for_span_tests();
    state.audio.output_resolved = Some(crate::native_app::test_support::audio::ResolvedOutput {
        host_id: String::from("wasapi"),
        device_name: String::from("Studio"),
        sample_rate: 48_000,
        buffer_size_frames: None,
        channel_count: 2,
        used_fallback: false,
    });

    assert_eq!(
        state.audio_engine_pill_style(),
        WidgetStyle::subtle(WidgetTone::Neutral)
    );
}

#[test]
fn audio_sample_rate_label_matches_status_chip_format() {
    assert_eq!(
        crate::native_app::test_support::state::format_sample_rate_label(48_000),
        "48 kHz"
    );
    assert_eq!(
        crate::native_app::test_support::state::format_sample_rate_label(44_100),
        "44.1 kHz"
    );
    assert_eq!(
        crate::native_app::test_support::state::format_sample_rate_label(960),
        "960 Hz"
    );
}
