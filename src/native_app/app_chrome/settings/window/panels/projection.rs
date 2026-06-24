use crate::native_app::app::{AppSettingsTab, AudioSettingsDropdown, SettingsMessage};
use crate::native_app::app_chrome::view_models::settings::AudioSettingsSnapshot;

#[derive(Clone, Debug, PartialEq)]
pub(super) enum SettingsPanelProjection {
    General {
        rows: Vec<SettingsPanelRowProjection>,
    },
    AudioEngine {
        rows: Vec<SettingsPanelRowProjection>,
    },
}

impl SettingsPanelProjection {
    pub(super) fn rows(self) -> Vec<SettingsPanelRowProjection> {
        match self {
            Self::General { rows } | Self::AudioEngine { rows } => rows,
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub(super) enum SettingsPanelRowProjection {
    Title {
        label: &'static str,
    },
    AudioDetail {
        label: String,
    },
    AudioError {
        message: String,
    },
    AudioDropdown {
        label: &'static str,
        dropdown: AudioSettingsDropdown,
    },
    TrashFolder(TrashFolderProjection),
    CacheMaintenance(CacheMaintenanceProjection),
}

#[derive(Clone, Debug, PartialEq)]
pub(super) struct TrashFolderProjection {
    pub(super) label: &'static str,
    pub(super) value: String,
    pub(super) choose_action: SettingsActionProjection,
    pub(super) clear_action: SettingsActionProjection,
}

#[derive(Clone, Debug, PartialEq)]
pub(super) struct CacheMaintenanceProjection {
    pub(super) label: &'static str,
    pub(super) clear_action: SettingsActionProjection,
}

#[derive(Clone, Debug, PartialEq)]
pub(super) struct SettingsActionProjection {
    pub(super) label: &'static str,
    pub(super) message: SettingsMessage,
}

pub(super) fn settings_panel_projection(
    snapshot: &AudioSettingsSnapshot,
) -> SettingsPanelProjection {
    match snapshot.tab {
        AppSettingsTab::General => SettingsPanelProjection::General {
            rows: general_settings_panel_rows(snapshot),
        },
        AppSettingsTab::AudioEngine => SettingsPanelProjection::AudioEngine {
            rows: audio_settings_panel_rows(snapshot),
        },
    }
}

fn audio_settings_panel_rows(snapshot: &AudioSettingsSnapshot) -> Vec<SettingsPanelRowProjection> {
    let mut rows = vec![SettingsPanelRowProjection::AudioDetail {
        label: snapshot.detail_label.clone(),
    }];
    if let Some(error) = &snapshot.error {
        rows.push(SettingsPanelRowProjection::AudioError {
            message: error.clone(),
        });
    }
    rows.extend([
        SettingsPanelRowProjection::AudioDropdown {
            label: "Backend",
            dropdown: AudioSettingsDropdown::Backend,
        },
        SettingsPanelRowProjection::AudioDropdown {
            label: "Output",
            dropdown: AudioSettingsDropdown::Output,
        },
        SettingsPanelRowProjection::AudioDropdown {
            label: "Sample Rate",
            dropdown: AudioSettingsDropdown::SampleRate,
        },
    ]);
    rows
}

fn general_settings_panel_rows(
    snapshot: &AudioSettingsSnapshot,
) -> Vec<SettingsPanelRowProjection> {
    vec![
        SettingsPanelRowProjection::Title { label: "General" },
        SettingsPanelRowProjection::TrashFolder(TrashFolderProjection {
            label: "Trash Folder",
            value: snapshot
                .trash_folder
                .as_ref()
                .map(|path| path.display().to_string())
                .unwrap_or_else(|| "No trash folder configured".to_string()),
            choose_action: SettingsActionProjection {
                label: "Choose Folder",
                message: SettingsMessage::PickTrashFolder,
            },
            clear_action: SettingsActionProjection {
                label: "Clear",
                message: SettingsMessage::ClearTrashFolder,
            },
        }),
        SettingsPanelRowProjection::CacheMaintenance(CacheMaintenanceProjection {
            label: "Maintenance",
            clear_action: SettingsActionProjection {
                label: "Clear Rebuildable Caches",
                message: SettingsMessage::ClearRebuildableCaches,
            },
        }),
    ]
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

        let SettingsPanelProjection::AudioEngine { rows } = settings_panel_projection(&snapshot)
        else {
            panic!("expected audio engine panel");
        };

        assert!(matches!(
            &rows[0],
            SettingsPanelRowProjection::AudioDetail { label } if label == &snapshot.detail_label
        ));
        assert_eq!(
            rows[1],
            SettingsPanelRowProjection::AudioError {
                message: "Could not open output".to_string()
            }
        );
        assert_eq!(
            &rows[2..],
            [
                SettingsPanelRowProjection::AudioDropdown {
                    label: "Backend",
                    dropdown: AudioSettingsDropdown::Backend,
                },
                SettingsPanelRowProjection::AudioDropdown {
                    label: "Output",
                    dropdown: AudioSettingsDropdown::Output,
                },
                SettingsPanelRowProjection::AudioDropdown {
                    label: "Sample Rate",
                    dropdown: AudioSettingsDropdown::SampleRate,
                },
            ]
        );
    }

    #[test]
    fn general_panel_projection_uses_trash_folder_fallback() {
        let snapshot = snapshot(|state| {
            state.ui.settings.ui.app_settings_tab = AppSettingsTab::General;
        });

        let SettingsPanelProjection::General { rows } = settings_panel_projection(&snapshot) else {
            panic!("expected general panel");
        };

        assert_eq!(rows.len(), 3);
        assert_eq!(
            rows[0],
            SettingsPanelRowProjection::Title { label: "General" }
        );
        let SettingsPanelRowProjection::TrashFolder(trash_folder) = &rows[1] else {
            panic!("expected trash folder row");
        };
        assert_eq!(trash_folder.label, "Trash Folder");
        assert_eq!(trash_folder.value, "No trash folder configured");
        assert_eq!(trash_folder.choose_action.label, "Choose Folder");
        assert_eq!(
            trash_folder.choose_action.message,
            SettingsMessage::PickTrashFolder
        );
        assert_eq!(trash_folder.clear_action.label, "Clear");
        assert_eq!(
            trash_folder.clear_action.message,
            SettingsMessage::ClearTrashFolder
        );

        let SettingsPanelRowProjection::CacheMaintenance(maintenance) = &rows[2] else {
            panic!("expected cache maintenance row");
        };
        assert_eq!(maintenance.label, "Maintenance");
        assert_eq!(maintenance.clear_action.label, "Clear Rebuildable Caches");
        assert_eq!(
            maintenance.clear_action.message,
            SettingsMessage::ClearRebuildableCaches
        );
    }

    #[test]
    fn general_panel_projection_formats_configured_trash_folder() {
        let snapshot = snapshot(|state| {
            state.ui.settings.ui.app_settings_tab = AppSettingsTab::General;
            state.ui.settings.persisted.trash_folder = Some("wavecrate-trash".into());
        });

        let SettingsPanelProjection::General { rows } = settings_panel_projection(&snapshot) else {
            panic!("expected general panel");
        };
        let SettingsPanelRowProjection::TrashFolder(trash_folder) = &rows[1] else {
            panic!("expected trash folder row");
        };

        assert_eq!(trash_folder.value, "wavecrate-trash");
    }
}
