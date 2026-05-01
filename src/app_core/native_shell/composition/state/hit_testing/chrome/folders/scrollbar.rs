use self::sempal_crate::app as native_model;
use super::*;
use crate as sempal_crate;
use native_model::FolderPaneIdModel;

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
        [FolderPaneIdModel::Upper, FolderPaneIdModel::Lower]
            .into_iter()
            .find_map(|pane| folder_scrollbar_thumb_hit(self, layout, model, pane, point))
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
        [FolderPaneIdModel::Upper, FolderPaneIdModel::Lower]
            .into_iter()
            .find_map(|pane| folder_scrollbar_track_jump(self, layout, model, pane, point))
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
    let hit_rect = Rect::from_min_max(
        Point::new(
            scrollbar.track.min.x - FOLDER_SCROLLBAR_THUMB_HIT_SLOP,
            scrollbar.thumb.min.y - FOLDER_SCROLLBAR_THUMB_HIT_SLOP,
        ),
        Point::new(
            scrollbar.track.max.x + FOLDER_SCROLLBAR_THUMB_HIT_SLOP,
            scrollbar.thumb.max.y + FOLDER_SCROLLBAR_THUMB_HIT_SLOP,
        ),
    );
    hit_rect.contains(point).then_some((
        pane,
        (point.y - scrollbar.thumb.min.y).clamp(0.0, scrollbar.thumb.height()),
    ))
}

fn folder_scrollbar_track_jump(
    shell_state: &mut NativeShellState,
    layout: &ShellLayout,
    model: &AppModel,
    pane: FolderPaneIdModel,
    point: Point,
) -> Option<(FolderPaneIdModel, usize)> {
    let (scrollbar, viewport_len) = shell_state.cached_folder_scrollbar(layout, model, pane)?;
    if !scrollbar.track.contains(point) || scrollbar.thumb.contains(point) {
        return None;
    }
    folder_scrollbar_view_start_for_pointer(
        scrollbar,
        viewport_len,
        model.sources.folder_pane(pane).tree_rows.len(),
        point.y,
        scrollbar.thumb.height() * 0.5,
    )
    .map(|view_start| (pane, view_start))
}
