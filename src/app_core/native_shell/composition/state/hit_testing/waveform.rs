use self::sempal_crate::app as native_model;
use super::*;
use crate as sempal_crate;

#[path = "waveform/toolbar.rs"]
mod toolbar;

pub(in crate::gui::native_shell::state) use self::toolbar::{
    waveform_toolbar_hit_test_cache_key, waveform_toolbar_hover_hint,
};

#[cfg(test)]
pub(in crate::gui::native_shell::state) fn waveform_toolbar_model_flags(
    model: &NativeMotionModel,
) -> u16 {
    toolbar::waveform_toolbar_model_flags(model)
}

impl NativeShellState {
    /// Return a waveform-toolbar button rect for one control label in tests.
    #[cfg(test)]
    pub(crate) fn waveform_toolbar_button_rect(
        &self,
        layout: &ShellLayout,
        model: &AppModel,
        label: &'static str,
    ) -> Option<Rect> {
        let style = style_for_layout(layout);
        let motion_model = NativeMotionModel::from_app_model(model);
        waveform_toolbar_buttons(
            layout,
            &style,
            &motion_model,
            self.waveform_bpm_input_active,
            self.waveform_bpm_input_display.as_deref(),
        )
        .into_iter()
        .find(|button| button.label == label)
        .map(|button| button.rect)
    }

    /// Return the hovered resize edge resolved from one waveform point in tests.
    #[cfg(test)]
    pub(crate) fn hovered_waveform_resize_edge_at_point(
        &self,
        layout: &ShellLayout,
        model: &AppModel,
        point: Point,
    ) -> Option<WaveformResizeHoverEdge> {
        hovered_waveform_resize_edge_for_point(
            layout,
            model,
            point,
            Some(ShellNodeKind::WaveformCard),
        )
    }

    /// Return the pointer's offset within the waveform scrollbar thumb when hovered.
    pub(crate) fn waveform_scrollbar_thumb_offset_at_point(
        &self,
        layout: &ShellLayout,
        model: &AppModel,
        point: Point,
    ) -> Option<f32> {
        let viewport = model.waveform.viewport();
        let scrollbar = waveform_scrollbar_layout(
            layout.waveform_scrollbar_lane,
            viewport.start_micros,
            viewport.end_micros,
        )?;
        scrollbar
            .thumb
            .contains(point)
            .then_some((point.x - scrollbar.thumb.min.x).clamp(0.0, scrollbar.thumb.width()))
    }

    /// Return the normalized pointer grip ratio within the waveform scrollbar thumb.
    pub(crate) fn waveform_scrollbar_thumb_ratio_at_point(
        &self,
        layout: &ShellLayout,
        model: &AppModel,
        point: Point,
    ) -> Option<f32> {
        let viewport = model.waveform.viewport();
        let scrollbar = waveform_scrollbar_layout(
            layout.waveform_scrollbar_lane,
            viewport.start_micros,
            viewport.end_micros,
        )?;
        scrollbar.thumb.contains(point).then_some({
            let thumb_width = scrollbar.thumb.width().max(1.0);
            ((point.x - scrollbar.thumb.min.x) / thumb_width).clamp(0.0, 1.0)
        })
    }

    /// Return the current waveform scrollbar thumb width for one view model.
    pub(crate) fn waveform_scrollbar_thumb_width(
        &self,
        layout: &ShellLayout,
        model: &AppModel,
    ) -> Option<f32> {
        let viewport = model.waveform.viewport();
        waveform_scrollbar_layout(
            layout.waveform_scrollbar_lane,
            viewport.start_micros,
            viewport.end_micros,
        )
        .map(|scrollbar| scrollbar.thumb.width().max(1.0))
    }

    /// Resolve the waveform viewport center for an active scrollbar-thumb drag.
    pub(crate) fn waveform_scrollbar_view_center_for_drag(
        &self,
        layout: &ShellLayout,
        model: &AppModel,
        pointer_x: f32,
        thumb_pointer_offset_x: f32,
    ) -> Option<u32> {
        let viewport = model.waveform.viewport();
        let scrollbar = waveform_scrollbar_layout(
            layout.waveform_scrollbar_lane,
            viewport.start_micros,
            viewport.end_micros,
        )?;
        waveform_scrollbar_center_for_pointer(
            scrollbar,
            viewport.start_micros,
            viewport.end_micros,
            pointer_x,
            thumb_pointer_offset_x,
        )
    }

