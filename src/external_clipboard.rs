//! Platform helpers for copying file paths and text through the system clipboard.
//!
//! On Windows this publishes `CF_HDROP` for file paste targets such as Explorer
//! and `CF_UNICODETEXT` for plain text. On macOS this publishes both modern
//! pasteboard file URLs and the legacy filename-list flavor that older DAW
//! paste targets still consume.

use std::path::PathBuf;

#[cfg(target_os = "macos")]
mod macos;
#[cfg(all(not(target_os = "windows"), not(target_os = "macos")))]
mod unsupported;
#[cfg(target_os = "windows")]
mod windows;

#[cfg(target_os = "macos")]
use macos as platform;
#[cfg(all(not(target_os = "windows"), not(target_os = "macos")))]
use unsupported as platform;
#[cfg(target_os = "windows")]
use windows as platform;

/// Copy the given file paths to the system clipboard for pasting elsewhere.
pub fn copy_file_paths(paths: &[PathBuf]) -> Result<(), String> {
    if paths.is_empty() {
        return Err("No files to copy".into());
    }
    platform::copy_file_paths(paths)
}

/// Copy plain text to the system clipboard.
pub fn copy_text(text: &str) -> Result<(), String> {
    if text.is_empty() {
        return Err("No text to copy".into());
    }
    platform::copy_text(text)
}

/// Read file paths from the system clipboard (if available).
pub fn read_file_paths() -> Result<Vec<PathBuf>, String> {
    platform::read_file_paths()
}

/// Read plain text from the system clipboard.
pub fn read_text() -> Result<String, String> {
    platform::read_text()
}
