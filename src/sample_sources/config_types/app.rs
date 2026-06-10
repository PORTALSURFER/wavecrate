use std::{collections::BTreeMap, path::PathBuf};

use serde::{Deserialize, Deserializer, Serialize, Serializer};

use crate::{
    audio::{AudioInputConfig, AudioOutputConfig},
    sample_sources::library::LibraryState,
    sample_sources::{SampleSource, SourceId},
};

use super::super::config_defaults::{
    clamp_analysis_worker_count, clamp_job_message_queue_capacity, clamp_volume,
    default_audio_input, default_audio_output, default_job_message_queue_capacity, default_true,
    default_volume,
};
use super::{AnalysisSettings, AudioWriteFormatConfig, InteractionOptions, UpdateSettings};

/// Aggregate application state loaded from disk.
///
/// Config keys (TOML): `feature_flags`, `analysis`, `updates`, `app_data_dir`,
/// `trash_folder`, `drop_targets`, `last_selected_source`,
/// `upper_folder_pane_source`, `lower_folder_pane_source`, `active_folder_pane`,
/// `volume`, `audio_output`, `audio_input`, `audio_write_format`, `controls`,
/// `job_message_queue_capacity`, `default_identifier`, `tag_dictionary`.
///
/// `sources` are stored in the library database.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct AppConfig {
    /// Sample sources loaded from the library database.
    pub sources: Vec<SampleSource>,
    #[serde(default, flatten)]
    /// Core settings persisted in the config file.
    pub core: AppSettingsCore,
}

/// App settings that belong in the TOML config file.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub(crate) struct AppSettings {
    #[serde(default, flatten)]
    pub core: AppSettingsCore,
}

impl AppSettings {
    pub(crate) fn normalized(self) -> Self {
        Self {
            core: self.core.normalized(),
        }
    }
}

impl From<&AppConfig> for AppSettings {
    fn from(config: &AppConfig) -> Self {
        Self {
            core: config.core.clone(),
        }
    }
}

impl From<(AppSettings, LibraryState)> for AppConfig {
    fn from((settings, library): (AppSettings, LibraryState)) -> Self {
        Self {
            sources: library.sources,
            core: settings.core,
        }
    }
}

/// Runtime view of shared config fields used across app config surfaces.
///
/// New TOML saves use explicit nested sections through the custom serde bridge
/// below. Legacy flat keys are still accepted on load so existing user configs
/// and migrated JSON settings remain compatible.
#[derive(Debug, Clone)]
pub struct AppSettingsCore {
    /// Feature flags for experimental or optional UI behavior.
    pub feature_flags: FeatureFlags,
    /// Analysis settings.
    pub analysis: AnalysisSettings,
    /// Update check settings.
    pub updates: UpdateSettings,
    /// Maximum number of pending controller job messages.
    pub job_message_queue_capacity: u32,
    /// Optional override for the `.wavecrate` data folder.
    pub app_data_dir: Option<PathBuf>,
    /// Optional trash folder path.
    pub trash_folder: Option<PathBuf>,
    /// Drop target configurations for the sidebar.
    pub drop_targets: Vec<DropTargetConfig>,
    /// Last selected source id.
    pub last_selected_source: Option<SourceId>,
    /// Source assigned to the upper sidebar folder pane.
    pub upper_folder_pane_source: Option<SourceId>,
    /// Source assigned to the lower sidebar folder pane.
    pub lower_folder_pane_source: Option<SourceId>,
    /// Active folder pane id encoded as `"upper"` or `"lower"`.
    pub active_folder_pane: Option<String>,
    /// Output audio configuration.
    pub audio_output: AudioOutputConfig,
    /// Input audio configuration.
    pub audio_input: AudioInputConfig,
    /// Audio write-format policy for Wavecrate-created WAV files.
    pub audio_write_format: AudioWriteFormatConfig,
    /// Master volume (0.0-1.0).
    pub volume: f32,
    /// Interaction option defaults.
    pub controls: InteractionOptions,
    /// Global creator or artist identifier used by sample auto-rename.
    pub default_identifier: String,
    /// Global user-authored tag dictionary, keyed by normalized tag value with fixed category ids.
    pub tag_dictionary: BTreeMap<String, String>,
}

