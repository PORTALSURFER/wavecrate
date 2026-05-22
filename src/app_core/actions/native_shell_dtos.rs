//! Wavecrate-owned native shell projection DTOs.
//!
//! These models describe Wavecrate application state as projected for the current
//! native shell. The runtime adapter in `gui_runtime` converts these app-core
//! DTOs into the Wavecrate-owned native runtime contract consumed by Radiant.

use radiant::gui::frame;
use radiant::gui::range;
use radiant::gui::retained;

mod app_shell;
mod audio_options;
mod automation;
mod browser;
mod motion;
mod retained_segments;
mod sidebar;
mod waveform;

pub use self::app_shell::{
    AppModel, ColumnModel, ConfirmPromptKind, ConfirmPromptModel, DragOverlayModel,
    FocusContextModel, MapPanelModel, MapPointModel, MapRenderModeModel, ProgressOverlayModel,
    StatusBarModel, UpdatePanelModel, UpdateStatusModel,
};
pub use self::audio_options::{
    AudioEngineChipStateModel, AudioEngineModel, AudioFieldModel, AudioOptionItemModel,
    AudioOptionValueModel, AudioPickerTargetModel, OptionsPanelModel,
};
pub use self::automation::{
    AutomationBounds, AutomationNodeId, AutomationNodeSnapshot, AutomationRole,
    GuiAutomationSnapshot,
};
pub use self::browser::{
    BrowserActionsModel, BrowserChromeModel, BrowserPanelModel, BrowserRowModel,
    BrowserRowProcessingState, BrowserTagPillModel, BrowserTagSidebarModel, BrowserTagState,
    PlaybackAgeBucket, PlaybackAgeFilterChip,
};
pub use self::motion::NativeMotionModel;
pub use self::retained_segments::{DirtySegments, SegmentRevisions};
pub use self::sidebar::{
    folder_row_model, FolderActionsModel, FolderPaneIdModel, FolderPaneModel, FolderRecoveryModel,
    FolderRowKind, FolderRowModel, SourceRowModel, SourcesPanelModel,
};
pub use self::waveform::{
    parse_waveform_tempo_number_text, WaveformChannelViewModel, WaveformChromeModel,
    WaveformChromeStateModel, WaveformEditPreviewModel, WaveformFeedbackEventsModel,
    WaveformImagePreviewModel, WaveformMotionModel, WaveformPanelModel, WaveformPresentationModel,
    WaveformSlicePreviewModel, WaveformSurfaceModel, WaveformToolStateModel,
    WaveformTransportModel, WaveformViewportModel,
};

/// Shared storage used by retained app-model snapshots.
pub type RetainedVec<T> = retained::RetainedVec<T>;

/// Frame-level feedback from renderer to host bridge.
pub type FrameBuildResult = frame::FrameBuildResult;

/// Normalized interval with deterministic milli, micro, and nano projections.
pub type NormalizedRangeModel = range::NormalizedRange;

#[cfg(test)]
mod tests {
    use super::{parse_waveform_tempo_number_text, WaveformPanelModel};

    #[test]
    fn waveform_panel_default_bpm_grid_origin_is_zero() {
        assert_eq!(WaveformPanelModel::default().bpm_grid_origin_micros, 0);
    }

    #[test]
    fn options_panel_projects_generic_preference_state() {
        let model = super::OptionsPanelModel {
            visible: true,
            default_identifier: String::from("portal"),
            input_monitoring_enabled: true,
            advance_after_rating_enabled: false,
            destructive_yolo_mode_enabled: true,
            invert_waveform_scroll_enabled: false,
            trash_folder_label: Some(String::from("Trash")),
            audio_write_format_label: Some(String::from("Source rate, 32-bit float")),
        };
        let preferences = model.preference_state();

        assert!(preferences.visible);
        assert_eq!(preferences.primary_text_value, "portal");
        assert_eq!(preferences.toggles, [true, false, true, false]);
        assert_eq!(preferences.auxiliary_label.as_deref(), Some("Trash"));
    }

    #[test]
    fn waveform_panel_projects_generic_feedback_events() {
        let model = WaveformPanelModel {
            selection_export_flash_nonce: 11,
            selection_export_failure_flash_nonce: 12,
            edit_selection_apply_flash_nonce: 13,
            ..WaveformPanelModel::default()
        };
        let events = model.feedback_events();

        assert_eq!(events.primary_success_nonce, 11);
        assert_eq!(events.primary_failure_nonce, 12);
        assert_eq!(events.secondary_success_nonce, 13);
    }

    #[test]
    fn waveform_panel_projects_generic_presentation_state() {
        let model = WaveformPanelModel {
            beat_step_micros: Some(100_000),
            bpm_grid_origin_micros: 50_000,
            loop_enabled: true,
            tempo_label: Some(String::from("150 BPM")),
            zoom_label: Some(String::from("8x")),
            ..WaveformPanelModel::default()
        };
        let presentation = model.presentation();

        assert_eq!(presentation.guide_step_micros, Some(100_000));
        assert_eq!(presentation.guide_origin_micros, 50_000);
        assert!(presentation.repeat_enabled);
        assert_eq!(presentation.primary_label.as_deref(), Some("150 BPM"));
        assert_eq!(presentation.viewport_label.as_deref(), Some("8x"));
    }

