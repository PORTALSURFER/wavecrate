#[cfg(any(unix, target_os = "windows"))]
use super::super::super::config_types::AppSettings;
use super::super::super::config_types::{
    AnalysisSettings, AppSettingsCore, AudioWriteChannelBehavior, AudioWriteDither,
    AudioWriteFormatConfig, AudioWriteSampleFormat, AudioWriteSampleRate, DropTargetColor,
    DropTargetConfig, FeatureFlags, InteractionOptions, SimilarityAspectSettings, TooltipMode,
    UpdateChannel, UpdateSettings,
};
use super::super::load::load_settings_from;
#[cfg(any(unix, target_os = "windows"))]
use super::super::save::save_settings_to_path;
use super::super::save::save_to_path;
use super::TestConfigEnv;
use crate::audio::{AudioInputConfig, AudioOutputConfig};
use crate::sample_sources::config::AppConfig;
use crate::sample_sources::{SampleSource, SourceId};
use crate::waveform::WaveformChannelView;

mod round_trip;

#[test]
fn saves_settings_to_toml() {
    let env = TestConfigEnv::new();
    let path = env.path("cfg.toml");
    let cfg = AppConfig {
        core: AppSettingsCore {
            volume: 0.42,
            trash_folder: Some(std::path::PathBuf::from("trash")),
            ..AppSettingsCore::default()
        },
        ..AppConfig::default()
    };
    save_to_path(&cfg, &path).unwrap();
    let loaded = load_settings_from(&path).unwrap();
    assert!((loaded.core.volume - 0.42).abs() < f32::EPSILON);
    assert_eq!(
        loaded.core.trash_folder,
        Some(std::path::PathBuf::from("trash"))
    );
}

#[test]
fn saves_settings_with_nested_section_ownership() {
    let env = TestConfigEnv::new();
    let path = env.path("cfg.toml");
    let source_id = SourceId::from_string("source::nested");
    let cfg = AppConfig {
        core: AppSettingsCore {
            job_message_queue_capacity: 256,
            app_data_dir: Some(std::path::PathBuf::from("data")),
            trash_folder: Some(std::path::PathBuf::from("trash")),
            drop_targets: vec![DropTargetConfig::new(std::path::PathBuf::from("drop-a"))],
            last_selected_source: Some(source_id.clone()),
            collection_names: std::collections::BTreeMap::from([(
                String::from("0"),
                String::from("Drums"),
            )]),
            audio_output: AudioOutputConfig {
                host: Some("wasapi".into()),
                device: Some("Studio".into()),
                sample_rate: Some(48_000),
                buffer_size: None,
            },
            volume: 0.25,
            default_identifier: String::from("artist"),
            tag_dictionary: std::collections::BTreeMap::from([(
                String::from("deep-kick"),
                String::from("sound-type"),
            )]),
            ..AppSettingsCore::default()
        },
        ..AppConfig::default()
    };

    save_to_path(&cfg, &path).unwrap();

    let text = std::fs::read_to_string(&path).unwrap();
    let value = text.parse::<toml::Value>().unwrap();
    let root = value.as_table().expect("saved settings table");

    assert!(root.get("runtime").is_some());
    assert!(root.get("paths").is_some());
    assert!(root.get("library").is_some());
    assert!(root.get("audio").is_some());
    assert!(root.get("interaction").is_some());
    assert!(root.get("similarity").is_some());
    assert!(root.get("naming").is_some());
    assert!(root.get("tags").is_some());
    assert!(
        root.get("volume").is_none(),
        "volume should be owned by the audio section"
    );
    assert!(
        root.get("trash_folder").is_none(),
        "trash_folder should be owned by the paths section"
    );

    let loaded = load_settings_from(&path).unwrap();
    assert_eq!(loaded.core.job_message_queue_capacity, 256);
    assert_eq!(
        loaded.core.trash_folder,
        Some(std::path::PathBuf::from("trash"))
    );
    assert_eq!(
        loaded
            .core
            .last_selected_source
            .as_ref()
            .map(|id| id.as_str()),
        Some(source_id.as_str())
    );
    assert_eq!(loaded.core.audio_output.host.as_deref(), Some("wasapi"));
    assert!((loaded.core.volume - 0.25).abs() < f32::EPSILON);
    assert_eq!(loaded.core.default_identifier, "artist");
    assert_eq!(
        loaded.core.collection_names.get("0").map(String::as_str),
        Some("Drums")
    );
    assert_eq!(
        loaded
            .core
            .tag_dictionary
            .get("deep-kick")
            .map(String::as_str),
        Some("sound-type")
    );
}

#[test]
fn volume_defaults_and_persists() {
    let env = TestConfigEnv::new();
    let path = env.path("cfg.toml");
    let mut cfg = AppConfig::default();
    assert_eq!(cfg.core.volume, 1.0);
    cfg.core.volume = 0.42;
    save_to_path(&cfg, &path).unwrap();
    let loaded = load_settings_from(&path).unwrap();
    assert!((loaded.core.volume - 0.42).abs() < f32::EPSILON);
}

