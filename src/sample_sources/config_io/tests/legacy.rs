use super::super::super::config_types::{
    AnalysisSettings, AppSettingsCore, ConfigError, DropTargetConfig, FeatureFlags,
    InteractionOptions, UpdateSettings,
};
use super::super::load::load_or_default;
use super::super::{CONFIG_FILE_NAME, LEGACY_CONFIG_FILE_NAME};
use super::TestConfigEnv;
use crate::audio::{AudioInputConfig, AudioOutputConfig};
use crate::sample_sources::SampleSource;
use crate::sample_sources::config::{AppConfig, AudioWriteFormatConfig};

#[test]
fn migrates_from_legacy_json() {
    let env = TestConfigEnv::new();
    let legacy_path = env.ensure_app_dir().join(LEGACY_CONFIG_FILE_NAME);
    write_legacy_config(&legacy_path, "old_source");

    let loaded = load_or_default().unwrap();
    assert_eq!(loaded.sources.len(), 1);
    assert_eq!(
        loaded.core.trash_folder,
        Some(std::path::PathBuf::from("trash_here"))
    );

    let backup = legacy_path.with_extension("json.bak");
    assert!(backup.exists(), "expected backup file {}", backup.display());
}

#[test]
fn legacy_migration_replaces_existing_backup_with_current_json() {
    let env = TestConfigEnv::new();
    let legacy_path = env.ensure_app_dir().join(LEGACY_CONFIG_FILE_NAME);
    let backup_path = legacy_path.with_extension("json.bak");
    write_legacy_config(&legacy_path, "current_source");
    std::fs::write(&backup_path, br#"{"sources":["stale"]}"#).unwrap();

    let loaded = load_or_default().unwrap();

    assert_eq!(loaded.sources.len(), 1);
    assert!(!legacy_path.exists());
    let backup = std::fs::read_to_string(&backup_path).unwrap();
    assert!(
        backup.contains("current_source"),
        "backup should contain the migrated legacy JSON, got {backup}"
    );
    assert!(
        !backup.contains("stale"),
        "stale backup contents should be replaced, got {backup}"
    );
}

#[test]
fn legacy_migration_retries_backup_when_toml_was_already_written() {
    let env = TestConfigEnv::new();
    let app_dir = env.ensure_app_dir();
    let legacy_path = app_dir.join(LEGACY_CONFIG_FILE_NAME);
    let settings_path = app_dir.join(CONFIG_FILE_NAME);
    let backup_path = legacy_path.with_extension("json.bak");
    write_legacy_config(&legacy_path, "retry_source");
    std::fs::create_dir(&backup_path).unwrap();

    let first_attempt = load_or_default().unwrap_err();

    assert!(
        matches!(first_attempt, ConfigError::BackupLegacy { .. }),
        "expected backup failure, got {first_attempt:?}"
    );
    assert!(
        settings_path.exists(),
        "settings write should have completed before backup failure"
    );
    assert!(
        legacy_path.exists(),
        "legacy file should remain available for retry"
    );

    std::fs::remove_dir(&backup_path).unwrap();
    let loaded = load_or_default().unwrap();

    assert_eq!(loaded.sources.len(), 1);
    assert!(!legacy_path.exists());
    assert!(backup_path.is_file());
    let backup = std::fs::read_to_string(&backup_path).unwrap();
    assert!(backup.contains("retry_source"));
}

fn write_legacy_config(path: &std::path::Path, source_path: &str) {
    let legacy = AppConfig {
        sources: vec![SampleSource::new(std::path::PathBuf::from(source_path))],
        core: AppSettingsCore {
            feature_flags: FeatureFlags::default(),
            analysis: AnalysisSettings::default(),
            updates: UpdateSettings::default(),
            job_message_queue_capacity: AppSettingsCore::default().job_message_queue_capacity,
            app_data_dir: None,
            trash_folder: Some(std::path::PathBuf::from("trash_here")),
            drop_targets: vec![DropTargetConfig::new(std::path::PathBuf::from(
                "legacy_drop",
            ))],
            last_selected_source: None,
            upper_folder_pane_source: None,
            lower_folder_pane_source: None,
            active_folder_pane: None,
            audio_output: AudioOutputConfig::default(),
            audio_input: AudioInputConfig::default(),
            audio_write_format: AudioWriteFormatConfig::default(),
            volume: 0.9,
            controls: InteractionOptions::default(),
            similarity: Default::default(),
            default_identifier: String::from("legacy"),
            tag_dictionary: Default::default(),
        },
    };
    let mut data = serde_json::to_value(&legacy).unwrap();
    if let Some(core) = data.get_mut("core")
        && let Some(drop_targets) = core.get_mut("drop_targets")
    {
        *drop_targets = serde_json::json!(["legacy_drop"]);
    }
    let data = serde_json::to_vec_pretty(&data).unwrap();
    std::fs::write(path, data).unwrap();
}
