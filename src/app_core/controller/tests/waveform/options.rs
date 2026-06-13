use super::*;
use crate::app::controller::{
    startup_audio_refresh_count_for_tests, with_stubbed_startup_audio_refresh_for_tests,
};
use crate::app_core::actions::NativeOptionsAction;

#[test]
fn apply_ui_waveform_smart_scale_routes_to_controller_behavior() {
    let mut controller = AppController::new(WaveformRenderer::new(16, 16), None);
    controller.set_loaded_audio_duration_for_tests(4.0);
    controller.set_selection_range(crate::selection::SelectionRange::new(0.0, 0.25));
    controller.set_bpm_value(150.0);

    controller.apply_ui_action(NativeUiAction::Waveform(
        crate::app_core::actions::NativeWaveformAction::SetWaveformSelectionRangeSmartScale {
            start_micros: 0,
            end_micros: 500_000,
        },
    ));

    assert_eq!(
        controller.ui.waveform.selection,
        Some(crate::selection::SelectionRange::new(0.0, 0.5))
    );
    assert_eq!(controller.ui.waveform.bpm_value, Some(120.0));
    assert!((controller.settings.controls.bpm_value - 150.0).abs() < f32::EPSILON);
    assert!(controller.is_selection_dragging());

    controller.apply_ui_action(NativeUiAction::Waveform(
        crate::app_core::actions::NativeWaveformAction::FinishWaveformSelectionSmartScaleDrag,
    ));

    assert!(!controller.is_selection_dragging());
    assert!((controller.settings.controls.bpm_value - 120.0).abs() < 0.1);
}

#[test]
/// Waveform toolbar option actions should update controller waveform state.
fn apply_ui_waveform_option_actions_update_waveform_state() {
    let mut controller = AppController::new(WaveformRenderer::new(16, 16), None);

    controller.apply_ui_action(NativeUiAction::Options(
        crate::app_core::actions::NativeOptionsAction::SetWaveformChannelView { stereo: true },
    ));
    assert_eq!(
        controller.ui.waveform.channel_view,
        WaveformChannelView::SplitStereo
    );

    controller.apply_ui_action(NativeUiAction::Options(
        crate::app_core::actions::NativeOptionsAction::SetNormalizedAuditionEnabled {
            enabled: true,
        },
    ));
    assert!(controller.ui.waveform.normalized_audition_enabled);

    controller.ui.waveform.bpm_value = Some(120.0);
    controller.apply_ui_action(NativeUiAction::Options(
        crate::app_core::actions::NativeOptionsAction::AdjustWaveformBpm { delta: 1 },
    ));
    assert_eq!(controller.ui.waveform.bpm_value, Some(121.0));
    controller.apply_ui_action(NativeUiAction::Options(
        crate::app_core::actions::NativeOptionsAction::SetWaveformBpmValue { value_tenths: 1275 },
    ));
    assert_eq!(controller.ui.waveform.bpm_value, Some(127.5));

    controller.apply_ui_action(NativeUiAction::Options(
        crate::app_core::actions::NativeOptionsAction::SetBpmSnapEnabled { enabled: true },
    ));
    assert!(controller.ui.waveform.bpm_snap_enabled);

    controller.apply_ui_action(NativeUiAction::Options(
        crate::app_core::actions::NativeOptionsAction::SetRelativeBpmGridEnabled { enabled: true },
    ));
    assert!(controller.ui.waveform.relative_bpm_grid_enabled);

    controller.apply_ui_action(NativeUiAction::Options(
        crate::app_core::actions::NativeOptionsAction::SetTransientSnapEnabled { enabled: true },
    ));
    assert!(controller.ui.waveform.transient_snap_enabled);

    controller.apply_ui_action(NativeUiAction::Options(
        crate::app_core::actions::NativeOptionsAction::SetTransientMarkersEnabled {
            enabled: false,
        },
    ));
    assert!(!controller.ui.waveform.transient_markers_enabled);
    assert!(!controller.ui.waveform.transient_snap_enabled);

    controller.ui.waveform.selected_slices = vec![0, 1];
    controller.apply_ui_action(NativeUiAction::Options(
        crate::app_core::actions::NativeOptionsAction::SetSliceModeEnabled { enabled: true },
    ));
    assert!(controller.ui.waveform.slice_mode_enabled);

    controller.ui.waveform.slices = vec![
        crate::selection::SelectionRange::new(0.1, 0.2),
        crate::selection::SelectionRange::new(0.3, 0.4),
    ];
    controller.ui.waveform.selected_slices.clear();
    controller.apply_ui_action(NativeUiAction::PromptsAndEdits(
        crate::app_core::actions::NativePromptEditAction::ToggleWaveformSliceSelection { index: 1 },
    ));
    assert_eq!(controller.ui.waveform.selected_slices, vec![1]);
    controller.start_slice_review();
    controller.apply_ui_action(NativeUiAction::PromptsAndEdits(
        crate::app_core::actions::NativePromptEditAction::MoveWaveformSliceFocus { delta: 1 },
    ));
    assert_eq!(controller.ui.waveform.slice_review.focused_index, Some(1));
    controller.apply_ui_action(NativeUiAction::PromptsAndEdits(
        crate::app_core::actions::NativePromptEditAction::ToggleFocusedWaveformSliceExportMark,
    ));
    assert_eq!(controller.ui.waveform.slice_review.marked_indices, vec![1]);

    controller.apply_ui_action(NativeUiAction::Options(
        crate::app_core::actions::NativeOptionsAction::SetSliceModeEnabled { enabled: false },
    ));
    assert!(!controller.ui.waveform.slice_mode_enabled);
    assert!(controller.ui.waveform.selected_slices.is_empty());
    assert_eq!(
        controller.ui.waveform.slice_review,
        WaveformSliceReviewState::default()
    );
}

