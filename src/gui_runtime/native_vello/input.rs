//! Keyboard/pointer/wheel action mapping for the native runtime.

use super::*;
use crate::gui::{focus::FocusSurface, range::NormalizedRange, shortcuts::ShortcutResolution};

mod key;
mod pointer;
mod waveform_drag;
mod waveform_geometry;
mod waveform_handles;
mod waveform_routing;
mod wheel;

pub(super) use self::waveform_drag::{
    WAVEFORM_ANCHOR_RATIO_MICROS_SCALE, WAVEFORM_EDIT_FADE_HANDLE_HIT_HALF_WIDTH,
    WAVEFORM_EDIT_FADE_TOP_TAB_SIZE, WAVEFORM_RESIZE_EDGE_HEIGHT_RATIO,
    WAVEFORM_RESIZE_EDGE_HIT_HALF_WIDTH, WAVEFORM_SELECTION_CLICK_SLOP_PX,
    WAVEFORM_SELECTION_DRAG_HANDLE_HIT_INSET, WAVEFORM_SELECTION_DRAG_HANDLE_SIZE,
    WAVEFORM_SELECTION_SHIFT_HANDLE_HEIGHT, WAVEFORM_SELECTION_SHIFT_HANDLE_HIT_INSET,
    WAVEFORM_SELECTION_SHIFT_HANDLE_WIDTH, WAVEFORM_WHEEL_ZOOM_PIXEL_STEP, WaveformOutsidePlotSide,
    WaveformPointerDragMode, WaveformSelectionBoundaryLock,
};
pub(crate) use waveform_routing::duplicate_cleanup_exemption_action_from_pointer;

pub(super) fn action_from_key(
    key: KeyCode,
    modifiers: ModifiersState,
    model: &AppModel,
    pending_chord: Option<KeyPress>,
    resolve_hotkey: impl FnMut(Option<KeyPress>, KeyPress, FocusSurface) -> ShortcutResolution<UiAction>,
) -> ShortcutResolution<UiAction> {
    key::action_from_key(key, modifiers, model, pending_chord, resolve_hotkey)
}

#[cfg(test)]
pub(super) fn action_from_pointer(
    layout: &ShellLayout,
    model: &AppModel,
    shell_state: &mut NativeShellState,
    point: Point,
    modifiers: ModifiersState,
) -> Option<UiAction> {
    pointer::action_from_pointer_with_motion(layout, model, None, shell_state, point, modifiers)
}

/// Resolve one pointer click action using optional retained motion-model context.
pub(super) fn action_from_pointer_with_motion(
    layout: &ShellLayout,
    model: &AppModel,
    motion_model: Option<&NativeMotionModel>,
    shell_state: &mut NativeShellState,
    point: Point,
    modifiers: ModifiersState,
) -> Option<UiAction> {
    pointer::action_from_pointer_with_motion(
        layout,
        model,
        motion_model,
        shell_state,
        point,
        modifiers,
    )
}

pub(super) fn waveform_action_from_pointer(
    layout: &ShellLayout,
    model: &AppModel,
    point: Point,
    modifiers: ModifiersState,
) -> UiAction {
    waveform_routing::waveform_action_from_pointer(layout, model, point, modifiers)
}

pub(super) fn waveform_resize_handle_hovered(
    layout: &ShellLayout,
    model: &AppModel,
    point: Point,
) -> bool {
    waveform_routing::waveform_resize_handle_hovered(layout, model, point)
}

pub(super) fn waveform_selection_drag_handle_hovered(
    layout: &ShellLayout,
    model: &AppModel,
    point: Point,
) -> bool {
    waveform_routing::waveform_selection_drag_handle_hovered(layout, model, point)
}

pub(super) fn waveform_edit_action_from_pointer(
    layout: &ShellLayout,
    model: &AppModel,
    point: Point,
    modifiers: ModifiersState,
) -> UiAction {
    waveform_routing::waveform_edit_action_from_pointer(layout, model, point, modifiers)
}

