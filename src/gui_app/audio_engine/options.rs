use super::{
    GuiAppState, available_devices, available_hosts, format_sample_rate_label,
    supported_sample_rates,
};

impl GuiAppState {
    pub(in crate::gui_app) fn refresh_audio_options(&mut self) {
        let mut error = None;
        self.audio_hosts = available_hosts();
        let host_id = self.selected_audio_host_id();
        self.audio_devices = host_id
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
        self.audio_sample_rates = match (host_id.as_deref(), device_name.as_deref()) {
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
            self.audio_settings_error = error;
        }
    }

    pub(super) fn selected_audio_host_id(&self) -> Option<String> {
        self.audio_output_config.host.clone().or_else(|| {
            self.audio_hosts
                .iter()
                .find(|host| host.is_default)
                .or_else(|| self.audio_hosts.first())
                .map(|host| host.id.clone())
        })
    }

    pub(super) fn selected_audio_device_name(&self) -> Option<String> {
        self.audio_output_config.device.clone().or_else(|| {
            self.audio_devices
                .iter()
                .find(|device| device.is_default)
                .or_else(|| self.audio_devices.first())
                .map(|device| device.name.clone())
        })
    }

    pub(in crate::gui_app) fn audio_engine_pill_label(&self) -> String {
        if self.audio_settings_error.is_some() {
            String::from("Audio !")
        } else {
            String::from("Audio")
        }
    }

    pub(in crate::gui_app) fn audio_engine_detail_label(&self) -> String {
        self.audio_output_resolved
            .as_ref()
            .map(|output| {
                format!(
                    "{} | {} | {}",
                    self.audio_host_label(output.host_id.as_str()),
                    output.device_name,
                    format_sample_rate_label(output.sample_rate)
                )
            })
            .or_else(|| self.audio_settings_error.clone())
            .unwrap_or_else(|| String::from("Audio output idle"))
    }

    pub(super) fn audio_host_label(&self, id: &str) -> String {
        self.audio_hosts
            .iter()
            .find(|host| host.id == id)
            .map(|host| host.label.clone())
            .unwrap_or_else(|| id.to_string())
    }
}
