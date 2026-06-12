mod defaults;
mod drop_targets;
mod model;
mod sections;
mod serde_bridge;

pub use drop_targets::{DropTargetColor, DropTargetConfig};
pub(crate) use model::AppSettings;
pub use model::{AppConfig, AppSettingsCore, FeatureFlags};