    /// Resolve the waveform viewport center for a click inside the scrollbar track.
    pub(crate) fn waveform_scrollbar_view_center_at_point(
        &self,
        layout: &ShellLayout,
        model: &AppModel,
        point: Point,
    ) -> Option<u32> {
        let viewport = model.waveform.viewport();
        let scrollbar = waveform_scrollbar_layout(
            layout.waveform_scrollbar_lane,
            viewport.start_micros,
            viewport.end_micros,
        )?;
        if !scrollbar.track.contains(point) || scrollbar.thumb.contains(point) {
            return None;
        }
        waveform_scrollbar_center_for_pointer(
            scrollbar,
            viewport.start_micros,
            viewport.end_micros,
            point.x,
            scrollbar.thumb.width() * 0.5,
        )
    }

    /// Resolve a waveform-toolbar control click into a native UI action.
    #[cfg(test)]
    pub(crate) fn waveform_toolbar_action_at_point(
        &mut self,
        layout: &ShellLayout,
        model: &AppModel,
        point: Point,
    ) -> Option<UiAction> {
        self.waveform_toolbar_action_at_point_with_modifiers(layout, model, point, false)
    }

    /// Resolve a waveform-toolbar control click into a native UI action.
    pub(crate) fn waveform_toolbar_action_at_point_with_modifiers(
        &mut self,
        layout: &ShellLayout,
        model: &AppModel,
        point: Point,
        shift_down: bool,
    ) -> Option<UiAction> {
        let motion_model = NativeMotionModel::from_app_model(model);
        self.waveform_toolbar_action_at_point_with_motion_and_modifiers(
            layout,
            &motion_model,
            point,
            shift_down,
        )
    }

    /// Resolve a waveform-toolbar control click into a native UI action.
    pub(crate) fn waveform_toolbar_action_at_point_with_motion_and_modifiers(
        &mut self,
        layout: &ShellLayout,
        motion_model: &NativeMotionModel,
        point: Point,
        shift_down: bool,
    ) -> Option<UiAction> {
        let style = style_for_layout(layout);
        let resolved = self
            .cached_waveform_toolbar_buttons(layout, &style, motion_model)
            .into_iter()
            .find(|button| button.enabled && button.rect.contains(point))
            .map(|button| {
                (
                    waveform_toolbar_hover_hint(button.label),
                    if shift_down && button.label == "Loop" {
                        Some(UiAction::ToggleLoopLock)
                    } else {
                        button.action.clone()
                    },
                )
            });
        if let Some((Some(hint), _)) = resolved.as_ref() {
            self.trigger_waveform_toolbar_flash(*hint);
        }
        resolved.and_then(|(_, action)| action)
    }

    /// Return whether a point falls inside the waveform BPM text-input widget.
    pub(crate) fn waveform_bpm_input_at_point(
        &mut self,
        layout: &ShellLayout,
        model: &AppModel,
        point: Point,
    ) -> bool {
        let motion_model = NativeMotionModel::from_app_model(model);
        self.waveform_bpm_input_at_point_with_motion(layout, &motion_model, point)
    }

    /// Return whether a point falls inside the waveform BPM text-input widget.
    pub(crate) fn waveform_bpm_input_at_point_with_motion(
        &mut self,
        layout: &ShellLayout,
        motion_model: &NativeMotionModel,
        point: Point,
    ) -> bool {
        let style = style_for_layout(layout);
        let hit = self
            .cached_waveform_toolbar_buttons(layout, &style, motion_model)
            .iter()
            .any(|button| {
                button.label == "BPM Value" && button.enabled && button.rect.contains(point)
            });
        if hit {
            self.trigger_waveform_toolbar_flash(WaveformToolbarHoverHint::BpmValue);
        }
        hit
    }

    /// Return the waveform BPM input rect when the toolbar is available.
    pub(crate) fn waveform_bpm_input_rect(
        &mut self,
        layout: &ShellLayout,
        model: &AppModel,
    ) -> Option<Rect> {
        let motion_model = NativeMotionModel::from_app_model(model);
        let style = style_for_layout(layout);
        self.cached_waveform_toolbar_buttons(layout, &style, &motion_model)
            .iter()
            .find(|button| button.label == "BPM Value" && button.enabled)
            .map(|button| button.rect)
    }

    /// Return the waveform BPM text rect used for rendering inside the field.
    pub(crate) fn waveform_bpm_text_rect(
        &mut self,
        layout: &ShellLayout,
        model: &AppModel,
    ) -> Option<Rect> {
        let motion_model = NativeMotionModel::from_app_model(model);
        let style = style_for_layout(layout);
        self.cached_waveform_toolbar_buttons(layout, &style, &motion_model)
            .iter()
            .find(|button| button.label == "BPM Value" && button.enabled)
            .map(|button| compute_action_button_text_rect(button.rect, style.sizing))
    }
}

