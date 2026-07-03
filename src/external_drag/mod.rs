//! Platform helpers for starting external drag-and-drop operations.
//!
//! Implemented for Windows by emitting a `CF_HDROP` drag and for macOS by
//! starting an AppKit file-URL dragging session. Other platforms return an
//! unsupported error to keep behaviour predictable.

use std::path::PathBuf;

/// Start dragging the given file paths to an external target.
///
/// Returns an error if the platform does not support outgoing drags.
#[cfg(target_os = "windows")]
pub fn start_file_drag(
    hwnd: windows::Win32::Foundation::HWND,
    paths: &[PathBuf],
) -> Result<(), String> {
    if paths.is_empty() {
        return Err("No files to drag".into());
    }
    platform::start_file_drag(hwnd, paths)
}

#[cfg(target_os = "macos")]
/// Start dragging the given file paths to an external target.
///
/// The active AppKit key/main window and content view are used as the native
/// drag anchor.
pub fn start_file_drag(_anchor: (), paths: &[PathBuf]) -> Result<(), String> {
    if paths.is_empty() {
        return Err("No files to drag".into());
    }
    platform::start_file_drag(paths)
}

#[cfg(not(any(target_os = "windows", target_os = "macos")))]
/// Start dragging the given file paths to an external target.
///
/// Returns an error because this platform is not supported here.
pub fn start_file_drag(_anchor: (), _paths: &[PathBuf]) -> Result<(), String> {
    Err("External drag-out is only supported on Windows and macOS in this build".into())
}

#[cfg(target_os = "windows")]
mod payload;

#[cfg(target_os = "windows")]
mod platform;
#[cfg(target_os = "macos")]
#[path = "platform_macos.rs"]
mod platform;
#[cfg(all(test, target_os = "windows"))]
mod tests;

#[cfg(any(target_os = "windows", target_os = "macos"))]
/// Normalize one drag path to an absolute filesystem path.
fn normalize_path(path: &std::path::Path) -> PathBuf {
    let absolute = path.canonicalize().unwrap_or_else(|_| path.to_path_buf());
    let verbatim_prefix = "\\\\?\\";
    if absolute
        .as_os_str()
        .to_string_lossy()
        .starts_with(verbatim_prefix)
    {
        PathBuf::from(
            absolute
                .as_os_str()
                .to_string_lossy()
                .trim_start_matches(verbatim_prefix),
        )
    } else {
        absolute
    }
}
