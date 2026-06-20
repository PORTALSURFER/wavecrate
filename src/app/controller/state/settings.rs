//! Persistent settings state for the controller.

use crate::audio::{AudioInputConfig, AudioOutputConfig};
use crate::sample_sources::config::{
    AudioWriteFormatConfig, DropTargetConfig, SimilarityAspectSettings,
};
use std::{collections::BTreeMap, path::PathBuf};

pub(crate) struct AppSettingsState {
    pub(crate) feature_flags: crate::sample_sources::config::FeatureFlags,
    pub(crate) analysis: crate::sample_sources::config::AnalysisSettings,
    pub(crate) updates: crate::sample_sources::config::UpdateSettings,
    /// Maximum number of pending controller job messages.
    pub(crate) job_message_queue_capacity: u32,
    pub(crate) app_data_dir: Option<PathBuf>,
    pub(crate) audio_output: AudioOutputConfig,
    pub(crate) audio_input: AudioInputConfig,
    pub(crate) audio_write_format: AudioWriteFormatConfig,
    pub(crate) controls: crate::sample_sources::config::InteractionOptions,
    pub(crate) similarity: SimilarityAspectSettings,
    pub(crate) trash_folder: Option<PathBuf>,
    pub(crate) drop_targets: Vec<DropTargetConfig>,
    pub(crate) collection_names: BTreeMap<String, String>,
    /// Global creator or artist identifier used by sample auto-rename.
    pub(crate) default_identifier: String,
    pub(crate) tag_dictionary: BTreeMap<String, String>,
}

impl AppSettingsState {
    pub(crate) fn new() -> Self {
        Self {
            feature_flags: crate::sample_sources::config::FeatureFlags::default(),
            analysis: crate::sample_sources::config::AnalysisSettings::default(),
            updates: crate::sample_sources::config::UpdateSettings::default(),
            job_message_queue_capacity: crate::sample_sources::config::AppSettingsCore::default()
                .job_message_queue_capacity,
            app_data_dir: None,
            audio_output: AudioOutputConfig::default(),
            audio_input: AudioInputConfig::default(),
            audio_write_format: AudioWriteFormatConfig::default(),
            controls: crate::sample_sources::config::InteractionOptions::default(),
            similarity: SimilarityAspectSettings::default(),
            trash_folder: None,
            drop_targets: Vec::new(),
            collection_names: BTreeMap::new(),
            default_identifier: String::from("portal"),
            tag_dictionary: BTreeMap::new(),
        }
    }
}
