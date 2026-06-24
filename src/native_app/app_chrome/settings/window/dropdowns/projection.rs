use crate::native_app::app::AudioSettingsDropdown;
use crate::native_app::app_chrome::view_models::settings::AudioSettingsSnapshot;
use crate::native_app::ui::display::format_sample_rate_label;

const SYSTEM_DEFAULT_HOST_LABEL: &str = "System default";
const HOST_DEFAULT_DEVICE_LABEL: &str = "Host default";
const DEVICE_DEFAULT_SAMPLE_RATE_LABEL: &str = "Device default";

#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) struct AudioStringDropdownProjection {
    pub(super) selected_label: String,
    pub(super) selected_value: Option<String>,
    pub(super) open: bool,
    pub(super) options: Vec<AudioStringDropdownOptionProjection>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) struct AudioStringDropdownOptionProjection {
    pub(super) label: String,
    pub(super) value: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) struct AudioSampleRateDropdownProjection {
    pub(super) selected_label: String,
    pub(super) selected_value: Option<u32>,
    pub(super) open: bool,
    pub(super) options: Vec<AudioSampleRateDropdownOptionProjection>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) struct AudioSampleRateDropdownOptionProjection {
    pub(super) label: String,
    pub(super) value: Option<u32>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) struct OpenAudioSettingsDropdownProjection {
    pub(super) dropdown: AudioSettingsDropdown,
    pub(super) row_index: usize,
}

pub(super) fn audio_host_dropdown_projection(
    snapshot: &AudioSettingsSnapshot,
) -> AudioStringDropdownProjection {
    let selected_value = snapshot.audio_output_config.host.clone();
    let selected_label = selected_value
        .as_deref()
        .and_then(|selected| {
            snapshot
                .audio_hosts
                .iter()
                .find(|host| host.id == selected)
                .map(|host| default_option_label(host.label.as_str(), host.is_default))
        })
        .or_else(|| selected_value.clone())
        .unwrap_or_else(|| SYSTEM_DEFAULT_HOST_LABEL.to_string());
    let mut options = vec![AudioStringDropdownOptionProjection::new(
        SYSTEM_DEFAULT_HOST_LABEL,
        None,
    )];
    options.extend(snapshot.audio_hosts.iter().map(|host| {
        AudioStringDropdownOptionProjection::new(
            default_option_label(host.label.as_str(), host.is_default),
            Some(host.id.clone()),
        )
    }));
    AudioStringDropdownProjection {
        selected_label,
        selected_value,
        open: snapshot.dropdown_open(AudioSettingsDropdown::Backend),
        options,
    }
}

pub(super) fn audio_output_dropdown_projection(
    snapshot: &AudioSettingsSnapshot,
) -> AudioStringDropdownProjection {
    let selected_value = snapshot.audio_output_config.device.clone();
    let selected_label = selected_value
        .as_deref()
        .and_then(|selected| {
            snapshot
                .audio_devices
                .iter()
                .find(|device| device.name == selected)
                .map(|device| default_option_label(device.name.as_str(), device.is_default))
        })
        .or_else(|| selected_value.clone())
        .unwrap_or_else(|| HOST_DEFAULT_DEVICE_LABEL.to_string());
    let mut options = vec![AudioStringDropdownOptionProjection::new(
        HOST_DEFAULT_DEVICE_LABEL,
        None,
    )];
    options.extend(snapshot.audio_devices.iter().map(|device| {
        AudioStringDropdownOptionProjection::new(
            default_option_label(device.name.as_str(), device.is_default),
            Some(device.name.clone()),
        )
    }));
    AudioStringDropdownProjection {
        selected_label,
        selected_value,
        open: snapshot.dropdown_open(AudioSettingsDropdown::Output),
        options,
    }
}

pub(super) fn audio_sample_rate_dropdown_projection(
    snapshot: &AudioSettingsSnapshot,
) -> AudioSampleRateDropdownProjection {
    let selected_value = snapshot.audio_output_config.sample_rate;
    let selected_label = selected_value
        .map(format_sample_rate_label)
        .unwrap_or_else(|| DEVICE_DEFAULT_SAMPLE_RATE_LABEL.to_string());
    let mut options = vec![AudioSampleRateDropdownOptionProjection::new(
        DEVICE_DEFAULT_SAMPLE_RATE_LABEL,
        None,
    )];
    options.extend(snapshot.audio_sample_rates.iter().copied().map(|rate| {
        AudioSampleRateDropdownOptionProjection::new(format_sample_rate_label(rate), Some(rate))
    }));
    AudioSampleRateDropdownProjection {
        selected_label,
        selected_value,
        open: snapshot.dropdown_open(AudioSettingsDropdown::SampleRate),
        options,
    }
}