#[test]
fn handle_escape_exits_slice_review_before_clearing_slice_batch() {
    let mut controller = AppController::new(WaveformRenderer::new(16, 16), None);
    controller.ui.waveform.slices = vec![
        crate::selection::SelectionRange::new(0.1, 0.2),
        crate::selection::SelectionRange::new(0.3, 0.4),
    ];
    controller.start_slice_review();

    controller.apply_ui_action(NativeUiAction::Transport(
        crate::app_core::actions::NativeTransportAction::HandleEscape,
    ));

    assert!(!controller.ui.waveform.slice_review.active);
    assert_eq!(controller.ui.waveform.slices.len(), 2);
}

#[test]
fn duplicate_preview_actions_focus_audition_and_toggle_exemption() {
    let mut controller = AppController::new(WaveformRenderer::new(16, 16), None);
    let range = crate::selection::SelectionRange::new(0.1, 0.2);
    controller.ui.waveform.slice_batch_profile = WaveformSliceBatchProfile::ExactDuplicateBeats;
    controller.ui.waveform.slices = vec![range];
    controller.ui.waveform.duplicate_cleanup = Some(WaveformDuplicateCleanupState {
        group_count: 1,
        previews: vec![WaveformDuplicateCleanupPreview {
            range,
            group_id: 0,
            exempted: false,
            represented_window_count: 1,
        }],
    });
    controller.ui.waveform.slice_batch_beat_count = 1;

    controller.apply_ui_action(NativeUiAction::PromptsAndEdits(
        crate::app_core::actions::NativePromptEditAction::AuditionWaveformDuplicateSlice {
            index: 0,
        },
    ));
    assert_eq!(controller.ui.waveform.slice_review.focused_index, Some(0));

    controller.apply_ui_action(NativeUiAction::PromptsAndEdits(
        crate::app_core::actions::NativePromptEditAction::ToggleWaveformDuplicateSliceExemption {
            index: 0,
        },
    ));
    assert_eq!(controller.ui.waveform.slice_batch_beat_count, 0);
    assert!(
        controller
            .ui
            .waveform
            .duplicate_cleanup
            .as_ref()
            .is_some_and(|state| state.previews[0].exempted)
    );
}

