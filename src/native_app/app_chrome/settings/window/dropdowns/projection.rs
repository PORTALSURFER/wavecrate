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
    audio_string_dropdown_projection(
        snapshot.audio_output_config.host.clone(),
        SYSTEM_DEFAULT_HOST_LABEL,
        snapshot.dropdown_open(AudioSettingsDropdown::Backend),
        snapshot.audio_hosts.iter().map(|host| {
            AudioStringDropdownEntry::new(host.id.as_str(), host.label.as_str(), host.is_default)
        }),
    )
}

pub(super) fn audio_output_dropdown_projection(
    snapshot: &AudioSettingsSnapshot,
) -> AudioStringDropdownProjection {
    audio_string_dropdown_projection(
        snapshot.audio_output_config.device.clone(),
        HOST_DEFAULT_DEVICE_LABEL,
        snapshot.dropdown_open(AudioSettingsDropdown::Output),
        snapshot.audio_devices.iter().map(|device| {
            AudioStringDropdownEntry::new(
                device.name.as_str(),
                device.name.as_str(),
                device.is_default,
            )
        }),
    )
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

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct AudioStringDropdownEntry<'a> {
    value: &'a str,
    label: &'a str,
    is_default: bool,
}

impl<'a> AudioStringDropdownEntry<'a> {
    fn new(value: &'a str, label: &'a str, is_default: bool) -> Self {
        Self {
            value,
            label,
            is_default,
        }
    }
}

fn audio_string_dropdown_projection<'a>(
    selected_value: Option<String>,
    default_label: &'static str,
    open: bool,
    entries: impl IntoIterator<Item = AudioStringDropdownEntry<'a>>,
) -> AudioStringDropdownProjection {
    let entries = entries.into_iter().collect::<Vec<_>>();
    let selected_label = selected_value
        .as_deref()
        .and_then(|selected| {
            entries
                .iter()
                .find(|entry| entry.value == selected)
                .map(|entry| default_option_label(entry.label, entry.is_default))
        })
        .or_else(|| selected_value.clone())
        .unwrap_or_else(|| default_label.to_string());
    let mut options = vec![AudioStringDropdownOptionProjection::new(
        default_label,
        None,
    )];
    options.extend(entries.into_iter().map(|entry| {
        AudioStringDropdownOptionProjection::new(
            default_option_label(entry.label, entry.is_default),
            Some(entry.value.to_string()),
        )
    }));

    AudioStringDropdownProjection {
        selected_label,
        selected_value,
        open,
        options,
    }
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
    use wavecrate::audio::{AudioDeviceSummary, AudioHostSummary};

    fn snapshot(configure: impl FnOnce(&mut AudioSettingsSnapshot)) -> AudioSettingsSnapshot {
        let mut snapshot = AudioSettingsSnapshot::test_default();
        configure(&mut snapshot);
        snapshot
    }

    #[test]
    fn audio_host_projection_preserves_unknown_configured_host() {
        let snapshot = snapshot(|snapshot| {
            snapshot.audio_output_config.host = Some("jack".to_string());
            snapshot.audio_hosts = vec![
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
    fn string_dropdown_projection_preserves_unknown_value_and_default_labels() {
        let projection = audio_string_dropdown_projection(
            Some("custom".to_string()),
            "System default",
            true,
            [
                AudioStringDropdownEntry::new("wasapi", "WASAPI", true),
                AudioStringDropdownEntry::new("asio", "ASIO", false),
            ],
        );

        assert_eq!(projection.selected_label, "custom");
        assert_eq!(projection.selected_value.as_deref(), Some("custom"));
        assert!(projection.open);
        assert_eq!(
            option_labels(&projection.options),
            ["System default", "WASAPI (default)", "ASIO"]
        );
    }

    #[test]
    fn audio_output_projection_marks_default_device() {
        let snapshot = snapshot(|snapshot| {
            snapshot.audio_output_config.device = Some("Studio Out".to_string());
            snapshot.audio_devices = vec![AudioDeviceSummary {
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
        let snapshot = snapshot(|snapshot| {
            snapshot.audio_output_config.sample_rate = Some(44_100);
            snapshot.audio_sample_rates = vec![44_100, 48_000];
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
            let snapshot = snapshot(|snapshot| {
                snapshot.set_open_dropdown_for_tests(dropdown);
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
