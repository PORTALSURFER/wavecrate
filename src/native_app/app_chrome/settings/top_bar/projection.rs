use radiant::prelude as ui;

use crate::native_app::app::{AppSettingsTab, NativeAppState};

pub(super) const VOLUME_SLIDER_TOOLTIP: &str = "Preview volume for sample audition playback.";
pub(super) const NORMALIZED_AUDITION_TOOLTIP: &str =
    "Normalize audition playback and extracted selections.";
pub(super) const AUDIO_ENGINE_TOOLTIP: &str = "Audio engine status and output settings.";
pub(super) const GENERAL_SETTINGS_TOOLTIP: &str = "Open Wavecrate settings.";
pub(super) const HELP_TOOLTIPS_ACTIVE_TOOLTIP: &str =
    "Help tips: hover controls to see what they do.";
pub(super) const RELEASE_UPDATE_TOOLTIP: &str = "New Wavecrate release available.";

#[derive(Clone, Debug)]
pub(super) struct TopControlBarProjection {
    pub(super) volume: VolumeSliderProjection,
    pub(super) normalized_audition: NormalizedAuditionButtonProjection,
    pub(super) help_tooltips_enabled: bool,
    pub(super) settings_controls: SettingsControlsProjection,
}

impl TopControlBarProjection {
    pub(super) fn from_app_state(state: &NativeAppState) -> Self {
        let settings_window = &state.ui.settings.ui;
        let active_settings_tab = settings_window
            .audio_settings_open
            .then_some(settings_window.app_settings_tab);
        Self {
            volume: VolumeSliderProjection::new(state.audio.volume),
            normalized_audition: NormalizedAuditionButtonProjection {
                active: state.audio.normalized_audition_enabled,
                tooltip: NORMALIZED_AUDITION_TOOLTIP,
            },
            help_tooltips_enabled: state.ui.chrome.help_tooltips_enabled,
            settings_controls: SettingsControlsProjection {
                help_tooltips_enabled: state.ui.chrome.help_tooltips_enabled,
                audio_engine: AudioEnginePillProjection {
                    label: state.audio_engine_pill_label(),
                    style: state.audio_engine_pill_style(),
                    active: active_settings_tab == Some(AppSettingsTab::AudioEngine),
                    tooltip: AUDIO_ENGINE_TOOLTIP,
                },
                general_settings: GeneralSettingsButtonProjection {
                    active: active_settings_tab == Some(AppSettingsTab::General),
                    tooltip: GENERAL_SETTINGS_TOOLTIP,
                },
                release_update: ReleaseUpdateButtonProjection {
                    visible: state.ui.release_update.available(),
                    active: state.ui.release_update.available(),
                    tooltip: RELEASE_UPDATE_TOOLTIP,
                },
                help_tooltips: HelpTooltipsButtonProjection {
                    active: state.ui.chrome.help_tooltips_enabled,
                    active_tooltip: HELP_TOOLTIPS_ACTIVE_TOOLTIP,
                },
            },
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub(super) struct VolumeSliderProjection {
    pub(super) value: f32,
    pub(super) tooltip: &'static str,
}

impl VolumeSliderProjection {
    pub(super) fn new(value: f32) -> Self {
        Self {
            value,
            tooltip: VOLUME_SLIDER_TOOLTIP,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) struct NormalizedAuditionButtonProjection {
    pub(super) active: bool,
    pub(super) tooltip: &'static str,
}

#[derive(Clone, Debug)]
pub(super) struct SettingsControlsProjection {
    pub(super) help_tooltips_enabled: bool,
    pub(super) audio_engine: AudioEnginePillProjection,
    pub(super) general_settings: GeneralSettingsButtonProjection,
    pub(super) release_update: ReleaseUpdateButtonProjection,
    pub(super) help_tooltips: HelpTooltipsButtonProjection,
}

#[derive(Clone, Debug, PartialEq)]
pub(super) struct AudioEnginePillProjection {
    pub(super) label: String,
    pub(super) style: ui::WidgetStyle,
    pub(super) active: bool,
    pub(super) tooltip: &'static str,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) struct GeneralSettingsButtonProjection {
    pub(super) active: bool,
    pub(super) tooltip: &'static str,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) struct ReleaseUpdateButtonProjection {
    pub(super) visible: bool,
    pub(super) active: bool,
    pub(super) tooltip: &'static str,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) struct HelpTooltipsButtonProjection {
    pub(super) active: bool,
    pub(super) active_tooltip: &'static str,
}
