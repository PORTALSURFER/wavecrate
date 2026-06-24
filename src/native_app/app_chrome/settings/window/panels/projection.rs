use crate::native_app::app::AppSettingsTab;
use crate::native_app::app_chrome::view_models::settings::AudioSettingsSnapshot;

#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) enum SettingsPanelProjection {
    General(GeneralSettingsPanelProjection),
    AudioEngine(AudioSettingsPanelProjection),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) struct AudioSettingsPanelProjection {
    pub(super) detail_label: String,
    pub(super) error: Option<String>,
    pub(super) backend_label: &'static str,
    pub(super) output_label: &'static str,
    pub(super) sample_rate_label: &'static str,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) struct GeneralSettingsPanelProjection {
    pub(super) title: &'static str,
    pub(super) trash_folder: TrashFolderProjection,
    pub(super) maintenance: CacheMaintenanceProjection,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) struct TrashFolderProjection {
    pub(super) label: &'static str,
    pub(super) value: String,
    pub(super) choose_button_label: &'static str,
    pub(super) clear_button_label: &'static str,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) struct CacheMaintenanceProjection {
    pub(super) label: &'static str,
    pub(super) clear_button_label: &'static str,
}

pub(super) fn settings_panel_projection(
    snapshot: &AudioSettingsSnapshot,
) -> SettingsPanelProjection {
    match snapshot.tab {
        AppSettingsTab::General => {
            SettingsPanelProjection::General(general_settings_panel_projection(snapshot))
        }
        AppSettingsTab::AudioEngine => {
            SettingsPanelProjection::AudioEngine(audio_settings_panel_projection(snapshot))
        }
    }
}

fn audio_settings_panel_projection(
    snapshot: &AudioSettingsSnapshot,
) -> AudioSettingsPanelProjection {
    AudioSettingsPanelProjection {
        detail_label: snapshot.detail_label.clone(),
        error: snapshot.error.clone(),
        backend_label: "Backend",
        output_label: "Output",
        sample_rate_label: "Sample Rate",
    }
}

fn general_settings_panel_projection(
    snapshot: &AudioSettingsSnapshot,
) -> GeneralSettingsPanelProjection {
    GeneralSettingsPanelProjection {
        title: "General",
        trash_folder: TrashFolderProjection {
            label: "Trash Folder",
            value: snapshot
                .trash_folder
                .as_ref()
                .map(|path| path.display().to_string())
                .unwrap_or_else(|| "No trash folder configured".to_string()),
            choose_button_label: "Choose Folder",
            clear_button_label: "Clear",
        },
        maintenance: CacheMaintenanceProjection {
            label: "Maintenance",
            clear_button_label: "Clear Rebuildable Caches",
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::native_app::test_support::state::{NativeAppState, NativeAppStateFixture};

    fn snapshot(configure: impl FnOnce(&mut NativeAppState)) -> AudioSettingsSnapshot {
        let mut state = NativeAppStateFixture::default().build();
        configure(&mut state);
        AudioSettingsSnapshot::from_app_state(&state)
    }

    #[test]
    fn audio_panel_projection_carries_product_labels_and_error() {
        let snapshot = snapshot(|state| {
            state.ui.settings.ui.app_settings_tab = AppSettingsTab::AudioEngine;
            state.audio.settings_error = Some("Could not open output".to_string());
        });

        let SettingsPanelProjection::AudioEngine(projection) = settings_panel_projection(&snapshot)
        else {
            panic!("expected audio engine panel");
        };

        assert_eq!(projection.backend_label, "Backend");
        assert_eq!(projection.output_label, "Output");
        assert_eq!(projection.sample_rate_label, "Sample Rate");
        assert_eq!(projection.error.as_deref(), Some("Could not open output"));
    }

    #[test]
    fn general_panel_projection_uses_trash_folder_fallback() {
        let snapshot = snapshot(|state| {
            state.ui.settings.ui.app_settings_tab = AppSettingsTab::General;
        });

        let SettingsPanelProjection::General(projection) = settings_panel_projection(&snapshot)
        else {
            panic!("expected general panel");
        };

        assert_eq!(projection.title, "General");
        assert_eq!(projection.trash_folder.label, "Trash Folder");
        assert_eq!(projection.trash_folder.value, "No trash folder configured");
        assert_eq!(projection.trash_folder.choose_button_label, "Choose Folder");
        assert_eq!(projection.trash_folder.clear_button_label, "Clear");
        assert_eq!(projection.maintenance.label, "Maintenance");
        assert_eq!(
            projection.maintenance.clear_button_label,
            "Clear Rebuildable Caches"
        );
    }

    #[test]
    fn general_panel_projection_formats_configured_trash_folder() {
        let snapshot = snapshot(|state| {
            state.ui.settings.ui.app_settings_tab = AppSettingsTab::General;
            state.ui.settings.persisted.trash_folder = Some("wavecrate-trash".into());
        });

        let SettingsPanelProjection::General(projection) = settings_panel_projection(&snapshot)
        else {
            panic!("expected general panel");
        };

        assert_eq!(projection.trash_folder.value, "wavecrate-trash");
    }
}