#[test]
/// Native options panel actions should update UI settings state.
fn apply_ui_options_panel_actions_update_ui_state() {
    let mut controller = AppController::new(WaveformRenderer::new(16, 16), None);

    controller.apply_ui_action(NativeUiAction::Options(
        NativeOptionsAction::OpenOptionsMenu,
    ));
    assert!(controller.ui.options_panel.open);

    controller.apply_ui_action(NativeUiAction::Options(
        NativeOptionsAction::SetAdvanceAfterRatingEnabled { enabled: false },
    ));
    assert!(!controller.ui.controls.advance_after_rating);

    controller.apply_ui_action(NativeUiAction::Options(
        NativeOptionsAction::SetDestructiveYoloMode { enabled: true },
    ));
    assert!(controller.ui.controls.destructive_yolo_mode);

    controller.apply_ui_action(NativeUiAction::Options(
        NativeOptionsAction::SetInvertWaveformScroll { enabled: false },
    ));
    assert!(!controller.ui.controls.invert_waveform_scroll);

    controller.apply_ui_action(NativeUiAction::Options(
        NativeOptionsAction::CloseOptionsPanel,
    ));
    assert!(!controller.ui.options_panel.open);
}

#[test]
fn edit_default_identifier_prompt_updates_setting_and_ui_projection_state() {
    let mut controller = AppController::new(WaveformRenderer::new(16, 16), None);

    controller.apply_ui_action(NativeUiAction::Options(
        NativeOptionsAction::EditDefaultIdentifier,
    ));
    controller.set_active_prompt_input(String::from("Artist One"));
    controller.confirm_active_prompt_action();

    assert_eq!(controller.settings.default_identifier, "Artist One");
    assert_eq!(controller.ui.options_panel.default_identifier, "Artist One");
    assert!(controller.ui.options_panel.pending_prompt.is_none());
}

#[test]
fn open_options_menu_flushes_deferred_startup_audio_refresh_once() {
    with_stubbed_startup_audio_refresh_for_tests(|| {
        let mut controller = AppController::new(WaveformRenderer::new(16, 16), None);
        controller
            .apply_configuration(crate::sample_sources::config::AppConfig::default())
            .expect("apply startup config");

        controller.apply_ui_action(NativeUiAction::Options(
            NativeOptionsAction::OpenOptionsMenu,
        ));
        assert!(controller.ui.options_panel.open);
        assert_eq!(startup_audio_refresh_count_for_tests(), 1);
        assert!(!controller.has_pending_startup_audio_refresh());

        controller.open_options_panel();
        assert_eq!(startup_audio_refresh_count_for_tests(), 1);
    });
}

#[test]
fn audio_picker_actions_update_picker_state_and_return_to_overview() {
    let mut controller = AppController::new(WaveformRenderer::new(16, 16), None);

    controller.apply_ui_action(NativeUiAction::Options(
        NativeOptionsAction::OpenOptionsMenu,
    ));
    controller.apply_ui_action(NativeUiAction::Options(
        NativeOptionsAction::OpenAudioOutputSampleRatePicker,
    ));
    assert_eq!(
        controller.ui.options_panel.active_audio_picker,
        Some(AudioPickerTarget::OutputSampleRate)
    );

    controller.settings.audio_output.sample_rate = Some(48_000);
    controller.ui.audio.selected.sample_rate = Some(48_000);
    controller.apply_ui_action(NativeUiAction::Options(
        NativeOptionsAction::SetAudioOutputSampleRate {
            sample_rate: Some(48_000),
        },
    ));
    assert_eq!(controller.ui.options_panel.active_audio_picker, None);

    controller.apply_ui_action(NativeUiAction::Options(
        NativeOptionsAction::OpenAudioInputSampleRatePicker,
    ));
    assert_eq!(
        controller.ui.options_panel.active_audio_picker,
        Some(AudioPickerTarget::InputSampleRate)
    );

    controller.apply_ui_action(NativeUiAction::Options(
        NativeOptionsAction::SetAudioInputSampleRate {
            sample_rate: Some(44_100),
        },
    ));
    assert_eq!(controller.settings.audio_input.sample_rate, Some(44_100));
    assert_eq!(controller.ui.audio.input_selected.sample_rate, Some(44_100));
    assert_eq!(controller.ui.options_panel.active_audio_picker, None);
}
