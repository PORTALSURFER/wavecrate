use std::{collections::BTreeMap, path::PathBuf};

use serde::{Deserialize, Serialize};

use crate::{
    audio::{AudioInputConfig, AudioOutputConfig},
    sample_sources::SourceId,
};

use super::{
    super::{
        super::config_defaults::{
            default_audio_input, default_audio_output, default_job_message_queue_capacity,
            default_volume,
        },
        AudioWriteFormatConfig, InteractionOptions,
    },
    defaults::default_identifier,
    drop_targets::{DropTargetConfig, deserialize_drop_targets},
};

/// Runtime/job-queue settings.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub(super) struct RuntimeSettings {
    #[serde(default = "default_job_message_queue_capacity")]
    /// Maximum number of pending controller job messages.
    pub(super) job_message_queue_capacity: u32,
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
pub(super) struct PathSettings {
    /// Optional override for the `.wavecrate` data folder.
    #[serde(default)]
    pub(super) app_data_dir: Option<PathBuf>,
    /// Optional trash folder path.
    #[serde(default)]
    pub(super) trash_folder: Option<PathBuf>,
}

/// Source-selection and library-sidebar settings.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub(super) struct LibrarySettings {
    /// User-defined drop target folders used by the sidebar, with optional colors.
    #[serde(default, deserialize_with = "deserialize_drop_targets")]
    pub(super) drop_targets: Vec<DropTargetConfig>,
    /// Last selected source id.
    #[serde(default)]
    pub(super) last_selected_source: Option<SourceId>,
    /// Source assigned to the upper sidebar folder pane.
    #[serde(default)]
    pub(super) upper_folder_pane_source: Option<SourceId>,
    /// Source assigned to the lower sidebar folder pane.
    #[serde(default)]
    pub(super) lower_folder_pane_source: Option<SourceId>,
    /// Active folder pane id encoded as `"upper"` or `"lower"`.
    #[serde(default)]
    pub(super) active_folder_pane: Option<String>,
    /// User-authored collection labels, keyed by fixed collection index.
    #[serde(default)]
    pub(super) collection_names: BTreeMap<String, String>,
}

/// Audio IO, write-format, and playback-volume settings.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub(super) struct AudioSettings {
    #[serde(default = "default_audio_output")]
    /// Output audio configuration.
    pub(super) output: AudioOutputConfig,
    #[serde(default = "default_audio_input")]
    /// Input audio configuration.
    pub(super) input: AudioInputConfig,
    #[serde(default)]
    /// Audio write-format policy for Wavecrate-created WAV files.
    pub(super) write_format: AudioWriteFormatConfig,
    #[serde(default = "default_volume")]
    /// Master volume (0.0-1.0).
    pub(super) volume: f32,
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
pub(super) struct InteractionSettings {
    #[serde(default)]
    pub(super) controls: InteractionOptions,
}

/// Naming defaults for generated sample names.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub(super) struct NamingSettings {
    #[serde(default = "default_identifier")]
    /// Global creator or artist identifier used by sample auto-rename.
    pub(super) default_identifier: String,
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
pub(super) struct TagDictionarySettings {
    #[serde(default)]
    /// Global user-authored tag dictionary, keyed by normalized tag value with fixed category ids.
    pub(super) dictionary: BTreeMap<String, String>,
}
