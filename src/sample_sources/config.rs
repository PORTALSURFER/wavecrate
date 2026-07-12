#[path = "config_defaults.rs"]
mod config_defaults;
#[path = "config_io/mod.rs"]
mod config_io;
#[path = "config_types/mod.rs"]
mod config_types;

pub use config_io::{
    CONFIG_FILE_NAME, ConfigSaveRevision, LEGACY_CONFIG_FILE_NAME, config_path, load_or_default,
    normalize_path, reserve_save_revision, save, save_if_revision_current, save_to_path,
};
pub use config_types::{
    AnalysisSettings, AppConfig, AppSettingsCore, AudioWriteChannelBehavior, AudioWriteDither,
    AudioWriteFormatConfig, AudioWriteSampleFormat, AudioWriteSampleRate, ConfigError,
    DEFAULT_RATING_DECAY_WEEKS, DropTargetColor, DropTargetConfig, FeatureFlags,
    InteractionOptions, MAX_RATING_DECAY_WEEKS, MIN_RATING_DECAY_WEEKS, SimilarityAspectControl,
    SimilarityAspectSettings, TooltipMode, UpdateChannel, UpdateSettings, clamp_rating_decay_weeks,
};
