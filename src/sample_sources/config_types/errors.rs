use std::path::PathBuf;

use thiserror::Error;

/// Errors that may occur while loading or saving app configuration.
#[derive(Debug, Error)]
pub enum ConfigError {
    /// Failed to create the config directory.
    #[error("Unable to create config directory {path}: {source}")]
    CreateDir {
        /// Directory path that failed to create.
        path: PathBuf,
        /// Underlying IO error.
        source: std::io::Error,
    },
    /// Failed to read a config file.
    #[error("Failed to read {path}: {source}")]
    Read {
        /// Path that failed to read.
        path: PathBuf,
        /// Underlying IO error.
        source: std::io::Error,
    },
    /// Failed to write a config file.
    #[error("Failed to write {path}: {source}")]
    Write {
        /// Path that failed to write.
        path: PathBuf,
        /// Underlying IO error.
        source: std::io::Error,
    },
    /// Failed to parse TOML config.
    #[error("Invalid config at {path}: {source}")]
    ParseToml {
        /// TOML file path.
        path: PathBuf,
        /// TOML parse error.
        source: Box<toml::de::Error>,
    },
    /// Failed to parse legacy JSON config.
    #[error("Invalid legacy config at {path}: {source}")]
    ParseJson {
        /// JSON file path.
        path: PathBuf,
        /// JSON parse error.
        source: Box<serde_json::Error>,
    },
    /// Failed to serialize config to TOML.
    #[error("Failed to serialize config to TOML at {path}: {source}")]
    SerializeToml {
        /// TOML file path.
        path: PathBuf,
        /// TOML serialization error.
        source: Box<toml::ser::Error>,
    },
    /// Failed to migrate legacy config.
    #[error("Failed to migrate legacy config from {path}: {source}")]
    LegacyMigration {
        /// Legacy file path.
        path: PathBuf,
        /// Nested migration error.
        source: Box<ConfigError>,
    },
    /// Failed to back up legacy config.
    #[error("Failed to back up legacy config {path} to {backup_path}: {source}")]
    BackupLegacy {
        /// Legacy config path.
        path: PathBuf,
        /// Backup file path.
        backup_path: PathBuf,
        /// Underlying IO error.
        source: std::io::Error,
    },
    /// No usable config directory found.
    #[error("No suitable config directory found")]
    NoConfigDir,
    /// Failed to resolve the configured persistence profile.
    #[error("Invalid config persistence profile '{profile}'")]
    InvalidProfile {
        /// Rejected profile name.
        profile: String,
    },
    /// Library database error.
    #[error("Library database error: {0}")]
    Library(Box<crate::sample_sources::library::LibraryError>),
}

impl From<crate::sample_sources::library::LibraryError> for ConfigError {
    fn from(source: crate::sample_sources::library::LibraryError) -> Self {
        Self::Library(Box::new(source))
    }
}
