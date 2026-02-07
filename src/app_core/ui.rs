//! Backend-neutral UI constant aliases for migration consumers.
//!
//! These constants define default/minimum viewport constraints used by native
//! runtime entrypoints while the legacy `egui_app` module remains in-tree.

/// Default viewport size used by the native runtime host window.
pub const DEFAULT_VIEWPORT_SIZE: [f32; 2] = crate::egui_app::ui::DEFAULT_VIEWPORT_SIZE;
/// Minimum viewport size used by the native runtime host window.
pub const MIN_VIEWPORT_SIZE: [f32; 2] = crate::egui_app::ui::MIN_VIEWPORT_SIZE;
