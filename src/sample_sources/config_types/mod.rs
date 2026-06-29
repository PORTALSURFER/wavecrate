mod analysis;
mod app;
mod audio_write;
mod errors;
mod interaction;
mod similarity;
mod updates;

pub use analysis::AnalysisSettings;
pub(crate) use app::AppSettings;
pub use app::{AppConfig, AppSettingsCore, DropTargetColor, DropTargetConfig, FeatureFlags};
pub use audio_write::{
    AudioWriteChannelBehavior, AudioWriteDither, AudioWriteFormatConfig, AudioWriteSampleFormat,
    AudioWriteSampleRate,
};
pub use errors::ConfigError;
pub use interaction::{
    DEFAULT_RATING_DECAY_WEEKS, InteractionOptions, MAX_RATING_DECAY_WEEKS, MIN_RATING_DECAY_WEEKS,
    TooltipMode, clamp_rating_decay_weeks,
};
pub use similarity::{SimilarityAspectControl, SimilarityAspectSettings};
pub use updates::{UpdateChannel, UpdateSettings};
