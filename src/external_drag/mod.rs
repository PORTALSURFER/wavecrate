//! Platform helpers for starting external drag-and-drop operations.
//!
//! Currently implemented for Windows by emitting a `CF_HDROP` drag with one or
//! more absolute file paths. Other platforms return an unsupported error to
//! keep behaviour predictable.

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

#[cfg(not(target_os = "windows"))]
/// Start dragging the given file paths to an external target.
///
/// Returns an error because non-Windows platforms are not supported here.
pub fn start_file_drag(_hwnd: (), _paths: &[PathBuf]) -> Result<(), String> {
    Err("External drag-out is only supported on Windows in this build".into())
}

#[cfg(target_os = "windows")]
mod payload;
#[cfg(target_os = "windows")]
mod platform;
#[cfg(all(test, target_os = "windows"))]
mod tests;

#[cfg(target_os = "windows")]
/// Normalize one drag path to an absolute non-verbatim Windows filesystem path.
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
