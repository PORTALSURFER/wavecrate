//! Native-app lifecycle, startup, dispatch, and shortcut modules.

#[allow(unused_imports)]
use super::{app_scope, audio_settings, folder_browser, source_watcher, waveform};

mod launch;
pub(in crate::native_app) mod lifecycle;
pub(in crate::native_app) mod message_dispatch;
pub(in crate::native_app) mod shortcuts;

pub(crate) use launch::run;
pub(in crate::native_app) use launch::{
    DEBUG_LAYOUT_ARG, DEBUG_LAYOUT_SHORT_ARG, DEFAULT_WINDOW_TITLE, debug_layout_requested,
    emit_gui_action,
};
