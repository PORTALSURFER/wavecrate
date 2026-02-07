//! Backend-neutral view-model aliases for migration consumers.
//!
//! These helpers keep runtime-facing projection code independent from direct
//! `egui_app::view_model` module paths while controller internals continue to
//! migrate. Keep this surface minimal and add only functions needed by
//! migration-facing modules.

use std::path::Path;

/// Build a human-readable label for a sample path.
pub fn sample_display_label(path: &Path) -> String {
    crate::egui_app::view_model::sample_display_label(path)
}