#[test]
fn audio_output_defaults_and_persists() {
    let env = TestConfigEnv::new();
    let path = env.path("cfg.toml");
    let cfg = AppConfig {
        core: AppSettingsCore {
            audio_output: AudioOutputConfig {
                host: Some("asio".into()),
                device: Some("Test Interface".into()),
                sample_rate: Some(48_000),
                buffer_size: Some(512),
            },
            ..AppSettingsCore::default()
        },
        ..AppConfig::default()
    };

    save_to_path(&cfg, &path).unwrap();
    let loaded = load_settings_from(&path).unwrap();
    assert_eq!(loaded.core.audio_output.host.as_deref(), Some("asio"));
    assert_eq!(
        loaded.core.audio_output.device.as_deref(),
        Some("Test Interface")
    );
    assert_eq!(loaded.core.audio_output.sample_rate, Some(48_000));
    assert_eq!(loaded.core.audio_output.buffer_size, Some(512));
}

#[test]
fn audio_input_defaults_and_persists() {
    let env = TestConfigEnv::new();
    let path = env.path("cfg.toml");
    let cfg = AppConfig {
        core: AppSettingsCore {
            audio_input: AudioInputConfig {
                host: Some("asio".into()),
                device: Some("Test Mic".into()),
                sample_rate: Some(44_100),
                buffer_size: Some(256),
                channels: vec![1, 2],
            },
            ..AppSettingsCore::default()
        },
        ..AppConfig::default()
    };

    save_to_path(&cfg, &path).unwrap();
    let loaded = load_settings_from(&path).unwrap();
    assert_eq!(loaded.core.audio_input.host.as_deref(), Some("asio"));
    assert_eq!(loaded.core.audio_input.device.as_deref(), Some("Test Mic"));
    assert_eq!(loaded.core.audio_input.sample_rate, Some(44_100));
    assert_eq!(loaded.core.audio_input.buffer_size, Some(256));
    assert_eq!(loaded.core.audio_input.channels, vec![1, 2]);
}

#[test]
fn audio_write_format_defaults_and_persists() {
    let env = TestConfigEnv::new();
    let path = env.path("cfg.toml");
    let cfg = AppConfig {
        core: AppSettingsCore {
            audio_write_format: AudioWriteFormatConfig {
                sample_rate: AudioWriteSampleRate::Hz(48_000),
                sample_format: AudioWriteSampleFormat::Pcm24,
                channel_behavior: AudioWriteChannelBehavior::PreserveMonoStereo,
                dither: AudioWriteDither::None,
            },
            ..AppSettingsCore::default()
        },
        ..AppConfig::default()
    };

    save_to_path(&cfg, &path).unwrap();
    let loaded = load_settings_from(&path).unwrap();

    assert_eq!(loaded.core.audio_write_format, cfg.core.audio_write_format);
    assert_eq!(
        loaded.core.audio_write_format.summary_label(),
        "48 kHz, 24-bit PCM, Preserve mono/stereo, No dither"
    );
}

#[test]
fn trash_folder_round_trips() {
    let env = TestConfigEnv::new();
    let path = env.path("cfg.toml");
    let trash = std::path::PathBuf::from("trash_bin");
    let cfg = AppConfig {
        core: AppSettingsCore {
            trash_folder: Some(trash.clone()),
            ..AppSettingsCore::default()
        },
        ..AppConfig::default()
    };
    save_to_path(&cfg, &path).unwrap();
    let loaded = load_settings_from(&path).unwrap();
    assert_eq!(loaded.core.trash_folder, Some(trash));
}

#[test]
#[cfg(unix)]
fn settings_atomic_write_preserves_existing_on_failure() {
    use std::os::unix::fs::PermissionsExt;

    let env = TestConfigEnv::new();
    let path = env.path("cfg.toml");
    env.write(&path, "sentinel = true\n");

    let dir = path.parent().unwrap();
    let mut permissions = std::fs::metadata(dir).unwrap().permissions();
    permissions.set_mode(0o500);
    std::fs::set_permissions(dir, permissions).unwrap();

    let mut settings = AppSettings::default();
    settings.core.volume = 0.33;
    let result = save_settings_to_path(&settings, &path);
    assert!(result.is_err());

    let mut restore = std::fs::metadata(dir).unwrap().permissions();
    restore.set_mode(0o700);
    std::fs::set_permissions(dir, restore).unwrap();

    let contents = std::fs::read_to_string(&path).unwrap();
    assert_eq!(contents, "sentinel = true\n");
}

#[test]
#[cfg(target_os = "windows")]
fn settings_atomic_write_preserves_locked_destination_and_retries_cleanly() {
    use std::os::windows::fs::OpenOptionsExt;

    let env = TestConfigEnv::new();
    let path = env.path("cfg.toml");
    env.write(&path, "sentinel = true\n");
    let locked = std::fs::OpenOptions::new()
        .read(true)
        .share_mode(0)
        .open(&path)
        .unwrap();

    let mut settings = AppSettings::default();
    settings.core.volume = 0.33;
    let first_attempt = save_settings_to_path(&settings, &path);
    assert!(first_attempt.is_err());
    drop(locked);
    assert_eq!(std::fs::read_to_string(&path).unwrap(), "sentinel = true\n");

    save_settings_to_path(&settings, &path).unwrap();
    let loaded = load_settings_from(&path).unwrap();
    assert!((loaded.core.volume - 0.33).abs() < f32::EPSILON);

    let temp_prefix = format!("{}.tmp-", path.file_name().unwrap().to_string_lossy());
    assert!(
        std::fs::read_dir(path.parent().unwrap())
            .unwrap()
            .all(|entry| !entry
                .unwrap()
                .file_name()
                .to_string_lossy()
                .starts_with(&temp_prefix))
    );
}