/// Runtime/job-queue settings.
#[derive(Debug, Clone, Serialize, Deserialize)]
struct RuntimeSettings {
    #[serde(default = "default_job_message_queue_capacity")]
    /// Maximum number of pending controller job messages.
    pub job_message_queue_capacity: u32,
}

impl Default for RuntimeSettings {
    fn default() -> Self {
        Self {
            job_message_queue_capacity: default_job_message_queue_capacity(),
        }
    }
}

/// Config paths for Wavecrate-owned app data and library hygiene folders.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
struct PathSettings {
    /// Optional override for the `.wavecrate` data folder.
    #[serde(default)]
    pub app_data_dir: Option<PathBuf>,
    /// Optional trash folder path.
    #[serde(default)]
    pub trash_folder: Option<PathBuf>,
}

/// Source-selection and library-sidebar settings.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
struct LibrarySettings {
    /// User-defined drop target folders used by the sidebar, with optional colors.
    #[serde(default, deserialize_with = "deserialize_drop_targets")]
    pub drop_targets: Vec<DropTargetConfig>,
    /// Last selected source id.
    #[serde(default)]
    pub last_selected_source: Option<SourceId>,
    /// Source assigned to the upper sidebar folder pane.
    #[serde(default)]
    pub upper_folder_pane_source: Option<SourceId>,
    /// Source assigned to the lower sidebar folder pane.
    #[serde(default)]
    pub lower_folder_pane_source: Option<SourceId>,
    /// Active folder pane id encoded as `"upper"` or `"lower"`.
    #[serde(default)]
    pub active_folder_pane: Option<String>,
}

/// Audio IO, write-format, and playback-volume settings.
#[derive(Debug, Clone, Serialize, Deserialize)]
struct AudioSettings {
    #[serde(default = "default_audio_output")]
    /// Output audio configuration.
    pub output: AudioOutputConfig,
    #[serde(default = "default_audio_input")]
    /// Input audio configuration.
    pub input: AudioInputConfig,
    #[serde(default)]
    /// Audio write-format policy for Wavecrate-created WAV files.
    pub write_format: AudioWriteFormatConfig,
    #[serde(default = "default_volume")]
    /// Master volume (0.0-1.0).
    pub volume: f32,
}

impl Default for AudioSettings {
    fn default() -> Self {
        Self {
            output: default_audio_output(),
            input: default_audio_input(),
            write_format: AudioWriteFormatConfig::default(),
            volume: default_volume(),
        }
    }
}

/// Interaction option defaults.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
struct InteractionSettings {
    #[serde(default)]
    pub controls: InteractionOptions,
}

/// Naming defaults for generated sample names.
#[derive(Debug, Clone, Serialize, Deserialize)]
struct NamingSettings {
    #[serde(default = "default_identifier")]
    /// Global creator or artist identifier used by sample auto-rename.
    pub default_identifier: String,
}

impl Default for NamingSettings {
    fn default() -> Self {
        Self {
            default_identifier: default_identifier(),
        }
    }
}

/// User-authored tag dictionary settings.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
struct TagDictionarySettings {
    #[serde(default)]
    /// Global user-authored tag dictionary, keyed by normalized tag value with fixed category ids.
    pub dictionary: BTreeMap<String, String>,
}

#[derive(Serialize)]
struct AppSettingsCorePersisted<'a> {
    feature_flags: &'a FeatureFlags,
    analysis: &'a AnalysisSettings,
    updates: &'a UpdateSettings,
    runtime: RuntimeSettings,
    paths: PathSettings,
    library: LibrarySettings,
    audio: AudioSettings,
    interaction: InteractionSettings,
    naming: NamingSettings,
    tags: TagDictionarySettings,
}

#[derive(Deserialize)]
struct AppSettingsCoreWire {
    #[serde(default)]
    feature_flags: FeatureFlags,
    #[serde(default)]
    analysis: AnalysisSettings,
    #[serde(default)]
    updates: UpdateSettings,
    #[serde(default)]
    runtime: RuntimeSettings,
    #[serde(default)]
    paths: PathSettings,
    #[serde(default)]
    library: LibrarySettings,
    #[serde(default)]
    audio: AudioSettings,
    #[serde(default)]
    interaction: InteractionSettings,
    #[serde(default)]
    naming: NamingSettings,
    #[serde(default)]
    tags: TagDictionarySettings,

