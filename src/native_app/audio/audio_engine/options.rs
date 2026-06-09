use super::{
    NativeAppState, available_devices, available_hosts, format_sample_rate_label,
    supported_sample_rates,
};
use radiant::prelude as ui;

impl NativeAppState {
    pub(in crate::native_app) fn refresh_audio_options(&mut self) {
        let mut error = None;
        self.audio.hosts = available_hosts();
        let host_id = self.selected_audio_host_id();
        self.audio.devices = host_id
            .as_deref()
            .and_then(|host_id| match available_devices(host_id) {
                Ok(devices) => Some(devices),
                Err(err) => {
                    error = Some(err.to_string());
                    None
                }
            })
            .unwrap_or_default();
        let device_name = self.selected_audio_device_name();
        self.audio.sample_rates = match (host_id.as_deref(), device_name.as_deref()) {
            (Some(host_id), Some(device_name)) => {
                match supported_sample_rates(host_id, device_name) {
                    Ok(rates) => rates,
                    Err(err) => {
                        error = Some(err.to_string());
                        Vec::new()
                    }
                }
            }
            _ => Vec::new(),
        };
        if error.is_some() {
            self.audio.settings_error = error;
        }
    }

    pub(super) fn selected_audio_host_id(&self) -> Option<String> {
        self.audio.output_config.host.clone().or_else(|| {
            self.audio
                .hosts
                .iter()
                .find(|host| host.is_default)
                .or_else(|| self.audio.hosts.first())
                .map(|host| host.id.clone())
        })
    }

    pub(super) fn selected_audio_device_name(&self) -> Option<String> {
        self.audio.output_config.device.clone().or_else(|| {
            self.audio
                .devices
                .iter()
                .find(|device| device.is_default)
                .or_else(|| self.audio.devices.first())
                .map(|device| device.name.clone())
        })
    }

    pub(in crate::native_app) fn audio_engine_pill_label(&self) -> String {
        self.audio
            .output_resolved
            .as_ref()
            .map(|output| format_sample_rate_label(output.sample_rate))
            .unwrap_or_else(|| String::from("no audio"))
    }

    pub(in crate::native_app) fn audio_engine_pill_style(&self) -> ui::WidgetStyle {
        if self.audio.output_resolved.is_some() {
            ui::WidgetStyle::subtle(ui::WidgetTone::Neutral)
        } else {
            ui::WidgetStyle::subtle(ui::WidgetTone::Warning)
        }
    }

    pub(in crate::native_app) fn audio_engine_detail_label(&self) -> String {
        self.audio
            .output_resolved
            .as_ref()
            .map(|output| {
                let running_host = self.audio_host_label(output.host_id.as_str());
                let selected_host = self
                    .audio
                    .output_config
                    .host
                    .as_deref()
                    .map(|host| self.audio_host_label(host));
                let host_label = selected_host
                    .filter(|host| *host != running_host)
                    .map(|host| format!("{host} selected | using {running_host}"))
                    .unwrap_or(running_host);
                format!(
                    "{} | {} | {}",
                    host_label,
                    output.device_name,
                    format_sample_rate_label(output.sample_rate)
                )
            })
            .or_else(|| self.audio.settings_error.clone())
            .unwrap_or_else(|| String::from("Audio output idle"))
    }

    pub(super) fn audio_host_label(&self, id: &str) -> String {
        self.audio
            .hosts
            .iter()
            .find(|host| host.id == id)
            .map(|host| host.label.clone())
            .unwrap_or_else(|| id.to_string())
    }
}
