//! Hit-testing, hover resolution, and pointer-geometry helpers for native shell state.

use super::*;

mod browser;
mod chrome;
mod hover;
mod map;
mod waveform;

pub(in crate::app_core::native_shell::composition::state) use self::browser::{
    browser_action_hit_test_cache_key, browser_action_model_signature,
};
pub(in crate::app_core::native_shell::composition::state) use self::chrome::sidebar_filter_dropdown_spec;
pub(in crate::app_core::native_shell::composition::state) use self::map::{
    map_content_id_at_point, map_point_color, map_point_is_focused, map_point_is_selected,
};
pub(in crate::app_core::native_shell::composition::state) use self::waveform::{
    hovered_waveform_resize_edge_for_point, waveform_hover_marker_rect, waveform_hover_x_for_point,
    waveform_toolbar_hit_test_cache_key, waveform_toolbar_hover_hint,
};
