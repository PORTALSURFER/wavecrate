use super::*;
use crate::app_core::native_shell::runtime_contract::FolderPaneIdModel;
use crate::gui::list::{
    VirtualListScrollbar, virtual_list_scrollbar_thumb_offset_at_point,
    virtual_list_scrollbar_view_start_at_point,
};

impl NativeShellState {
    pub(crate) fn folder_viewport_len(
        &mut self,
        layout: &ShellLayout,
        model: &AppModel,
        pane: FolderPaneIdModel,
    ) -> usize {
        let style = style_for_layout(layout);
        self.cached_tree_rows(layout, &style, model, pane)
            .len()
            .min(model.sources.folder_pane(pane).tree_rows.len())
    }

    pub(crate) fn folder_viewport_start_row(
        &mut self,
        layout: &ShellLayout,
        model: &AppModel,
        pane: FolderPaneIdModel,
    ) -> Option<usize> {
        let style = style_for_layout(layout);
        self.cached_tree_rows(layout, &style, model, pane)
            .first()
            .map(|row| row.row_index)
    }

    pub(crate) fn folder_scrollbar_thumb_offset_at_point(
        &mut self,
        layout: &ShellLayout,
        model: &AppModel,
        point: Point,
    ) -> Option<(FolderPaneIdModel, f32)> {
        folder_scrollbar_thumb_hit(self, layout, model, model.sources.active_folder_pane, point)
    }

    pub(crate) fn folder_scrollbar_view_start_for_drag(
        &mut self,
        layout: &ShellLayout,
        model: &AppModel,
        pane: FolderPaneIdModel,
        pointer_y: f32,
        thumb_pointer_offset_y: f32,
    ) -> Option<usize> {
        let (scrollbar, viewport_len) = self.cached_folder_scrollbar(layout, model, pane)?;
        folder_scrollbar_view_start_for_pointer(
            scrollbar,
            viewport_len,
            model.sources.folder_pane(pane).tree_rows.len(),
            pointer_y,
            thumb_pointer_offset_y,
        )
    }

    pub(crate) fn folder_scrollbar_view_start_at_point(
        &mut self,
        layout: &ShellLayout,
        model: &AppModel,
        point: Point,
    ) -> Option<(FolderPaneIdModel, usize)> {
        folder_scrollbar_track_jump(self, layout, model, model.sources.active_folder_pane, point)
    }

    pub(crate) fn set_folder_view_start_row(
        &mut self,
        pane: FolderPaneIdModel,
        view_start_row: usize,
    ) -> bool {
        let pane_state = self.folder_pane_runtime_state_mut(pane);
        if pane_state.window_start == view_start_row && !pane_state.autoscroll {
            return false;
        }
        pane_state.window_start = view_start_row;
        pane_state.autoscroll = false;
        pane_state.cache_key = None;
        true
    }
}

fn folder_scrollbar_thumb_hit(
    shell_state: &mut NativeShellState,
    layout: &ShellLayout,
    model: &AppModel,
    pane: FolderPaneIdModel,
    point: Point,
) -> Option<(FolderPaneIdModel, f32)> {
    let (scrollbar, _) = shell_state.cached_folder_scrollbar(layout, model, pane)?;
    virtual_list_scrollbar_thumb_offset_at_point(
        VirtualListScrollbar {
            track: scrollbar.track,
            thumb: scrollbar.thumb,
        },
        point,
        FOLDER_SCROLLBAR_THUMB_HIT_SLOP,
    )
    .map(|offset| (pane, offset))
}

fn folder_scrollbar_track_jump(
    shell_state: &mut NativeShellState,
    layout: &ShellLayout,
    model: &AppModel,
    pane: FolderPaneIdModel,
    point: Point,
) -> Option<(FolderPaneIdModel, usize)> {
    let (scrollbar, viewport_len) = shell_state.cached_folder_scrollbar(layout, model, pane)?;
    virtual_list_scrollbar_view_start_at_point(
        VirtualListScrollbar {
            track: scrollbar.track,
            thumb: scrollbar.thumb,
        },
        viewport_len,
        model.sources.folder_pane(pane).tree_rows.len(),
        point,
    )
    .map(|view_start| (pane, view_start))
}
