use super::units::{normalized_to_micros, normalized_to_milli};
use super::*;

/// Projected edit-fade overlay endpoints and curve values for the waveform panel.
pub(in crate::app_core::ui_projection) struct WaveformEditFadeOverlayModel {
    /// End position of the fade-in ramp within the edit selection.
    pub(in crate::app_core::ui_projection) fade_in_end_milli: Option<u16>,
    /// End position of the fade-in ramp within the edit selection at micro precision.
    pub(in crate::app_core::ui_projection) fade_in_end_micros: Option<u32>,
    /// Start position of the fade-in mute segment before the edit selection.
    pub(in crate::app_core::ui_projection) fade_in_mute_start_milli: Option<u16>,
    /// Start position of the fade-in mute segment before the edit selection at micro precision.
    pub(in crate::app_core::ui_projection) fade_in_mute_start_micros: Option<u32>,
    /// Fade-in curve amount mapped into milli-space.
    pub(in crate::app_core::ui_projection) fade_in_curve_milli: Option<u16>,
    /// Start position of the fade-out ramp within the edit selection.
    pub(in crate::app_core::ui_projection) fade_out_start_milli: Option<u16>,
    /// Start position of the fade-out ramp within the edit selection at micro precision.
    pub(in crate::app_core::ui_projection) fade_out_start_micros: Option<u32>,
    /// End position of the fade-out mute segment after the edit selection.
    pub(in crate::app_core::ui_projection) fade_out_mute_end_milli: Option<u16>,
    /// End position of the fade-out mute segment after the edit selection at micro precision.
    pub(in crate::app_core::ui_projection) fade_out_mute_end_micros: Option<u32>,
    /// Fade-out curve amount mapped into milli-space.
    pub(in crate::app_core::ui_projection) fade_out_curve_milli: Option<u16>,
}

impl WaveformEditFadeOverlayModel {
    fn empty() -> Self {
        Self {
            fade_in_end_milli: None,
            fade_in_end_micros: None,
            fade_in_mute_start_milli: None,
            fade_in_mute_start_micros: None,
            fade_in_curve_milli: None,
            fade_out_start_milli: None,
            fade_out_start_micros: None,
            fade_out_mute_end_milli: None,
            fade_out_mute_end_micros: None,
            fade_out_curve_milli: None,
        }
    }
}

/// Project edit fade-handle positions into normalized milli and micro space.
pub(in crate::app_core::ui_projection) fn project_waveform_edit_fade_overlay_model(
    ui: &UiState,
) -> WaveformEditFadeOverlayModel {
    ui.waveform
        .edit_selection
        .map(|selection| {
            let start = selection.start();
            let end = selection.end();
            let width = selection.width();
            if width <= 0.0 {
                return WaveformEditFadeOverlayModel::empty();
            }
            let fade_in_end = selection
                .fade_in()
                .map(|fade| (start + (width * fade.length)).clamp(start, end));
            let fade_in_mute_start = selection
                .fade_in()
                .map(|fade| (start - (width * fade.mute)).clamp(0.0, start));
            let fade_out_start = selection
                .fade_out()
                .map(|fade| (end - (width * fade.length)).clamp(start, end));
            let fade_out_mute_end = selection
                .fade_out()
                .map(|fade| (end + (width * fade.mute)).clamp(end, 1.0));
            let fade_in_end_milli = selection
                .fade_in()
                .map(|fade| normalized_to_milli((start + (width * fade.length)).clamp(start, end)));
            let fade_in_mute_start_milli = selection
                .fade_in()
                .map(|fade| normalized_to_milli((start - (width * fade.mute)).clamp(0.0, start)));
            let fade_in_curve_milli = selection
                .fade_in()
                .map(|fade| normalized_to_milli(fade.curve));
            let fade_out_start_milli = selection
                .fade_out()
                .map(|fade| normalized_to_milli((end - (width * fade.length)).clamp(start, end)));
            let fade_out_mute_end_milli = selection
                .fade_out()
                .map(|fade| normalized_to_milli((end + (width * fade.mute)).clamp(end, 1.0)));
            let fade_out_curve_milli = selection
                .fade_out()
                .map(|fade| normalized_to_milli(fade.curve));
            WaveformEditFadeOverlayModel {
                fade_in_end_milli,
                fade_in_end_micros: fade_in_end.map(normalized_to_micros),
                fade_in_mute_start_milli,
                fade_in_mute_start_micros: fade_in_mute_start.map(normalized_to_micros),
                fade_in_curve_milli,
                fade_out_start_milli,
                fade_out_start_micros: fade_out_start.map(normalized_to_micros),
                fade_out_mute_end_milli,
                fade_out_mute_end_micros: fade_out_mute_end.map(normalized_to_micros),
                fade_out_curve_milli,
            }
        })
        .unwrap_or_else(WaveformEditFadeOverlayModel::empty)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fade_overlay_clamps_mute_handles_to_waveform_bounds() {
        let mut ui = UiState::default();
        ui.waveform.edit_selection = Some(
            crate::selection::SelectionRange::new(0.05, 0.95)
                .with_fade_in(0.5, 0.5)
                .with_fade_in_mute(1.0)
                .with_fade_out(0.5, 0.5)
                .with_fade_out_mute(1.0),
        );

        let overlay = project_waveform_edit_fade_overlay_model(&ui);

        assert_eq!(overlay.fade_in_mute_start_micros, Some(0));
        assert_eq!(overlay.fade_out_mute_end_micros, Some(1_000_000));
        assert_eq!(overlay.fade_in_end_milli, Some(500));
        assert_eq!(overlay.fade_out_start_milli, Some(500));
    }
}
