#![deny(missing_docs)]
#![deny(warnings)]

//! Shared application-directory and library-storage helpers.

/// Application directory helpers anchored to the `.sempal` root.
pub mod app_dirs;
mod env_flags;
/// Sample-source identifiers and persistent storage helpers.
pub mod sample_sources;
/// Optional SQLite extension loader shared by storage code.
pub mod sqlite_ext;
