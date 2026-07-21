use crate::native_app::app::GlobalStorageUsageState;
use crate::native_app::app::{AppSettingsTab, AudioSettingsDropdown, SettingsMessage};
use crate::native_app::app_chrome::view_models::settings::AudioSettingsSnapshot;
use wavecrate::sample_sources::config::{
    MAX_RATING_DECAY_WEEKS, MIN_RATING_DECAY_WEEKS, clamp_rating_decay_weeks,
};

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
    RatingDecay(RatingDecayProjection),
    GlobalStorage(GlobalStorageProjection),
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
pub(super) struct GlobalStorageProjection {
    pub(super) label: &'static str,
    pub(super) total_label: String,
    pub(super) detail_label: Option<String>,
}

#[derive(Clone, Debug, PartialEq)]
pub(super) struct RatingDecayProjection {
    pub(super) label: &'static str,
    pub(super) weeks: u16,
    pub(super) slider_value: f32,
    pub(super) value_label: String,
}

impl RatingDecayProjection {
    pub(super) fn weeks_from_slider_value(value: f32) -> u16 {
        let span = f32::from(MAX_RATING_DECAY_WEEKS - MIN_RATING_DECAY_WEEKS);
        let weeks = f32::from(MIN_RATING_DECAY_WEEKS) + value.clamp(0.0, 1.0) * span;
        clamp_rating_decay_weeks(weeks.round() as u16)
    }
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
        SettingsPanelRowProjection::RatingDecay(RatingDecayProjection {
            label: "Rating Decay",
            weeks: clamp_rating_decay_weeks(snapshot.rating_decay_weeks),
            slider_value: rating_decay_slider_value(snapshot.rating_decay_weeks),
            value_label: rating_decay_value_label(snapshot.rating_decay_weeks),
        }),
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
        SettingsPanelRowProjection::GlobalStorage(global_storage_projection(
            &snapshot.global_storage_usage,
        )),
        SettingsPanelRowProjection::CacheMaintenance(CacheMaintenanceProjection {
            label: "Maintenance",
            clear_action: SettingsActionProjection {
                label: "Clear Rebuildable Caches",
                message: SettingsMessage::ClearRebuildableCaches,
            },
        }),
    ]
}

fn global_storage_projection(state: &GlobalStorageUsageState) -> GlobalStorageProjection {
    match state {
        GlobalStorageUsageState::NotLoaded | GlobalStorageUsageState::Loading => {
            GlobalStorageProjection {
                label: "Database + Cache",
                total_label: "Calculating...".to_string(),
                detail_label: None,
            }
        }
        GlobalStorageUsageState::Ready(usage) => GlobalStorageProjection {
            label: "Database + Cache",
            total_label: format!("{} total", format_storage_bytes(usage.total_bytes())),
            detail_label: Some(format!(
                "{} database + {} cache",
                format_storage_bytes(usage.database_bytes),
                format_storage_bytes(usage.cache_bytes)
            )),
        },
        GlobalStorageUsageState::Unavailable => GlobalStorageProjection {
            label: "Database + Cache",
            total_label: "Size unavailable".to_string(),
            detail_label: None,
        },
    }
}

fn format_storage_bytes(bytes: u64) -> String {
    const KIB: u64 = 1024;
    const MIB: u64 = KIB * 1024;
    const GIB: u64 = MIB * 1024;
    if bytes >= GIB {
        format!("{:.1} GiB", bytes as f64 / GIB as f64)
    } else if bytes >= MIB {
        format!("{:.1} MiB", bytes as f64 / MIB as f64)
    } else if bytes >= KIB {
        format!("{:.1} KiB", bytes as f64 / KIB as f64)
    } else {
        format!("{bytes} B")
    }
}

fn rating_decay_slider_value(weeks: u16) -> f32 {
    let weeks = clamp_rating_decay_weeks(weeks);
    let span = f32::from(MAX_RATING_DECAY_WEEKS - MIN_RATING_DECAY_WEEKS);
    if span <= 0.0 {
        return 0.0;
    }
    f32::from(weeks - MIN_RATING_DECAY_WEEKS) / span
}

