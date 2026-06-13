use crate::native_app::test_support::shell::{
    DEBUG_LAYOUT_ARG, DEBUG_LAYOUT_SHORT_ARG, DEBUG_OVERLAYS_ARG, debug_layout_requested,
};
use std::ffi::OsString;

#[test]
fn canonical_debug_layout_arg_enables_default_gui_overlay() {
    assert!(debug_layout_requested([
        OsString::from("wavecrate"),
        OsString::from(DEBUG_LAYOUT_ARG),
    ]));
}

#[test]
fn short_debug_layout_arg_enables_default_gui_overlay() {
    assert!(debug_layout_requested([
        OsString::from("wavecrate"),
        OsString::from(DEBUG_LAYOUT_SHORT_ARG),
    ]));
}

#[test]
fn debug_overlays_arg_enables_default_gui_overlay() {
    assert!(debug_layout_requested([
        OsString::from("wavecrate"),
        OsString::from(DEBUG_OVERLAYS_ARG),
    ]));
}

#[test]
fn unrelated_args_leave_default_gui_overlay_disabled() {
    assert!(!debug_layout_requested([
        OsString::from("wavecrate"),
        OsString::from("--debug-log"),
    ]));
}
