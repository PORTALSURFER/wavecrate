//! Product-specific native app shell, lifecycle, dispatch, and shortcut modules.

mod launch;
pub(in crate::native_app) mod lifecycle;
pub(in crate::native_app) mod message_dispatch;
pub(in crate::native_app) mod shortcuts;

pub(in crate::native_app) use launch::emit_gui_action;
pub(crate) use launch::run;
#[cfg(test)]
pub(in crate::native_app) use launch::{
    DEBUG_LAYOUT_ARG, DEBUG_LAYOUT_SHORT_ARG, DEFAULT_WINDOW_TITLE, debug_layout_requested,
};
