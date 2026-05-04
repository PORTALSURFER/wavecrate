//! Viewport synchronization helpers for immediate native input paths.

use super::super::*;
use crate::gui::panel::SplitPaneSlot;

impl<Bridge> NativeVelloRunner<Bridge>
where
    Bridge: NativeAppBridge,
{
    pub(crate) fn process_folder_view_start_immediately(
        &mut self,
        pane: SplitPaneSlot,
        view_start_row: usize,
    ) -> bool {
        if !self
            .shell_state
            .set_folder_view_start_row(pane, view_start_row)
        {
            return false;
        }
        self.apply_invalidation_scope(RuntimeInvalidationScope::StaticAndOverlays);
        true
    }

    /// Rebase the captured scrollbar grip onto the refreshed thumb geometry.
    fn refresh_waveform_scrollbar_drag_if_needed(&mut self) {
        if !self.waveform_view_refresh_pending {
            return;
        }
        let thumb_pointer_ratio_x = self
            .waveform_scrollbar_drag
            .map(|drag| drag.thumb_pointer_ratio_x);
        self.refresh_waveform_view_if_needed();
        let (Some(layout), Some(thumb_pointer_ratio_x), Some(drag)) = (
            self.shell_layout.as_ref(),
            thumb_pointer_ratio_x,
            self.waveform_scrollbar_drag,
        ) else {
            return;
        };
        let remapped_offset_x = self
            .shell_state
            .waveform_scrollbar_thumb_width(layout, &self.model)
            .map(|thumb_width| (thumb_width * thumb_pointer_ratio_x).clamp(0.0, thumb_width))
            .unwrap_or(drag.thumb_pointer_offset_x);
        self.waveform_scrollbar_drag = Some(WaveformScrollbarDragState {
            thumb_pointer_offset_x: remapped_offset_x,
            thumb_pointer_ratio_x,
        });
    }

    pub(crate) fn sync_browser_viewport_from_shell(&mut self, layout: &ShellLayout) {
        let Some(visible_row) = self
            .shell_state
            .browser_viewport_start_row(layout, &self.model)
        else {
            return;
        };
        if visible_row == self.model.browser.view_start_row
            || self.last_emitted_browser_list_view_start == Some(visible_row)
        {
            return;
        }
        self.last_emitted_browser_list_view_start = Some(visible_row);
        self.emit_model_action(UiAction::SetBrowserViewStart { visible_row });
    }

    pub(crate) fn sync_browser_viewport_for_pointer_row_action(&mut self, action: &UiAction) {
        let Some(target_visible_row) = browser_list_pointer_action_visible_row(action) else {
            return;
        };
        let Some(layout) = self.shell_layout.as_ref() else {
            return;
        };
        let viewport_len = self.shell_state.browser_viewport_len(layout, &self.model);
        let current_view_start = self
            .shell_state
            .browser_viewport_start_row(layout, &self.model)
            .unwrap_or(self.model.browser.view_start_row);
        let Some(next_view_start) = browser_list_view_start_after_focus(
            current_view_start,
            self.model.browser.visible_count,
            viewport_len,
            target_visible_row,
        ) else {
            return;
        };
        if next_view_start == self.model.browser.view_start_row {
            return;
        }
        self.last_emitted_browser_list_view_start = Some(next_view_start);
        self.emit_model_action(UiAction::SetBrowserViewStart {
            visible_row: next_view_start,
        });
    }

    /// Emit one wheel-derived browser-list viewport-scroll action immediately.
    pub(crate) fn process_wheel_rows_immediately(&mut self, visible_row: usize) -> bool {
        self.shell_state.clear_browser_row_hover();
        self.emit_model_action_with_profile(
            UiAction::SetBrowserViewStart { visible_row },
            Some(InteractionProfileKind::Wheel),
        );
        true
    }

    /// Emit one browser-list scrollbar drag viewport update immediately.
    pub(crate) fn process_browser_scrollbar_drag_immediately(&mut self, point: Point) -> bool {
        let Some(layout) = self.shell_layout.as_ref() else {
            return false;
        };
        let Some(drag) = self.browser_scrollbar_drag else {
            return false;
        };
        let Some(visible_row) = self.shell_state.browser_scrollbar_view_start_for_drag(
            layout,
            &self.model,
            point.y,
            drag.thumb_pointer_offset_y,
        ) else {
            return false;
        };
        if self.last_emitted_browser_list_view_start == Some(visible_row) {
            return true;
        }
        self.last_emitted_browser_list_view_start = Some(visible_row);
        self.shell_state.clear_browser_row_hover();
        self.emit_model_action(UiAction::SetBrowserViewStart { visible_row });
        true
    }

    pub(crate) fn process_folder_scrollbar_drag_immediately(&mut self, point: Point) -> bool {
        let Some(layout) = self.shell_layout.as_ref() else {
            return false;
        };
        let Some(drag) = self.folder_scrollbar_drag else {
            return false;
        };
        let Some(view_start_row) = self.shell_state.folder_scrollbar_view_start_for_drag(
            layout,
            &self.model,
            drag.pane,
            point.y,
            drag.thumb_pointer_offset_y,
        ) else {
            return false;
        };
        self.process_folder_view_start_immediately(drag.pane, view_start_row)
    }

    /// Emit one browser-list scrollbar track-click viewport update immediately.
    pub(crate) fn process_browser_scrollbar_track_click_immediately(
        &mut self,
        point: Point,
    ) -> bool {
        let Some(layout) = self.shell_layout.as_ref() else {
            return false;
        };
        let Some(visible_row) =
            self.shell_state
                .browser_scrollbar_view_start_at_point(layout, &self.model, point)
        else {
            return false;
        };
        self.shell_state.clear_browser_row_hover();
        self.emit_model_action(UiAction::SetBrowserViewStart { visible_row });
        true
    }

    pub(crate) fn process_folder_scrollbar_track_click_immediately(
        &mut self,
        point: Point,
    ) -> bool {
        let Some(layout) = self.shell_layout.as_ref() else {
            return false;
        };
        let Some((pane, view_start_row)) =
            self.shell_state
                .folder_scrollbar_view_start_at_point(layout, &self.model, point)
        else {
            return false;
        };
        self.process_folder_view_start_immediately(pane, view_start_row)
    }

    /// Emit one waveform-scrollbar drag viewport update immediately.
    pub(crate) fn process_waveform_scrollbar_drag_immediately(&mut self, point: Point) -> bool {
        self.refresh_waveform_scrollbar_drag_if_needed();
        let Some(layout) = self.shell_layout.as_ref() else {
            return false;
        };
        let Some(drag) = self.waveform_scrollbar_drag else {
            return false;
        };
        let Some(center_micros) = self.shell_state.waveform_scrollbar_view_center_for_drag(
            layout,
            &self.model,
            point.x,
            drag.thumb_pointer_offset_x,
        ) else {
            return false;
        };
        if self.last_emitted_waveform_view_center == Some(center_micros) {
            return true;
        }
        self.last_emitted_waveform_view_center = Some(center_micros);
        self.emit_model_action_with_profile(
            UiAction::SetWaveformViewCenter {
                center_micros,
                center_nanos: None,
            },
            Some(InteractionProfileKind::Timeline),
        );
        true
    }

    /// Emit one waveform-scrollbar track-click viewport update immediately.
    pub(crate) fn process_waveform_scrollbar_track_click_immediately(
        &mut self,
        point: Point,
    ) -> bool {
        self.refresh_waveform_view_if_needed();
        let Some(layout) = self.shell_layout.as_ref() else {
            return false;
        };
        let Some(center_micros) =
            self.shell_state
                .waveform_scrollbar_view_center_at_point(layout, &self.model, point)
        else {
            return false;
        };
        self.last_emitted_waveform_view_center = Some(center_micros);
        self.emit_model_action_with_profile(
            UiAction::SetWaveformViewCenter {
                center_micros,
                center_nanos: None,
            },
            Some(InteractionProfileKind::Timeline),
        );
        true
    }
}

