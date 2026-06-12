//! Backend-neutral view-model helpers for migration consumers.

use std::path::Path;

/// Produce a user-facing sample label that omits folders and extensions.
pub(crate) fn sample_display_label(path: &Path) -> String {
    path.file_stem()
        .and_then(|stem| stem.to_str())
        .map(|stem| stem.to_string())
        .or_else(|| {
            path.file_name()
                .and_then(|name| name.to_str())
                .map(|name| name.to_string())
        })
        .unwrap_or_else(|| path.to_string_lossy().to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;

    #[test]
    fn sample_display_label_strips_directories_and_extension() {
        let label = sample_display_label(Path::new("kits/sub/hihat_open.WAV"));
        assert_eq!(label, "hihat_open");
    }

    #[test]
    fn sample_display_label_handles_files_without_extension() {
        let label = sample_display_label(Path::new("clips/snare_roll"));
        assert_eq!(label, "snare_roll");
    }
}