pub(super) fn open_audio_settings_dropdown_projection(
    snapshot: &AudioSettingsSnapshot,
) -> Option<OpenAudioSettingsDropdownProjection> {
    let dropdown = snapshot.open_dropdown()?;
    Some(OpenAudioSettingsDropdownProjection {
        dropdown,
        row_index: match dropdown {
            AudioSettingsDropdown::Backend => 0,
            AudioSettingsDropdown::Output => 1,
            AudioSettingsDropdown::SampleRate => 2,
        },
    })
}

impl AudioStringDropdownOptionProjection {
    fn new(label: impl Into<String>, value: Option<String>) -> Self {
        Self {
            label: label.into(),
            value,
        }
    }
}

impl AudioSampleRateDropdownOptionProjection {
    fn new(label: impl Into<String>, value: Option<u32>) -> Self {
        Self {
            label: label.into(),
            value,
        }
    }
}

fn default_option_label(label: &str, is_default: bool) -> String {
    if is_default {
        format!("{label} (default)")
    } else {
        label.to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::native_app::test_support::{
        audio::{AudioDeviceSummary, AudioHostSummary},
        state::{NativeAppState, NativeAppStateFixture},
    };

    fn snapshot(configure: impl FnOnce(&mut NativeAppState)) -> AudioSettingsSnapshot {
        let mut state = NativeAppStateFixture::default().build();
        configure(&mut state);
        AudioSettingsSnapshot::from_app_state(&state)
    }

    #[test]
    fn audio_host_projection_preserves_unknown_configured_host() {
        let snapshot = snapshot(|state| {
            state.audio.output_config.host = Some("jack".to_string());
            state.audio.hosts = vec![
                AudioHostSummary {
                    id: "wasapi".to_string(),
                    label: "WASAPI".to_string(),
                    is_default: true,
                },
                AudioHostSummary {
                    id: "asio".to_string(),
                    label: "ASIO".to_string(),
                    is_default: false,
                },
            ];
        });

        let projection = audio_host_dropdown_projection(&snapshot);

        assert_eq!(projection.selected_label, "jack");
        assert_eq!(projection.selected_value.as_deref(), Some("jack"));
        assert_eq!(
            option_labels(&projection.options),
            ["System default", "WASAPI (default)", "ASIO"]
        );
    }

    #[test]
    fn audio_output_projection_marks_default_device() {
        let snapshot = snapshot(|state| {
            state.audio.output_config.device = Some("Studio Out".to_string());
            state.audio.devices = vec![AudioDeviceSummary {
                host_id: "asio".to_string(),
                name: "Studio Out".to_string(),
                is_default: true,
            }];
        });

        let projection = audio_output_dropdown_projection(&snapshot);

        assert_eq!(projection.selected_label, "Studio Out (default)");
        assert_eq!(projection.selected_value.as_deref(), Some("Studio Out"));
        assert_eq!(
            option_labels(&projection.options),
            ["Host default", "Studio Out (default)"]
        );
    }

    #[test]
    fn sample_rate_projection_formats_configured_rate_and_default_option() {
        let snapshot = snapshot(|state| {
            state.audio.output_config.sample_rate = Some(44_100);
            state.audio.sample_rates = vec![44_100, 48_000];
        });

        let projection = audio_sample_rate_dropdown_projection(&snapshot);

        assert_eq!(projection.selected_label, "44.1 kHz");
        assert_eq!(projection.selected_value, Some(44_100));
        assert_eq!(
            sample_rate_option_labels(&projection.options),
            ["Device default", "44.1 kHz", "48 kHz"]
        );
    }

    #[test]
    fn open_dropdown_projection_uses_labeled_row_order() {
        let no_dropdown = snapshot(|_| {});
        assert_eq!(open_audio_settings_dropdown_projection(&no_dropdown), None);

        for (dropdown, row_index) in [
            (AudioSettingsDropdown::Backend, 0),
            (AudioSettingsDropdown::Output, 1),
            (AudioSettingsDropdown::SampleRate, 2),
        ] {
            let snapshot = snapshot(|state| {
                state.ui.settings.ui.audio_settings_dropdown.open(dropdown);
            });
            assert_eq!(
                open_audio_settings_dropdown_projection(&snapshot),
                Some(OpenAudioSettingsDropdownProjection {
                    dropdown,
                    row_index
                })
            );
        }
    }

    fn option_labels(options: &[AudioStringDropdownOptionProjection]) -> Vec<&str> {
        options.iter().map(|option| option.label.as_str()).collect()
    }

    fn sample_rate_option_labels(options: &[AudioSampleRateDropdownOptionProjection]) -> Vec<&str> {
        options.iter().map(|option| option.label.as_str()).collect()
    }
}