/// Return hovered waveform marker x-position for one pointer point.
pub(in crate::gui::native_shell::state) fn waveform_hover_x_for_point(
    layout: &ShellLayout,
    hover: Option<ShellNodeKind>,
    point: Point,
) -> Option<f32> {
    if hover != Some(ShellNodeKind::WaveformCard) || !layout.waveform_plot.contains(point) {
        return None;
    }
    Some(
        point
            .x
            .clamp(layout.waveform_plot.min.x, layout.waveform_plot.max.x)
            .round(),
    )
}

/// Return the hovered waveform resize-edge target for one pointer point.
pub(in crate::gui::native_shell::state) fn hovered_waveform_resize_edge_for_point(
    layout: &ShellLayout,
    model: &AppModel,
    point: Point,
    hover: Option<ShellNodeKind>,
) -> Option<WaveformResizeHoverEdge> {
    if hover != Some(ShellNodeKind::WaveformCard) || !layout.waveform_plot.contains(point) {
        return None;
    }
    let edit_preview = model.waveform.edit_preview();
    let transport = model.waveform.transport();
    hovered_resize_edge_for_range(layout, model, point, edit_preview.selection)
        .map(|left_edge| {
            if left_edge {
                WaveformResizeHoverEdge::EditSelectionStart
            } else {
                WaveformResizeHoverEdge::EditSelectionEnd
            }
        })
        .or_else(|| {
            hovered_resize_edge_for_range(layout, model, point, transport.selection).map(
                |left_edge| {
                    if left_edge {
                        WaveformResizeHoverEdge::SelectionStart
                    } else {
                        WaveformResizeHoverEdge::SelectionEnd
                    }
                },
            )
        })
}

/// Return whether the pointer is hovering the start (`true`) or end (`false`) edge of one range.
fn hovered_resize_edge_for_range(
    layout: &ShellLayout,
    model: &AppModel,
    point: Point,
    range: Option<native_model::NormalizedRangeModel>,
) -> Option<bool> {
    let range = range?;
    let start_micros = range.start_micros.min(range.end_micros);
    let end_micros = range.start_micros.max(range.end_micros);
    if end_micros <= start_micros {
        return None;
    }
    let (handle_top, handle_bottom) = waveform_centered_resize_edge_y_bounds(layout.waveform_plot);
    if point.y < handle_top || point.y > handle_bottom {
        return None;
    }
    let start_x = waveform_x_for_micros(layout.waveform_plot, model, start_micros);
    let end_x = waveform_x_for_micros(layout.waveform_plot, model, end_micros);
    let threshold = 7.0;
    let start_distance = (point.x - start_x).abs();
    let end_distance = (point.x - end_x).abs();
    if start_distance > threshold && end_distance > threshold {
        return None;
    }
    Some(start_distance <= end_distance)
}

/// Convert one normalized waveform micro position into plot-space x.
pub(in crate::gui::native_shell::state) fn waveform_x_for_micros(
    plot: Rect,
    model: &AppModel,
    micros: u32,
) -> f32 {
    let viewport = model.waveform.viewport();
    let view = waveform_view_window_from_bounds(
        viewport.start_micros,
        viewport.end_micros,
        Some(viewport.start_nanos),
        Some(viewport.end_nanos),
    );
    waveform_plot_x_for_micros(plot, micros, view, NormalizedPixelSnap::Nearest)
}

/// Return the centered vertical hit span used by waveform edge-resize targets.
pub(in crate::gui::native_shell::state) fn waveform_centered_resize_edge_y_bounds(
    plot: Rect,
) -> (f32, f32) {
    let height = (plot.height() * 0.34).max(1.0).min(plot.height());
    let center_y = plot.min.y + (plot.height() * 0.5);
    let top = (center_y - (height * 0.5)).max(plot.min.y);
    let bottom = (top + height).min(plot.max.y).max(top + 1.0);
    (top, bottom)
}

/// Return one plot-bounded hover marker rectangle for a waveform x-position.
pub(in crate::gui::native_shell::state) fn waveform_hover_marker_rect(
    waveform_plot: Rect,
    marker_width: f32,
    hover_x: f32,
) -> Option<Rect> {
    if waveform_plot.width() <= 0.0 || waveform_plot.height() <= 0.0 {
        return None;
    }
    let width = marker_width.max(1.0);
    let half = width * 0.5;
    let clamped_x = hover_x.clamp(waveform_plot.min.x, waveform_plot.max.x);
    let left = (clamped_x - half).clamp(waveform_plot.min.x, waveform_plot.max.x - 1.0);
    let right = (left + width).min(waveform_plot.max.x).max(left + 1.0);
    Some(Rect::from_min_max(
        Point::new(left, waveform_plot.min.y),
        Point::new(right, waveform_plot.max.y),
    ))
}
