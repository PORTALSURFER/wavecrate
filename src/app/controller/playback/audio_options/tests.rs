use super::apply::apply_audio_selection_result;
use super::fallback::{audio_input_fallback_message, audio_output_fallback_message};
use super::normalize::NormalizedAudioOptions;
use super::refresh::{apply_audio_input_refresh, apply_audio_output_refresh};
use crate::app::controller::test_support::dummy_controller;
use crate::app::state::StatusTone;
use crate::audio::{
    AudioDeviceSummary, AudioHostSummary, ResolvedInput, ResolvedOutput,
};

fn host(id: &str, is_default: bool) -> AudioHostSummary {
    AudioHostSummary {
        id: id.to_string(),
        label: id.to_string(),
        is_default,
    }
}

fn device(host_id: &str, name: &str, is_default: bool) -> AudioDeviceSummary {
    AudioDeviceSummary {
        host_id: host_id.to_string(),
        name: name.to_string(),
        is_default,
    }
}

fn normalized_audio_options(
    host_id: Option<&str>,
    device_name: Option<&str>,
    sample_rate: Option<u32>,
) -> NormalizedAudioOptions {
    NormalizedAudioOptions {
        host_id: host_id.map(str::to_string),
        device_name: device_name.map(str::to_string),
        sample_rate,
        devices: vec![device(host_id.unwrap_or("default"), device_name.unwrap_or("Built-in"), true)],
        sample_rates: vec![48_000, 96_000],
        warning: None,
    }
}

#[test]
fn output_refresh_without_probe_retains_sample_rates_when_selection_is_unchanged() {
    let (mut controller, _) = dummy_controller();
    controller.settings.audio_output.host = Some("default".into());
    controller.settings.audio_output.device = Some("Built-in".into());
    controller.ui.audio.selected = controller.settings.audio_output.clone();
    controller.ui.audio.sample_rates = vec![44_100];
    let previous = controller.ui.audio.selected.clone();

    apply_audio_output_refresh(
        &mut controller,
        previous,
        vec![host("default", true)],
        normalized_audio_options(Some("default"), Some("Built-in"), Some(48_000)),
        false,
    );

    assert_eq!(controller.ui.audio.sample_rates, vec![44_100]);
    assert_eq!(controller.ui.audio.selected.sample_rate, Some(48_000));
}

#[test]
fn output_refresh_without_probe_clears_sample_rates_when_selection_changes() {
    let (mut controller, _) = dummy_controller();
    controller.ui.audio.selected.host = Some("default".into());
    controller.ui.audio.selected.device = Some("Built-in".into());
    controller.ui.audio.sample_rates = vec![44_100];
    let previous = controller.ui.audio.selected.clone();

    apply_audio_output_refresh(
        &mut controller,
        previous,
        vec![host("alt", true)],
        normalized_audio_options(Some("alt"), Some("USB"), Some(48_000)),
        false,
    );

    assert!(controller.ui.audio.sample_rates.is_empty());
    assert_eq!(controller.ui.audio.selected.host.as_deref(), Some("alt"));
    assert_eq!(controller.ui.audio.selected.device.as_deref(), Some("USB"));
}

#[test]
fn input_refresh_normalizes_unavailable_channels_and_reports_warning() {
    let (mut controller, _) = dummy_controller();
    controller.settings.audio_input.host = Some("default".into());
    controller.settings.audio_input.device = Some("Mic".into());
    controller.settings.audio_input.channels = vec![1, 3];
    controller.ui.audio.input_selected = controller.settings.audio_input.clone();
    let previous = controller.ui.audio.input_selected.clone();

    apply_audio_input_refresh(
        &mut controller,
        previous,
        vec![host("default", true)],
        normalized_audio_options(Some("default"), Some("Mic"), Some(48_000)),
        Ok(2),
        true,
    );

    assert_eq!(controller.settings.audio_input.channels, vec![1]);
    assert_eq!(controller.ui.audio.input_channel_count, 2);
    assert_eq!(
        controller.ui.audio.input_warning.as_deref(),
        Some("Input channels 1, 3 unavailable; using 1")
    );
}

#[test]
fn apply_audio_selection_result_syncs_selected_state_on_success() {
    let (mut controller, _) = dummy_controller();
    controller.settings.audio_output.host = Some("default".into());
    controller.settings.audio_output.device = Some("Built-in".into());
    controller.settings.audio_output.sample_rate = Some(48_000);

    apply_audio_selection_result(&mut controller, Ok(()));

    assert_eq!(controller.ui.audio.selected, controller.settings.audio_output);
    assert_eq!(controller.ui.status.status_tone, StatusTone::Idle);
}

#[test]
fn apply_audio_selection_result_reports_rebuild_failures() {
    let (mut controller, _) = dummy_controller();

    apply_audio_selection_result(&mut controller, Err("Audio unavailable".into()));

    assert_eq!(controller.ui.status.text, "Audio unavailable");
    assert_eq!(controller.ui.status.status_tone, StatusTone::Error);
}

#[test]
fn output_fallback_message_lists_unavailable_requested_settings() {
    let (mut controller, _) = dummy_controller();
    controller.settings.audio_output.host = Some("wasapi".into());
    controller.settings.audio_output.device = Some("Studio".into());
    controller.settings.audio_output.sample_rate = Some(96_000);
    controller.settings.audio_output.buffer_size = Some(512);

    let message = audio_output_fallback_message(
        &controller.settings.audio_output,
        &ResolvedOutput {
            host_id: "asio".into(),
            device_name: "Built-in".into(),
            sample_rate: 48_000,
            buffer_size_frames: Some(256),
            channel_count: 2,
            used_fallback: true,
        },
    );

    assert_eq!(
        message.as_deref(),
        Some("Using Built-in via asio (host wasapi, device Studio, sample rate 96000, buffer 512 unavailable)")
    );
}

#[test]
fn input_fallback_message_lists_requested_channels_when_downmixed() {
    let (mut controller, _) = dummy_controller();
    controller.settings.audio_input.host = Some("wasapi".into());
    controller.settings.audio_input.device = Some("Mic".into());
    controller.settings.audio_input.sample_rate = Some(48_000);
    controller.settings.audio_input.channels = vec![1, 2];

    let message = audio_input_fallback_message(
        &controller.settings.audio_input,
        &ResolvedInput {
            host_id: "asio".into(),
            device_name: "USB".into(),
            sample_rate: 44_100,
            buffer_size_frames: Some(256),
            stream_channel_count: 2,
            recorded_channel_count: 1,
            selected_channels: vec![1],
            used_fallback: true,
        },
    );

    assert_eq!(
        message.as_deref(),
        Some("Using USB via asio (host wasapi, device Mic, sample rate 48000, inputs 1, 2 unavailable)")
    );
}
