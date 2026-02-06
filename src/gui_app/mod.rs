//! Native GUI bridge exports for the `radiant` runtime path.

mod bridge;

/// Default viewport size for the main application window.
pub use crate::egui_app::ui::DEFAULT_VIEWPORT_SIZE;
/// Minimum viewport size for the main application window.
pub use crate::egui_app::ui::MIN_VIEWPORT_SIZE;
/// Construct a native runtime bridge for the `radiant` backend path.
pub use bridge::new_native_bridge;
/// Native runtime bridge for the `radiant` backend path.
pub use bridge::SempalNativeBridge;
