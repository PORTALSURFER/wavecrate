use std::{collections::BTreeMap, path::PathBuf};

use serde::{Deserialize, Serialize};

use crate::{
    audio::{AudioInputConfig, AudioOutputConfig},
    sample_sources::library::LibraryState,
    sample_sources::{SampleCollection, SampleSource, SourceId},
};

use super::{
    super::{
        super::config_defaults::{
            clamp_analysis_worker_count, clamp_job_message_queue_capacity, clamp_volume,
            default_audio_input, default_audio_output, default_job_message_queue_capacity,
            default_true, default_volume,
        },
        AnalysisSettings, AudioWriteFormatConfig, InteractionOptions, SimilarityAspectSettings,
        UpdateSettings,
    },
    defaults::default_identifier,
    drop_targets::DropTargetConfig,
};

/// Aggregate application state loaded from disk.
///
/// Config keys (TOML): `feature_flags`, `analysis`, `updates`, `app_data_dir`,
/// `trash_folder`, `drop_targets`, `last_selected_source`,
/// `upper_folder_pane_source`, `lower_folder_pane_source`, `active_folder_pane`,
/// `collection_names`, `folder_locks`,
/// `volume`, `audio_output`, `audio_input`, `audio_write_format`, `controls`,
/// `job_message_queue_capacity`, `default_identifier`, `tag_dictionary`,
/// `similarity`.
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
/// New TOML saves use explicit nested sections through the serde bridge module.
/// Legacy flat keys are still accepted on load so existing user configs and
/// migrated JSON settings remain compatible.
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
    /// User-authored collection labels, keyed by fixed collection index.
    pub collection_names: BTreeMap<String, String>,
    /// Folder roots protected from Wavecrate file mutations.
    pub folder_locks: Vec<PathBuf>,
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
    /// Similarity aspect controls and ranking preferences.
    pub similarity: SimilarityAspectSettings,
    /// Global creator or artist identifier used by sample auto-rename.
    pub default_identifier: String,
    /// Global user-authored tag dictionary, keyed by normalized tag value with fixed category ids.
    pub tag_dictionary: BTreeMap<String, String>,
}

impl AppSettingsCore {
    pub(super) fn normalized(mut self) -> Self {
        self.volume = clamp_volume(self.volume);
        self.analysis.analysis_worker_count =
            clamp_analysis_worker_count(self.analysis.analysis_worker_count);
        self.job_message_queue_capacity =
            clamp_job_message_queue_capacity(self.job_message_queue_capacity);
        self.similarity = self.similarity.normalized();
        self.collection_names = normalized_collection_names(self.collection_names);
        self
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
            collection_names: BTreeMap::new(),
            folder_locks: Vec::new(),
            audio_output: default_audio_output(),
            audio_input: default_audio_input(),
            audio_write_format: AudioWriteFormatConfig::default(),
            volume: default_volume(),
            controls: InteractionOptions::default(),
            similarity: SimilarityAspectSettings::default(),
            default_identifier: default_identifier(),
            tag_dictionary: BTreeMap::new(),
        }
    }
}

fn normalized_collection_names(names: BTreeMap<String, String>) -> BTreeMap<String, String> {
    names
        .into_iter()
        .filter_map(|(key, value)| {
            let index = key.parse::<u8>().ok()?;
            SampleCollection::new(index)?;
            let label = value.trim();
            (!label.is_empty()).then(|| (index.to_string(), label.to_string()))
        })
        .collect()
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
