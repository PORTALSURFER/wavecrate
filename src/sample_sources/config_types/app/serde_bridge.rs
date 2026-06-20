use std::{collections::BTreeMap, path::PathBuf};

use serde::{Deserialize, Deserializer, Serialize, Serializer};

use crate::{
    audio::{AudioInputConfig, AudioOutputConfig},
    sample_sources::SourceId,
};

use super::{
    super::{
        AnalysisSettings, AudioWriteFormatConfig, InteractionOptions, SimilarityAspectSettings,
        UpdateSettings,
    },
    drop_targets::{DropTargetConfig, deserialize_optional_drop_targets},
    model::{AppSettingsCore, FeatureFlags},
    sections::{
        AudioSettings, InteractionSettings, LibrarySettings, NamingSettings, PathSettings,
        RuntimeSettings, TagDictionarySettings,
    },
};

/// Current nested settings shape written to TOML.
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
    similarity: &'a SimilarityAspectSettings,
    naming: NamingSettings,
    tags: TagDictionarySettings,
}

/// Compatibility input shape for current nested config plus legacy flat keys.
///
/// The flat fields are migration bridge code: they remain accepted on load, but
/// normal saves always use [`AppSettingsCorePersisted`].
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
    similarity: SimilarityAspectSettings,
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
    collection_names: Option<BTreeMap<String, String>>,
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
    similarity_aspects: Option<SimilarityAspectSettings>,
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
            collection_names: self
                .collection_names
                .unwrap_or(self.library.collection_names),
            audio_output: self.audio_output.unwrap_or(self.audio.output),
            audio_input: self.audio_input.unwrap_or(self.audio.input),
            audio_write_format: self.audio_write_format.unwrap_or(self.audio.write_format),
            volume: self.volume.unwrap_or(self.audio.volume),
            controls: self.controls.unwrap_or(self.interaction.controls),
            similarity: self.similarity_aspects.unwrap_or(self.similarity),
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
                collection_names: self.collection_names.clone(),
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
            similarity: &self.similarity,
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