#[cfg(test)]
pub(super) fn waveform_drag_action_for_mode(
    layout: &ShellLayout,
    model: &AppModel,
    point: Point,
    mode: WaveformPointerDragMode,
    modifiers: ModifiersState,
) -> UiAction {
    waveform_routing::waveform_drag_action_for_mode(layout, model, point, mode, modifiers)
}

/// Resolve one waveform drag action and the updated drag mode for the pointer.
pub(super) fn waveform_drag_action_and_mode_for_point(
    layout: &ShellLayout,
    model: &AppModel,
    point: Point,
    mode: WaveformPointerDragMode,
    modifiers: ModifiersState,
) -> (UiAction, WaveformPointerDragMode) {
    waveform_routing::waveform_drag_action_and_mode_for_point(layout, model, point, mode, modifiers)
}

pub(super) fn waveform_drag_exceeds_click_slop(
    layout: &ShellLayout,
    model: &AppModel,
    point: Point,
    mode: WaveformPointerDragMode,
) -> bool {
    waveform_routing::waveform_drag_exceeds_click_slop(layout, model, point, mode)
}

pub(super) fn waveform_drag_mode_for_action(action: &UiAction) -> Option<WaveformPointerDragMode> {
    waveform_routing::waveform_drag_mode_for_action(action)
}

pub(super) fn waveform_drag_mode_is_edit_fade(mode: WaveformPointerDragMode) -> bool {
    waveform_routing::waveform_drag_mode_is_edit_fade(mode)
}

pub(super) fn waveform_press_action_emits_immediately(action: &UiAction) -> bool {
    waveform_routing::waveform_press_action_emits_immediately(action)
}

#[cfg(test)]
pub(super) fn waveform_position_milli_from_point(
    layout: &ShellLayout,
    model: &AppModel,
    point: Point,
) -> u16 {
    waveform_geometry::waveform_position_milli_from_point(layout, model, point)
}

pub(super) fn waveform_position_micros_from_point(
    layout: &ShellLayout,
    model: &AppModel,
    point: Point,
) -> u32 {
    waveform_geometry::waveform_position_micros_from_point(layout, model, point)
}

pub(super) fn waveform_position_nanos_from_point(
    layout: &ShellLayout,
    model: &AppModel,
    point: Point,
) -> u32 {
    waveform_geometry::waveform_position_nanos_from_point(layout, model, point)
}

pub(super) fn waveform_pointer_position_from_point(
    layout: &ShellLayout,
    model: &AppModel,
    point: Point,
) -> waveform_geometry::WaveformPointerPosition {
    waveform_geometry::waveform_pointer_position_from_point(layout, model, point)
}

pub(super) fn waveform_ratio_from_point(layout: &ShellLayout, point: Point) -> f32 {
    waveform_geometry::waveform_ratio_from_point(layout, point)
}

pub(super) fn ratio_to_micros(ratio: f32) -> u32 {
    waveform_geometry::ratio_to_micros(ratio)
}

pub(super) fn waveform_anchor_micros(model: &AppModel) -> u32 {
    waveform_geometry::waveform_anchor_micros(model)
}

#[cfg(test)]
pub(super) fn shift_waveform_range_micros(
    pointer_micros: u32,
    position_micros: u32,
    start_micros: u32,
    end_micros: u32,
) -> (u32, u32) {
    waveform_geometry::shift_waveform_range_micros(
        pointer_micros,
        position_micros,
        start_micros,
        end_micros,
    )
}

pub(super) fn shift_waveform_range_nanos(
    pointer_nanos: u32,
    position_nanos: u32,
    start_nanos: u32,
    end_nanos: u32,
) -> (u32, u32) {
    waveform_geometry::shift_waveform_range_nanos(
        pointer_nanos,
        position_nanos,
        start_nanos,
        end_nanos,
    )
}

pub(super) fn nanos_to_micros(value_nanos: u32) -> u32 {
    waveform_geometry::nanos_to_micros(value_nanos)
}

