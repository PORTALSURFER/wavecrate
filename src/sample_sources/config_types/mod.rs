mod analysis;
mod app;
mod audio_write;
mod errors;
mod interaction;
mod updates;

pub use analysis::AnalysisSettings;
pub(crate) use app::AppSettings;
pub use app::{AppConfig, AppSettingsCore, DropTargetColor, DropTargetConfig, FeatureFlags};
pub use audio_write::{
    AudioWriteChannelBehavior, AudioWriteDither, AudioWriteFormatConfig, AudioWriteSampleFormat,
    AudioWriteSampleRate,
};
pub use errors::ConfigError;
pub use interaction::{InteractionOptions, TooltipMode};
pub use updates::{UpdateChannel, UpdateSettings};