    #[serde(default)]
    job_message_queue_capacity: Option<u32>,
    #[serde(default)]
    app_data_dir: Option<PathBuf>,
    #[serde(default)]
    trash_folder: Option<PathBuf>,
    #[serde(default, deserialize_with = "deserialize_optional_drop_targets")]
    drop_targets: Option<Vec<DropTargetConfig>>,
    #[serde(default)]
    last_selected_source: Option<SourceId>,
    #[serde(default)]
    upper_folder_pane_source: Option<SourceId>,
    #[serde(default)]
    lower_folder_pane_source: Option<SourceId>,
    #[serde(default)]
    active_folder_pane: Option<String>,
    #[serde(default)]
    audio_output: Option<AudioOutputConfig>,
    #[serde(default)]
    audio_input: Option<AudioInputConfig>,
    #[serde(default)]
    audio_write_format: Option<AudioWriteFormatConfig>,
    #[serde(default)]
    volume: Option<f32>,
    #[serde(default)]
    controls: Option<InteractionOptions>,
    #[serde(default)]
    default_identifier: Option<String>,
    #[serde(default)]
    tag_dictionary: Option<BTreeMap<String, String>>,
}

impl AppSettingsCoreWire {
    fn into_core(self) -> AppSettingsCore {
        AppSettingsCore {
            feature_flags: self.feature_flags,
            analysis: self.analysis,
            updates: self.updates,
            job_message_queue_capacity: self
                .job_message_queue_capacity
                .unwrap_or(self.runtime.job_message_queue_capacity),
            app_data_dir: self.app_data_dir.or(self.paths.app_data_dir),
            trash_folder: self.trash_folder.or(self.paths.trash_folder),
            drop_targets: self.drop_targets.unwrap_or(self.library.drop_targets),
            last_selected_source: self
                .last_selected_source
                .or(self.library.last_selected_source),
            upper_folder_pane_source: self
                .upper_folder_pane_source
                .or(self.library.upper_folder_pane_source),
            lower_folder_pane_source: self
                .lower_folder_pane_source
                .or(self.library.lower_folder_pane_source),
            active_folder_pane: self.active_folder_pane.or(self.library.active_folder_pane),
            audio_output: self.audio_output.unwrap_or(self.audio.output),
            audio_input: self.audio_input.unwrap_or(self.audio.input),
            audio_write_format: self.audio_write_format.unwrap_or(self.audio.write_format),
            volume: self.volume.unwrap_or(self.audio.volume),
            controls: self.controls.unwrap_or(self.interaction.controls),
            default_identifier: self
                .default_identifier
                .unwrap_or(self.naming.default_identifier),
            tag_dictionary: self.tag_dictionary.unwrap_or(self.tags.dictionary),
        }
    }
}

impl Serialize for AppSettingsCore {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        AppSettingsCorePersisted {
            feature_flags: &self.feature_flags,
            analysis: &self.analysis,
            updates: &self.updates,
            runtime: RuntimeSettings {
                job_message_queue_capacity: self.job_message_queue_capacity,
            },
            paths: PathSettings {
                app_data_dir: self.app_data_dir.clone(),
                trash_folder: self.trash_folder.clone(),
            },
            library: LibrarySettings {
                drop_targets: self.drop_targets.clone(),
                last_selected_source: self.last_selected_source.clone(),
                upper_folder_pane_source: self.upper_folder_pane_source.clone(),
                lower_folder_pane_source: self.lower_folder_pane_source.clone(),
                active_folder_pane: self.active_folder_pane.clone(),
            },
            audio: AudioSettings {
                output: self.audio_output.clone(),
                input: self.audio_input.clone(),
                write_format: self.audio_write_format.clone(),
                volume: self.volume,
            },
            interaction: InteractionSettings {
                controls: self.controls.clone(),
            },
            naming: NamingSettings {
                default_identifier: self.default_identifier.clone(),
            },
            tags: TagDictionarySettings {
                dictionary: self.tag_dictionary.clone(),
            },
        }
        .serialize(serializer)
    }
}

