//! Product-domain helper for destructive WAV edit capability checks.
//!
//! The file-format predicate remains implemented by the retained controller
//! library while app-core owns the projection-facing helper surface.

use std::path::Path;

/// Return whether the file path can be handled by destructive WAV edit actions.
pub(crate) fn supports_wav_destructive_edits(path: &Path) -> bool {
    crate::app::controller::supports_wav_destructive_edits(path)
}
