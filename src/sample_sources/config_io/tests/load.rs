use super::super::super::config_defaults::MAX_ANALYSIS_WORKER_COUNT;
use super::super::super::config_types::AppSettings;
use super::super::CONFIG_FILE_NAME;
use super::super::load::{apply_app_data_dir, load_settings_from};
use super::super::save::save_settings_to_path;
use super::TestConfigEnv;

#[test]
fn load_settings_from_accepts_nested_sections() {
    let env = TestConfigEnv::new();
    let path = env.path("cfg.toml");
    let data = r#"
[runtime]
job_message_queue_capacity = 256

[paths]
app_data_dir = "data"
trash_folder = "trash"

[library]
drop_targets = ["drop-a"]
last_selected_source = "source::test"
upper_folder_pane_source = "source::test"
active_folder_pane = "upper"

[library.collection_names]
"0" = "Drums"
"9" = "Ignored"

[audio]
volume = 0.25

[audio.output]
host = "wasapi"
device = "Studio"
sample_rate = 48000

[interaction.controls]
invert_waveform_scroll = false

[naming]
default_identifier = "artist"

[tags.dictionary]
deep-kick = "sound-type"
"#;
    env.write(&path, data);

    let loaded = load_settings_from(&path).unwrap();

    assert_eq!(loaded.core.job_message_queue_capacity, 256);
    assert_eq!(
        loaded.core.app_data_dir,
        Some(std::path::PathBuf::from("data"))
    );
    assert_eq!(
        loaded.core.trash_folder,
        Some(std::path::PathBuf::from("trash"))
    );
    assert_eq!(loaded.core.drop_targets.len(), 1);
    assert_eq!(
        loaded.core.drop_targets[0].path,
        std::path::PathBuf::from("drop-a")
    );
    assert_eq!(
        loaded
            .core
            .last_selected_source
            .as_ref()
            .map(|id| id.as_str()),
        Some("source::test")
    );
    assert_eq!(loaded.core.active_folder_pane.as_deref(), Some("upper"));
    assert_eq!(
        loaded.core.collection_names.get("0").map(String::as_str),
        Some("Drums")
    );
    assert!(
        !loaded.core.collection_names.contains_key("9"),
        "invalid collection slots should be ignored"
    );
    assert!((loaded.core.volume - 0.25).abs() < f32::EPSILON);
    assert_eq!(loaded.core.audio_output.host.as_deref(), Some("wasapi"));
    assert_eq!(loaded.core.audio_output.device.as_deref(), Some("Studio"));
    assert_eq!(loaded.core.audio_output.sample_rate, Some(48_000));
    assert!(!loaded.core.controls.invert_waveform_scroll);
    assert_eq!(loaded.core.default_identifier, "artist");
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
fn audio_input_channels_accepts_single_value() {
    let env = TestConfigEnv::new();
    let path = env.path("cfg.toml");
    let data = r#"
[core.audio_input]
host = "asio"
device = "Test Mic"
channels = 1
"#;
    env.write(&path, data);
    let loaded = load_settings_from(&path).unwrap();
    assert_eq!(loaded.core.audio_input.channels, vec![1]);
}

#[test]
fn clamps_volume_and_worker_count_on_load() {
    let env = TestConfigEnv::new();
    let path = env.path("cfg.toml");
    let data = r#"
volume = 2.5

[analysis]
analysis_worker_count = 999
"#;
    env.write(&path, data);
    let loaded = load_settings_from(&path).unwrap();
    assert!((loaded.core.volume - 1.0).abs() < f32::EPSILON);
    assert_eq!(
        loaded.core.analysis.analysis_worker_count,
        MAX_ANALYSIS_WORKER_COUNT
    );
}

#[test]
fn load_settings_from_merges_core_table() {
    let env = TestConfigEnv::new();
    let path = env.path("cfg.toml");
    let data = r#"
volume = 0.75

[core]
volume = 0.25
"#;
    env.write(&path, data);
    let loaded = load_settings_from(&path).unwrap();
    assert!((loaded.core.volume - 0.75).abs() < f32::EPSILON);
}

#[test]
fn apply_app_data_dir_loads_override_when_exists() {
    let env = TestConfigEnv::new();
    let settings_path = env.path("cfg.toml");
    let override_dir = env.path("override");
    std::fs::create_dir_all(&override_dir).unwrap();
    let override_path = override_dir.join(CONFIG_FILE_NAME);

    let mut settings = AppSettings::default();
    settings.core.volume = 0.9;
    settings.core.app_data_dir = Some(override_dir.clone());

    let mut override_settings = AppSettings::default();
    override_settings.core.volume = 0.33;
    save_settings_to_path(&override_settings, &override_path).unwrap();

    apply_app_data_dir(&settings_path, &mut settings).unwrap();

    assert!((settings.core.volume - 0.33).abs() < f32::EPSILON);
    assert_eq!(settings.core.app_data_dir, Some(override_dir.clone()));
    assert_eq!(crate::app_dirs::app_root_dir().unwrap(), override_dir);
}

#[test]
fn apply_app_data_dir_copies_settings_when_missing() {
    let env = TestConfigEnv::new();
    let settings_path = env.path("cfg.toml");
    let override_dir = env.path("override");
    std::fs::create_dir_all(&override_dir).unwrap();
    let override_path = override_dir.join(CONFIG_FILE_NAME);

    let mut settings = AppSettings::default();
    settings.core.volume = 0.47;
    settings.core.app_data_dir = Some(override_dir.clone());
    save_settings_to_path(&settings, &settings_path).unwrap();

    apply_app_data_dir(&settings_path, &mut settings).unwrap();

    let loaded_override = load_settings_from(&override_path).unwrap();
    assert!((loaded_override.core.volume - 0.47).abs() < f32::EPSILON);
    assert_eq!(settings.core.app_data_dir, Some(override_dir.clone()));
    assert_eq!(crate::app_dirs::app_root_dir().unwrap(), override_dir);
}