impl<'de> Deserialize<'de> for AppSettingsCore {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        Ok(AppSettingsCoreWire::deserialize(deserializer)?.into_core())
    }
}

impl AppSettingsCore {
    pub(super) fn normalized(mut self) -> Self {
        self.volume = clamp_volume(self.volume);
        self.analysis.analysis_worker_count =
            clamp_analysis_worker_count(self.analysis.analysis_worker_count);
        self.job_message_queue_capacity =
            clamp_job_message_queue_capacity(self.job_message_queue_capacity);
        self
    }
}

/// Persisted color choices for drop target rows.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum DropTargetColor {
    /// Mint accent color.
    Mint,
    /// Ice accent color.
    Ice,
    /// Copper accent color.
    Copper,
    /// Fog accent color.
    Fog,
    /// Amber accent color.
    Amber,
    /// Rose accent color.
    Rose,
    /// Spruce accent color.
    Spruce,
    /// Clay accent color.
    Clay,
}

/// Config data for a single drop target.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DropTargetConfig {
    /// Folder path that receives dropped samples.
    pub path: PathBuf,
    /// Optional display color selected for the target.
    pub color: Option<DropTargetColor>,
}

impl DropTargetConfig {
    /// Build a drop target entry for the given path, with no color assigned.
    pub fn new(path: PathBuf) -> Self {
        Self { path, color: None }
    }
}

/// Toggleable features that can be persisted and evolve without breaking old configs.
///
/// Config keys: `autoplay_selection`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FeatureFlags {
    #[serde(default = "default_true")]
    /// Auto-play when selection changes.
    pub autoplay_selection: bool,
}

impl Default for FeatureFlags {
    fn default() -> Self {
        Self {
            autoplay_selection: true,
        }
    }
}

impl Default for AppSettingsCore {
    fn default() -> Self {
        Self {
            feature_flags: FeatureFlags::default(),
            analysis: AnalysisSettings::default(),
            updates: UpdateSettings::default(),
            job_message_queue_capacity: default_job_message_queue_capacity(),
            app_data_dir: None,
            trash_folder: None,
            drop_targets: Vec::new(),
            last_selected_source: None,
            upper_folder_pane_source: None,
            lower_folder_pane_source: None,
            active_folder_pane: None,
            audio_output: default_audio_output(),
            audio_input: default_audio_input(),
            audio_write_format: AudioWriteFormatConfig::default(),
            volume: default_volume(),
            controls: InteractionOptions::default(),
            default_identifier: default_identifier(),
            tag_dictionary: BTreeMap::new(),
        }
    }
}

fn default_identifier() -> String {
    String::from("portal")
}

fn deserialize_drop_targets<'de, D>(deserializer: D) -> Result<Vec<DropTargetConfig>, D::Error>
where
    D: Deserializer<'de>,
{
    #[derive(Deserialize)]
    #[serde(untagged)]
    enum DropTargetEntry {
        Path(PathBuf),
        Config(DropTargetConfig),
    }

    let items = Option::<Vec<DropTargetEntry>>::deserialize(deserializer)?.unwrap_or_default();
    Ok(items
        .into_iter()
        .map(|item| match item {
            DropTargetEntry::Path(path) => DropTargetConfig::new(path),
            DropTargetEntry::Config(config) => config,
        })
        .collect())
}

fn deserialize_optional_drop_targets<'de, D>(
    deserializer: D,
) -> Result<Option<Vec<DropTargetConfig>>, D::Error>
where
    D: Deserializer<'de>,
{
    #[derive(Deserialize)]
    #[serde(untagged)]
    enum DropTargetEntry {
        Path(PathBuf),
        Config(DropTargetConfig),
    }

    Ok(
        Option::<Vec<DropTargetEntry>>::deserialize(deserializer)?.map(|items| {
            items
                .into_iter()
                .map(|item| match item {
                    DropTargetEntry::Path(path) => DropTargetConfig::new(path),
                    DropTargetEntry::Config(config) => config,
                })
                .collect()
        }),
    )
}
