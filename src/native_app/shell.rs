//! Product-specific native app shell, lifecycle, dispatch, and shortcut modules.

mod launch;
pub(in crate::native_app) mod lifecycle;
mod macos_app_icon;
pub(in crate::native_app) mod message_dispatch;
pub(in crate::native_app) mod shortcuts;

pub(in crate::native_app) use launch::emit_gui_action;
#[cfg(any(test, feature = "legacy-controller"))]
pub(in crate::native_app) use launch::native_app_runtime_bridge;
pub use launch::run;
#[cfg(test)]
pub(in crate::native_app) use launch::{
    DEBUG_LAYOUT_ARG, DEBUG_LAYOUT_SHORT_ARG, DEBUG_OVERLAYS_ARG, debug_layout_requested,
    default_window_title,
};
