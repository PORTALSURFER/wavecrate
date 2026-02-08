//! Minimal view-model helpers when legacy runtime is disabled.

use std::path::Path;

/// Build a human-readable label for a sample path.
pub fn sample_display_label(path: &Path) -> String {
    path.file_name()
        .and_then(|name| name.to_str())
        .map(ToOwned::to_owned)
        .unwrap_or_else(|| path.display().to_string())
}
