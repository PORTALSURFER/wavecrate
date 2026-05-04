use super::*;

pub(in crate::gui_runtime::native_vello) const FOCUS_PULSE_HZ: u64 = 60;
pub(in crate::gui_runtime::native_vello) const IDLE_STATUS_REFRESH_HZ: u64 = 4;
/// Short-lived redraw cadence used immediately after cursor movement.
pub(in crate::gui_runtime::native_vello) const CURSOR_ACTIVITY_REDRAW_HZ: u64 = 120;
/// Duration to keep the high-frequency cursor redraw cadence active.
pub(in crate::gui_runtime::native_vello) const CURSOR_ACTIVITY_REDRAW_WINDOW: Duration =
    Duration::from_millis(100);
/// Maximum retained image-upload blobs before cache reset.
pub(in crate::gui_runtime::native_vello) const IMAGE_UPLOAD_BLOB_CACHE_LIMIT: usize = 32;
pub(in crate::gui_runtime::native_vello) const INCREMENTAL_FRAME_PIPELINE_ENV: &str =
    "RADIANT_NATIVE_INCREMENTAL_FRAME_PIPELINE";
/// Maximum time to wait for a deferred startup refresh before revealing anyway.
pub(in crate::gui_runtime::native_vello) const STARTUP_REVEAL_STALL_TIMEOUT: Duration =
    Duration::from_millis(300);

/// Convert one logical pointer point into lossless-enough action coordinates.
pub(in crate::gui_runtime::native_vello) fn ui_action_pointer_coords(point: Point) -> (u16, u16) {
    crate::gui::input::logical_point_to_u16_coords(point)
}