    #[test]
    fn waveform_panel_projects_generic_timeline_surface_state() {
        let model = WaveformPanelModel {
            view_start_micros: 125_000,
            playhead_micros: Some(250_250),
            selection_export_failure_flash_nonce: 5,
            loop_enabled: true,
            loaded_label: Some(String::from("Loaded")),
            slices: vec![super::WaveformSlicePreviewModel {
                range: super::NormalizedRangeModel::new(100, 200),
                selected: true,
                focused: false,
                marked_for_export: false,
                review_candidate: false,
                review_exempted: false,
            }],
            ..WaveformPanelModel::default()
        };
        let surface = model.timeline_surface();

        assert_eq!(surface.viewport.start_micros, 125_000);
        assert_eq!(surface.transport.resolved_playhead_micros(), Some(250_250));
        assert_eq!(surface.feedback_events.primary_failure_nonce, 5);
        assert!(surface.presentation.repeat_enabled);
        assert_eq!(
            surface.raster_preview.loaded_label.as_deref(),
            Some("Loaded")
        );
        assert_eq!(surface.markers.len(), 1);
    }

    #[test]
    fn native_motion_projects_generic_timeline_motion_state() {
        let model = super::NativeMotionModel {
            transport_running: true,
            map_active: false,
            active_rating_filters: [false; 8],
            active_playback_age_filters: [false; 3],
            marked_filter_active: false,
            waveform_selection_milli: Some(super::NormalizedRangeModel::new(100, 400)),
            waveform_slices: Vec::new(),
            waveform_selection_export_flash_nonce: 11,
            waveform_selection_export_failure_flash_nonce: 12,
            waveform_edit_selection_apply_flash_nonce: 13,
            waveform_edit_selection_milli: None,
            waveform_edit_fade_in_end_milli: Some(120),
            waveform_edit_fade_in_end_micros: Some(120_000),
            waveform_edit_fade_in_mute_start_milli: None,
            waveform_edit_fade_in_mute_start_micros: None,
            waveform_edit_fade_in_curve_milli: Some(200),
            waveform_edit_fade_out_start_milli: None,
            waveform_edit_fade_out_start_micros: None,
            waveform_edit_fade_out_mute_end_milli: Some(390),
            waveform_edit_fade_out_mute_end_micros: Some(390_000),
            waveform_edit_fade_out_curve_milli: Some(800),
            waveform_loop_enabled: true,
            waveform_loop_lock_enabled: true,
            waveform_cursor_milli: Some(150),
            waveform_playhead_milli: Some(250),
            waveform_playhead_micros: Some(250_500),
            waveform_view_start_milli: 10,
            waveform_view_end_milli: 900,
            waveform_view_start_micros: 10_000,
            waveform_view_end_micros: 900_000,
            waveform_view_start_nanos: 10_000_000,
            waveform_view_end_nanos: 900_000_000,
            waveform_tempo_label: Some(String::from("128 BPM")),
            waveform_zoom_label: Some(String::from("4x")),
            waveform_loaded_label: Some(String::from("Loaded")),
            waveform_loading: true,
            waveform_image_signature: Some(42),
            waveform_transport_hint: String::from("playing"),
            waveform_compare_anchor_available: true,
            waveform_compare_anchor_label: Some(String::from("A")),
            waveform_channel_view: super::WaveformChannelViewModel::Stereo,
            waveform_normalized_audition_enabled: true,
            waveform_bpm_snap_enabled: true,
            waveform_relative_bpm_grid_enabled: false,
            waveform_transient_snap_enabled: true,
            waveform_transient_markers_enabled: true,
            waveform_slice_mode_enabled: false,
            waveform_exact_duplicate_cleanup_available: true,
            status_right: String::from("ready"),
        };

        let motion = model.timeline_motion();

        assert!(motion.transport_running);
        assert_eq!(motion.surface.viewport.start_micros, 10_000);
        assert_eq!(
            motion.surface.transport.resolved_playhead_micros(),
            Some(250_500)
        );
        assert_eq!(motion.surface.feedback_events.primary_success_nonce, 11);
        assert!(motion.surface.presentation.repeat_enabled);
        assert_eq!(
            motion.surface.raster_preview.loaded_label.as_deref(),
            Some("Loaded")
        );
        assert_eq!(motion.chrome.status_hint, "playing");
        assert_eq!(
            motion.chrome.channel_view,
            super::WaveformChannelViewModel::Stereo
        );
        assert!(motion.tools.lock_enabled);
        assert!(motion.tools.cleanup_available);
    }

    #[test]
    fn parse_waveform_tempo_number_text_accepts_integer_and_fractional_labels() {
        assert_eq!(
            parse_waveform_tempo_number_text("128 BPM"),
            Some(String::from("128"))
        );
        assert_eq!(
            parse_waveform_tempo_number_text("128.5 BPM"),
            Some(String::from("128.5"))
        );
    }

    #[test]
    fn parse_waveform_tempo_number_text_rejects_empty_and_invalid_labels() {
        assert_eq!(parse_waveform_tempo_number_text(""), None);
        assert_eq!(parse_waveform_tempo_number_text("0 BPM"), None);
        assert_eq!(parse_waveform_tempo_number_text("-1 BPM"), None);
        assert_eq!(parse_waveform_tempo_number_text("fast BPM"), None);
    }
}