pub(super) fn waveform_point_is_outside_plot_x(layout: &ShellLayout, point: Point) -> bool {
    waveform_geometry::waveform_point_is_outside_plot_x(layout, point)
}

pub(super) fn waveform_x_for_micros(plot: UiRect, model: &AppModel, micros: u32) -> f32 {
    waveform_geometry::waveform_x_for_micros(plot, model, micros)
}

pub(super) fn waveform_centered_resize_edge_y_bounds(plot: UiRect) -> (f32, f32) {
    waveform_geometry::waveform_centered_resize_edge_y_bounds(plot)
}

pub(super) fn waveform_edit_fade_curve_milli_from_point(layout: &ShellLayout, point: Point) -> u16 {
    waveform_geometry::waveform_edit_fade_curve_milli_from_point(layout, point)
}

pub(super) fn waveform_edit_fade_handle_action_from_pointer(
    layout: &ShellLayout,
    model: &AppModel,
    point: Point,
) -> Option<UiAction> {
    waveform_handles::waveform_edit_fade_handle_action_from_pointer(layout, model, point)
}

pub(super) fn waveform_edit_fade_curve_action_from_pointer(
    layout: &ShellLayout,
    model: &AppModel,
    point: Point,
) -> Option<UiAction> {
    waveform_handles::waveform_edit_fade_curve_action_from_pointer(layout, model, point)
}

pub(super) fn waveform_edit_resize_action_from_pointer(
    layout: &ShellLayout,
    model: &AppModel,
    point: Point,
) -> Option<UiAction> {
    waveform_handles::waveform_edit_resize_action_from_pointer(layout, model, point)
}

pub(super) fn waveform_edit_selection_contains_point(
    layout: &ShellLayout,
    model: &AppModel,
    point: Point,
) -> bool {
    waveform_handles::waveform_edit_selection_contains_point(layout, model, point)
}

pub(super) fn waveform_selection_contains_point(
    layout: &ShellLayout,
    model: &AppModel,
    point: Point,
) -> bool {
    waveform_handles::waveform_selection_contains_point(layout, model, point)
}

pub(super) fn waveform_selection_drag_handle_hit_rect(
    layout: &ShellLayout,
    model: &AppModel,
) -> Option<UiRect> {
    waveform_handles::waveform_selection_drag_handle_hit_rect(layout, model)
}

pub(super) fn waveform_selection_shift_handle_hit_rect(
    layout: &ShellLayout,
    model: &AppModel,
    selection: NormalizedRange,
) -> Option<UiRect> {
    waveform_handles::waveform_selection_shift_handle_hit_rect(layout, model, selection)
}

pub(super) fn browser_list_wheel_row_delta(
    layout: &ShellLayout,
    model: &AppModel,
    point: Point,
    style: &StyleTokens,
    delta: MouseScrollDelta,
) -> Option<i8> {
    wheel::browser_list_wheel_row_delta(layout, model, point, style, delta)
}

pub(super) fn folder_wheel_row_delta(
    shell_state: &mut NativeShellState,
    layout: &ShellLayout,
    model: &AppModel,
    point: Point,
    style: &StyleTokens,
    delta: MouseScrollDelta,
) -> Option<i8> {
    wheel::folder_wheel_row_delta(shell_state, layout, model, point, style, delta)
}

pub(super) fn browser_list_view_start_after_wheel(
    current_view_start: usize,
    visible_count: usize,
    viewport_len: usize,
    steps: i8,
) -> Option<usize> {
    wheel::browser_list_view_start_after_wheel(
        current_view_start,
        visible_count,
        viewport_len,
        steps,
    )
}

pub(super) fn waveform_wheel_zoom_action(
    layout: &ShellLayout,
    model: &AppModel,
    point: Point,
    delta: MouseScrollDelta,
) -> Option<UiAction> {
    wheel::waveform_wheel_zoom_action(layout, model, point, delta)
}
