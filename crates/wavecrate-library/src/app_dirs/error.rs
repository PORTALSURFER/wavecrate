//! Error types for application-directory resolution.

use std::path::PathBuf;

use thiserror::Error;

/// Errors that can occur while resolving or preparing application directories.
#[derive(Debug, Error)]
pub enum AppDirError {
    /// No suitable base config directory could be resolved.
    #[error("No suitable base config directory available for application files")]
    NoBaseDir,
    /// Failed to create the application directory.
    #[error("Failed to create application directory at {path}: {source}")]
    CreateDir {
        /// Path that told the directory to be created.
        path: PathBuf,
        /// Underlying IO error.
        source: std::io::Error,
    },
    /// The configured profile name cannot be represented safely on disk.
    #[error("Invalid Wavecrate profile name '{profile}'")]
    InvalidProfileName {
        /// Rejected profile name.
        profile: String,
    },
}
