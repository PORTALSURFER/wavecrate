//! Drag-session and immediate action emission helpers for native input.

use super::super::*;

/// Horizontal click slop used to distinguish waveform clicks from drags.
const WAVEFORM_CLICK_SEEK_SLOP_PX: f32 = 3.0;
/// Pointer slop used to distinguish browser-row clicks from drag/drop.
const CONTENT_ROW_DRAG_SLOP_PX: f32 = 3.0;

impl<Bridge> NativeVelloRunner<Bridge>
where
    Bridge: NativeAppBridge,
{
    /// Refresh the cached app model before immediate follow-up input uses stale state.
    ///
    /// Selection creation and focus changes can reduce into the bridge one event
    /// before the next redraw rebuild refreshes `self.model`. Keyboard shortcuts
    /// and same-frame pointer hit-testing should pull that pending snapshot so
    /// actions like immediate selection export see the latest focus/selection.
    pub(crate) fn refresh_cached_model_for_pending_input(&mut self) {
        if !self.frame_state.model_dirty {
            return;
        }
        self.model = self.bridge.pull_model_arc();
        self.waveform_view_refresh_pending = false;
        self.shell_state.sync_from_model(&self.model);
        self.refresh_motion_model_from_model();
        self.sync_text_input_target();
    }

    pub(crate) fn queue_volume_milli(&mut self, value_milli: u16) {
        self.pending_volume_milli = Some(value_milli.min(1000));
    }

    /// Emit one normalized volume update immediately for smooth drag visuals.
    pub(crate) fn emit_volume_milli_immediately(&mut self, value_milli: u16) {
        self.queue_volume_milli(value_milli);
        let _ = self.flush_pending_volume_action();
    }

    pub(crate) fn flush_pending_volume_action(&mut self) -> bool {
        let Some(value_milli) = self.pending_volume_milli.take() else {
            return false;
        };
        self.emit_model_action_with_profile(
            UiAction::SetVolume { value_milli },
            Some(InteractionProfileKind::Volume),
        );
        true
    }

    /// Emit one middle-button waveform pan viewport update immediately.
    pub(crate) fn process_waveform_pan_drag_immediately(&mut self, point: Point) -> bool {
        self.refresh_waveform_view_if_needed();
        let Some(layout) = self.shell_layout.as_ref() else {
            return false;
        };
        let Some(drag) = self.waveform_pan_drag else {
            return false;
        };
        let plot_width = layout.waveform_plot.width().max(1.0);
        let view = waveform_view_window_from_bounds(
            drag.view_start_micros,
            drag.view_end_micros,
            Some(drag.view_start_nanos),
            Some(drag.view_end_nanos),
        );
        let delta_ratio = f64::from((point.x - drag.origin_x) / plot_width);
        let max_start_ratio = (1.0 - view.width_ratio).max(0.0);
        let delta_view_ratio = delta_ratio * view.width_ratio;
        let next_start_ratio = (view.start_ratio - delta_view_ratio).clamp(0.0, max_start_ratio);
        let center_ratio = (next_start_ratio + (view.width_ratio * 0.5)).clamp(0.0, 1.0);
        let center_micros = ratio_to_micros(center_ratio as f32);
        let center_nanos = (center_ratio * 1_000_000_000.0).round() as u32;
        if self.last_emitted_waveform_view_center == Some(center_micros) {
            return true;
        }
        self.last_emitted_waveform_view_center = Some(center_micros);
        self.emit_model_action_with_profile(
            UiAction::SetWaveformViewCenter {
                center_micros,
                center_nanos: Some(center_nanos),
            },
            Some(InteractionProfileKind::Timeline),
        );
        true
    }

    /// Emit one waveform action immediately during active pointer drag.
    pub(crate) fn emit_waveform_drag_action_immediately(&mut self, action: UiAction) {
        if self.last_emitted_waveform_drag_action.as_ref() == Some(&action) {
            return;
        }
        self.last_emitted_waveform_drag_action = Some(action.clone());
        self.emit_model_action_with_profile(action, Some(InteractionProfileKind::Timeline));
    }

    /// Process one waveform drag cursor update when waveform drag mode is active.
    pub(crate) fn process_waveform_drag_immediately(&mut self, point: Point) -> bool {
        self.refresh_waveform_view_if_needed();
        let Some(layout) = self.shell_layout.as_ref() else {
            return false;
        };
        let Some(mode) = self.waveform_drag_mode else {
            return false;
        };
        if self.last_emitted_waveform_drag_action.is_none()
            && !self.waveform_drag_exceeds_click_slop(layout, point, mode)
        {
            return false;
        }
        let (action, next_mode) = waveform_drag_action_and_mode_for_point(
            layout,
            &self.model,
            point,
            mode,
            self.modifiers,
        );
        self.waveform_drag_mode = Some(next_mode);
        self.emit_waveform_drag_action_immediately(action);
        true
    }

    fn waveform_drag_exceeds_click_slop(
        &self,
        layout: &ShellLayout,
        point: Point,
        mode: WaveformPointerDragMode,
    ) -> bool {
        if let (WaveformPointerDragMode::Selection { .. }, Some(click_seek_press)) =
            (mode, self.waveform_click_seek_press)
        {
            return (point.x - click_seek_press.press_x).abs() > WAVEFORM_CLICK_SEEK_SLOP_PX;
        }
        waveform_drag_exceeds_click_slop(layout, &self.model, point, mode)
    }

    /// Refresh the local waveform view once after a wheel zoom changed it mid-drag.
    ///
    /// Wheel zoom reduces into the bridge immediately, but the runner's cached
    /// `AppModel` normally updates on the next scene rebuild. When the user
    /// keeps dragging before that rebuild lands, refresh the local snapshot
    /// first so pointer-to-time conversion uses the latest view bounds.
    pub(crate) fn refresh_waveform_view_if_needed(&mut self) {
        if !self.waveform_view_refresh_pending {
            return;
        }
        self.model = self.bridge.pull_model_arc();
        self.shell_state.sync_from_model(&self.model);
        self.refresh_motion_model_from_model();
        self.waveform_view_refresh_pending = false;
    }

    /// Process one waveform-selection export drag cursor update.
    pub(crate) fn process_selection_drag_immediately(&mut self, point: Point) -> bool {
        let Some(layout) = self.shell_layout.as_ref() else {
            return false;
        };
        let (pointer_x, pointer_y) = ui_action_pointer_coords(point);
        let (hovered_folder_row, over_folder_panel) = self
            .shell_state
            .sync_folder_drag_hover_target(layout, &self.model, point);
        self.emit_model_action_with_profile(
            UiAction::UpdateWaveformSelectionDrag {
                pointer_x,
                pointer_y,
                hovered_folder_pane: hovered_folder_row.map(|(pane, _)| pane),
                hovered_folder_row: hovered_folder_row.map(|(_, row)| row),
                over_folder_panel,
                over_browser_list: !self.model.map.active && layout.browser_rows.contains(point),
                shift_down: self.modifiers.shift_key(),
                alt_down: self.modifiers.alt_key(),
            },
            Some(InteractionProfileKind::Timeline),
        );
        true
    }

    /// Process one browser-item drag cursor update.
    pub(crate) fn process_content_item_drag_immediately(&mut self, point: Point) -> bool {
        let Some(layout) = self.shell_layout.as_ref() else {
            return false;
        };
        let Some(_drag) = self.content_item_drag else {
            return false;
        };
        let (hovered_projected_folder_row, over_folder_panel) = self
            .shell_state
            .sync_folder_drag_hover_target(layout, &self.model, point);
        let hovered_folder_pane = hovered_projected_folder_row.map(|(pane, _)| pane);
        let hovered_folder_row =
            hovered_projected_folder_row.and_then(|(pane, projected_index)| {
                self.model
                    .sources
                    .folder_pane(pane)
                    .tree_rows
                    .get(projected_index)
                    .and_then(|row| row.backing_index)
                    .or(Some(projected_index))
            });
        let (pointer_x, pointer_y) = ui_action_pointer_coords(point);
        self.emit_model_action(UiAction::UpdateContentItemDrag {
            pointer_x,
            pointer_y,
            hovered_folder_pane,
            hovered_folder_row,
            over_folder_panel,
            shift_down: self.modifiers.shift_key(),
            alt_down: self.modifiers.alt_key(),
        });
        true
    }

    /// Process one spatial-focus drag cursor update while spatial drag mode is active.
    pub(crate) fn process_spatial_focus_drag_immediately(&mut self, point: Point) -> bool {
        let Some(layout) = self.shell_layout.as_ref() else {
            return false;
        };
        if !self.model.map.active {
            return false;
        }
        let Some(action) = self
            .shell_state
            .map_content_action_at_point(layout, &self.model, point)
        else {
            return false;
        };
        let UiAction::FocusSpatialContentItem { content_id } = &action else {
            return false;
        };
        if self.last_emitted_spatial_drag_content_id.as_deref() == Some(content_id.as_str()) {
            return false;
        }
        self.last_emitted_spatial_drag_content_id = Some(content_id.clone());
        self.emit_model_action_with_profile(action, Some(InteractionProfileKind::SpatialPanProxy));
        true
    }

    /// Handle one pointer-press action from cached runtime state in tests.
    ///
    /// Production pointer presses should call `handle_pointer_press_action_at_point`
    /// so click-seek arming uses the live borrowed layout instead of the retained
    /// cache.
    #[cfg(test)]
    pub(crate) fn handle_pointer_press_action(
        &mut self,
        action: UiAction,
        spatial_drag_start: bool,
    ) -> bool {
        let click_seek_press =
            self.shell_layout
                .as_ref()
                .zip(self.last_cursor)
                .and_then(|(layout, point)| {
                    self.waveform_click_seek_press_for_action(&action, layout.as_ref(), point)
                });
        self.handle_pointer_press_action_with_click_seek(
            action,
            spatial_drag_start,
            click_seek_press,
        )
    }

    /// Handle one pointer-press action using the actively borrowed layout state.
    ///
    /// Real pointer presses run inside `with_shell_layout`, which temporarily
    /// takes `self.shell_layout` out of the runner. Accepting the live layout
    /// and point here keeps click-to-seek arming aligned with the same hit-test
    /// inputs that resolved the press action in the first place.
    pub(crate) fn handle_pointer_press_action_at_point(
        &mut self,
        action: UiAction,
        spatial_drag_start: bool,
        layout: &ShellLayout,
        point: Point,
    ) -> bool {
        let click_seek_press = self.waveform_click_seek_press_for_action(&action, layout, point);
        self.handle_pointer_press_action_with_click_seek(
            action,
            spatial_drag_start,
            click_seek_press,
        )
    }

    /// Handle one pointer-press action after any click-seek snapshot is prepared.
    fn handle_pointer_press_action_with_click_seek(
        &mut self,
        action: UiAction,
        spatial_drag_start: bool,
        click_seek_press: Option<WaveformClickSeekPress>,
    ) -> bool {
        if let Some(visible_row) = browser_primary_row_action_visible_row(&action) {
            self.pending_browser_row_press = Some(PendingBrowserRowPress {
                action,
                visible_row,
                press_point: self.last_cursor.unwrap_or(Point::new(0.0, 0.0)),
            });
            self.shell_state.clear_browser_row_hover();
            return true;
        }
        self.emit_pointer_press_action_now(action, spatial_drag_start, click_seek_press)
    }

    pub(crate) fn emit_pointer_press_action_now(
        &mut self,
        action: UiAction,
        spatial_drag_start: bool,
        click_seek_press: Option<WaveformClickSeekPress>,
    ) -> bool {
        if matches!(
            action,
            UiAction::FocusBrowserRow { .. }
                | UiAction::CommitFocusedBrowserRow
                | UiAction::ToggleBrowserRowSelection { .. }
                | UiAction::ExtendBrowserSelectionToRow { .. }
                | UiAction::AddRangeBrowserSelection { .. }
        ) {
            self.shell_state.clear_browser_row_hover();
        }
        let spatial_drag_content_id = match &action {
            UiAction::FocusSpatialContentItem { content_id } => Some(content_id.clone()),
            _ => None,
        };
        self.begin_waveform_pointer_interaction(&action, click_seek_press);
        if !waveform_press_action_emits_immediately(&action) {
            return false;
        }
        self.sync_browser_viewport_for_pointer_row_action(&action);
        self.update_text_target_after_action(&action);
        self.emit_model_action(action);
        if spatial_drag_start {
            self.begin_spatial_focus_drag(spatial_drag_content_id);
        }
        true
    }

    fn waveform_click_seek_press_for_action(
        &self,
        action: &UiAction,
        layout: &ShellLayout,
        point: Point,
    ) -> Option<WaveformClickSeekPress> {
        let clear_selection_on_release = match action {
            UiAction::BeginWaveformSelectionAt { .. }
            | UiAction::BeginWaveformSelectionAtPrecise { .. } => true,
            UiAction::ClearWaveformSelection
            | UiAction::ClearWaveformEditSelection
            | UiAction::ClearWaveformSelections => false,
            _ => return None,
        };
        Some(WaveformClickSeekPress {
            press_x: point.x,
            position_micros: waveform_position_micros_from_point(layout, &self.model, point),
            position_nanos: waveform_position_nanos_from_point(layout, &self.model, point),
            clear_selection_on_release,
        })
    }

    pub(crate) fn maybe_start_content_item_drag(&mut self, point: Point) -> bool {
        let Some(pending_press) = self.pending_browser_row_press.clone() else {
            return false;
        };
        if !content_item_drag_exceeds_click_slop(pending_press.press_point, point) {
            return false;
        }
        let (pointer_x, pointer_y) = ui_action_pointer_coords(point);
        self.pending_browser_row_press = None;
        self.begin_content_item_drag(pending_press.visible_row);
        self.emit_model_action(UiAction::StartContentItemDrag {
            visible_row: pending_press.visible_row,
            pointer_x,
            pointer_y,
        });
        self.process_content_item_drag_immediately(point)
    }
}

fn content_item_drag_exceeds_click_slop(press_point: Point, point: Point) -> bool {
    (point.x - press_point.x).abs() > CONTENT_ROW_DRAG_SLOP_PX
        || (point.y - press_point.y).abs() > CONTENT_ROW_DRAG_SLOP_PX
}

fn browser_primary_row_action_visible_row(action: &UiAction) -> Option<usize> {
    match action {
        UiAction::FocusBrowserRow { visible_row }
        | UiAction::ToggleBrowserRowSelection { visible_row }
        | UiAction::ExtendBrowserSelectionToRow { visible_row }
        | UiAction::AddRangeBrowserSelection { visible_row } => Some(*visible_row),
        _ => None,
    }
}
