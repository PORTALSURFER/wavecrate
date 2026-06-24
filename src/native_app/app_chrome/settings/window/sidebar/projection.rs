use crate::native_app::app::{AppSettingsTab, SettingsMessage};
use crate::native_app::app_chrome::view_models::settings::AudioSettingsSnapshot;

#[derive(Clone, Debug, PartialEq)]
pub(super) struct SettingsSidebarProjection {
    pub(super) title: &'static str,
    pub(super) tabs: Vec<SettingsTabProjection>,
}

#[derive(Clone, Debug, PartialEq)]
pub(super) struct SettingsTabProjection {
    pub(super) label: &'static str,
    pub(super) selected: bool,
    pub(super) message: SettingsMessage,
}

pub(super) fn settings_sidebar_projection(
    snapshot: &AudioSettingsSnapshot,
) -> SettingsSidebarProjection {
    SettingsSidebarProjection {
        title: "Settings",
        tabs: [AppSettingsTab::General, AppSettingsTab::AudioEngine]
            .into_iter()
            .map(|tab| SettingsTabProjection {
                label: settings_tab_label(tab),
                selected: tab == snapshot.tab,
                message: SettingsMessage::SelectSettingsTab(tab),
            })
            .collect(),
    }
}

fn settings_tab_label(tab: AppSettingsTab) -> &'static str {
    match tab {
        AppSettingsTab::General => "General",
        AppSettingsTab::AudioEngine => "Audio Engine",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn snapshot(configure: impl FnOnce(&mut AudioSettingsSnapshot)) -> AudioSettingsSnapshot {
        let mut snapshot = AudioSettingsSnapshot::test_default();
        configure(&mut snapshot);
        snapshot
    }

    #[test]
    fn sidebar_projection_carries_tabs_and_selection_actions() {
        let snapshot = snapshot(|snapshot| {
            snapshot.tab = AppSettingsTab::General;
        });

        let projection = settings_sidebar_projection(&snapshot);

        assert_eq!(projection.title, "Settings");
        assert_eq!(
            projection.tabs,
            [
                SettingsTabProjection {
                    label: "General",
                    selected: true,
                    message: SettingsMessage::SelectSettingsTab(AppSettingsTab::General),
                },
                SettingsTabProjection {
                    label: "Audio Engine",
                    selected: false,
                    message: SettingsMessage::SelectSettingsTab(AppSettingsTab::AudioEngine),
                },
            ]
        );
    }
}