fn rating_decay_value_label(weeks: u16) -> String {
    let weeks = clamp_rating_decay_weeks(weeks);
    if weeks == 1 {
        "1 week".to_string()
    } else {
        format!("{weeks} weeks")
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
    fn audio_panel_projection_carries_product_labels_and_error() {
        let snapshot = snapshot(|snapshot| {
            snapshot.tab = AppSettingsTab::AudioEngine;
            snapshot.error = Some("Could not open output".to_string());
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
        let snapshot = snapshot(|snapshot| {
            snapshot.tab = AppSettingsTab::General;
        });

        let SettingsPanelProjection::General { rows } = settings_panel_projection(&snapshot) else {
            panic!("expected general panel");
        };

        assert_eq!(rows.len(), 5);
        assert_eq!(
            rows[0],
            SettingsPanelRowProjection::Title { label: "General" }
        );
        let SettingsPanelRowProjection::RatingDecay(rating_decay) = &rows[1] else {
            panic!("expected rating decay row");
        };
        assert_eq!(rating_decay.label, "Rating Decay");
        assert_eq!(rating_decay.weeks, snapshot.rating_decay_weeks);
        assert_eq!(rating_decay.value_label, "4 weeks");
        assert_eq!(
            RatingDecayProjection::weeks_from_slider_value(rating_decay.slider_value),
            snapshot.rating_decay_weeks
        );

        let SettingsPanelRowProjection::TrashFolder(trash_folder) = &rows[2] else {
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

        let SettingsPanelRowProjection::GlobalStorage(storage) = &rows[3] else {
            panic!("expected global storage row");
        };
        assert_eq!(storage.label, "Database + Cache");
        assert_eq!(storage.total_label, "Calculating...");
        assert_eq!(storage.detail_label, None);

        let SettingsPanelRowProjection::CacheMaintenance(maintenance) = &rows[4] else {
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
        let snapshot = snapshot(|snapshot| {
            snapshot.tab = AppSettingsTab::General;
            snapshot.trash_folder = Some("wavecrate-trash".into());
        });

        let SettingsPanelProjection::General { rows } = settings_panel_projection(&snapshot) else {
            panic!("expected general panel");
        };
        let SettingsPanelRowProjection::TrashFolder(trash_folder) = &rows[2] else {
            panic!("expected trash folder row");
        };

        assert_eq!(trash_folder.value, "wavecrate-trash");
    }

    #[test]
    fn rating_decay_projection_clamps_and_formats_weeks() {
        let snapshot = snapshot(|snapshot| {
            snapshot.tab = AppSettingsTab::General;
            snapshot.rating_decay_weeks = 12;
        });

        let SettingsPanelProjection::General { rows } = settings_panel_projection(&snapshot) else {
            panic!("expected general panel");
        };
        let SettingsPanelRowProjection::RatingDecay(rating_decay) = &rows[1] else {
            panic!("expected rating decay row");
        };

        assert_eq!(rating_decay.weeks, 12);
        assert_eq!(rating_decay.value_label, "12 weeks");
        assert_eq!(
            RatingDecayProjection::weeks_from_slider_value(rating_decay.slider_value),
            12
        );
    }

    #[test]
    fn global_storage_projection_formats_total_and_breakdown() {
        let snapshot = snapshot(|snapshot| {
            snapshot.tab = AppSettingsTab::General;
            snapshot.global_storage_usage =
                GlobalStorageUsageState::Ready(wavecrate::app_dirs::GlobalStorageUsage {
                    database_bytes: 9 * 1024 * 1024 + 512 * 1024,
                    cache_bytes: 470 * 1024 * 1024,
                });
        });

        let SettingsPanelProjection::General { rows } = settings_panel_projection(&snapshot) else {
            panic!("expected general panel");
        };
        let SettingsPanelRowProjection::GlobalStorage(storage) = &rows[3] else {
            panic!("expected global storage row");
        };

        assert_eq!(storage.total_label, "479.5 MiB total");
        assert_eq!(
            storage.detail_label.as_deref(),
            Some("9.5 MiB database + 470.0 MiB cache")
        );
    }

    #[test]
    fn storage_size_formatter_preserves_small_values_and_scales_large_values() {
        assert_eq!(format_storage_bytes(0), "0 B");
        assert_eq!(format_storage_bytes(512), "512 B");
        assert_eq!(format_storage_bytes(1536), "1.5 KiB");
        assert_eq!(format_storage_bytes(2 * 1024 * 1024), "2.0 MiB");
        assert_eq!(format_storage_bytes(3 * 1024 * 1024 * 1024), "3.0 GiB");
    }
}
