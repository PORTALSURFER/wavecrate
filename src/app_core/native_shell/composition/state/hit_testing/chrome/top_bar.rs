use super::*;

impl NativeShellState {
    /// Resolve a click inside the top-bar volume meter to a volume action.
    pub(crate) fn top_bar_volume_action_at_point(
        &self,
        layout: &ShellLayout,
        model: &AppModel,
        point: Point,
    ) -> Option<UiAction> {
        let surface = resolve_top_bar_surface_layout(
            layout.top_bar,
            style_for_layout(layout).sizing,
            &top_bar_surface_content(model),
        );
        if !surface.volume_meter_rect.contains(point) {
            return None;
        }
        Some(volume_action_for_meter(surface.volume_meter_rect, point))
    }

    /// Resolve a drag point against the top-bar volume meter.
    ///
    /// The x-position is clamped to the meter width so dragging beyond the
    /// edges still emits a stable `SetVolume` action.
    pub(crate) fn top_bar_volume_drag_action(
        &self,
        layout: &ShellLayout,
        model: &AppModel,
        point: Point,
    ) -> Option<UiAction> {
        let surface = resolve_top_bar_surface_layout(
            layout.top_bar,
            style_for_layout(layout).sizing,
            &top_bar_surface_content(model),
        );
        if surface.volume_meter_rect.width() <= 0.0 || surface.volume_meter_rect.height() <= 0.0 {
            return None;
        }
        Some(volume_action_for_meter(surface.volume_meter_rect, point))
    }
}
