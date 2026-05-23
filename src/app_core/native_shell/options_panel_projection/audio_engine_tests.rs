use super::*;
use crate::app_core::app_api::state::{ActiveAudioOutput, AudioDeviceView, AudioHostView};

#[test]
fn audio_engine_projection_reports_healthy_chip_from_applied_output() {
    let mut ui = UiState::default();
    ui.audio.selected.host = Some(String::from("asio"));
    ui.audio.selected.device = Some(String::from("Studio"));
    ui.audio.selected.sample_rate = Some(48_000);
    ui.audio.applied = Some(ActiveAudioOutput {
        host_id: String::from("asio"),
        device_name: String::from("Studio"),
        sample_rate: 48_000,
        buffer_size_frames: Some(256),
        channel_count: 2,
    });

    let projected = project_audio_engine_model(&ui);

    assert_eq!(
        projected.chip_state,
        crate::app_core::actions::NativeAudioEngineChipStateModel::Healthy
    );
    assert_eq!(projected.chip_label, "48 kHz");
    assert_eq!(projected.detail_label, None);
    assert_eq!(projected.output_host.value_label, "asio");
    assert_eq!(projected.output_device.value_label, "Studio");
    assert_eq!(projected.output_sample_rate.value_label, "48 kHz");
}

#[test]
fn audio_engine_projection_reports_error_detail_picker_and_options() {
    let mut ui = UiState::default();
    ui.audio.output_runtime_error = Some(String::from("USB device disconnected"));
    ui.audio.selected.host = Some(String::from("asio"));
    ui.audio.selected.device = Some(String::from("USB"));
    ui.audio.selected.sample_rate = Some(44_100);
    ui.audio.hosts.push(AudioHostView {
        id: String::from("asio"),
        label: String::from("ASIO"),
        is_default: true,
    });
    ui.audio.devices.push(AudioDeviceView {
        host_id: String::from("asio"),
        name: String::from("USB"),
        is_default: true,
    });
    ui.audio.sample_rates = vec![44_100, 48_000];
    ui.options_panel.active_audio_picker = Some(AudioPickerTarget::OutputSampleRate);

    let projected = project_audio_engine_model(&ui);

    assert_eq!(
        projected.chip_state,
        crate::app_core::actions::NativeAudioEngineChipStateModel::Error
    );
    assert_eq!(projected.chip_label, "Audio Err");
    assert_eq!(
        projected.detail_label.as_deref(),
        Some("USB device disconnected")
    );
    assert_eq!(
        projected.active_picker,
        Some(crate::app_core::actions::NativeAudioPickerTargetModel::OutputSampleRate)
    );
    assert_eq!(projected.output_host_options.len(), 2);
    assert_eq!(projected.output_device_options.len(), 2);
    assert_eq!(projected.output_sample_rate_options.len(), 3);
    assert!(projected.output_sample_rate_options[1].selected);
    assert_eq!(projected.output_sample_rate_options[1].label, "44.1 kHz");
}

#[test]
fn audio_engine_projection_surfaces_output_warning_without_error_chip() {
    let mut ui = UiState::default();
    ui.audio.selected.host = Some(String::from("asio"));
    ui.audio.selected.device = Some(String::from("Studio"));
    ui.audio.selected.sample_rate = Some(96_000);
    ui.audio.applied = Some(ActiveAudioOutput {
        host_id: String::from("asio"),
        device_name: String::from("Studio"),
        sample_rate: 48_000,
        buffer_size_frames: Some(256),
        channel_count: 2,
    });
    ui.audio.warning = Some(String::from(
        "Using Studio via asio (sample rate 96000 unavailable)",
    ));

    let projected = project_audio_engine_model(&ui);

    assert_eq!(
        projected.chip_state,
        crate::app_core::actions::NativeAudioEngineChipStateModel::Healthy
    );
    assert_eq!(projected.chip_label, "48 kHz");
    assert_eq!(
        projected.detail_label.as_deref(),
        Some("Using Studio via asio (sample rate 96000 unavailable)")
    );
}

#[test]
fn audio_engine_projection_reports_generic_mismatch_without_warning() {
    let mut ui = UiState::default();
    ui.audio.selected.host = Some(String::from("asio"));
    ui.audio.selected.device = Some(String::from("Studio"));
    ui.audio.selected.sample_rate = Some(96_000);
    ui.audio.applied = Some(ActiveAudioOutput {
        host_id: String::from("asio"),
        device_name: String::from("Studio"),
        sample_rate: 48_000,
        buffer_size_frames: Some(256),
        channel_count: 2,
    });

    let projected = project_audio_engine_model(&ui);

    assert_eq!(
        projected.chip_state,
        crate::app_core::actions::NativeAudioEngineChipStateModel::Healthy
    );
    assert_eq!(
        projected.detail_label.as_deref(),
        Some("Selected output differs from the active engine")
    );
    assert_eq!(projected.output_host.value_label, "asio");
    assert_eq!(projected.output_device.value_label, "Studio");
    assert_eq!(projected.output_sample_rate.value_label, "96 kHz");
}