fn browser_list_pointer_action_visible_row(action: &UiAction) -> Option<usize> {
    match action {
        UiAction::FocusBrowserRow { visible_row }
        | UiAction::ToggleBrowserRowSelection { visible_row }
        | UiAction::ExtendBrowserSelectionToRow { visible_row }
        | UiAction::AddRangeBrowserSelection { visible_row } => Some(*visible_row),
        _ => None,
    }
}

fn browser_list_view_start_after_focus(
    current_view_start: usize,
    visible_count: usize,
    viewport_len: usize,
    focus_visible_row: usize,
) -> Option<usize> {
    if visible_count == 0 || viewport_len == 0 {
        return None;
    }
    if visible_count <= viewport_len {
        return Some(0);
    }
    let max_start = visible_count.saturating_sub(viewport_len);
    let edge_margin = 3usize.min(viewport_len.saturating_sub(1) / 2);
    let focus_visible_row = focus_visible_row.min(visible_count.saturating_sub(1));
    let mut view_start = current_view_start.min(max_start);
    let view_end = view_start + viewport_len;
    let top_guard = view_start + edge_margin;
    let bottom_guard = view_end.saturating_sub(edge_margin);
    if focus_visible_row < top_guard {
        view_start = focus_visible_row.saturating_sub(edge_margin);
    } else if focus_visible_row >= bottom_guard {
        view_start = focus_visible_row
            .saturating_add(edge_margin + 1)
            .saturating_sub(viewport_len);
    }
    Some(view_start.min(max_start))
}
