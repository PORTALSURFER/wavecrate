use self::sempal_crate::app as native_model;
use super::*;
use crate as sempal_crate;
use native_model::{FolderPaneIdModel, FolderRowKind, FolderRowModel};

impl NativeShellState {
    /// Resolve a rendered folder-row index for a point within the sidebar.
    pub(crate) fn folder_row_at_point(
        &mut self,
        layout: &ShellLayout,
        model: &AppModel,
        point: Point,
    ) -> Option<(FolderPaneIdModel, usize)> {
        let style = style_for_layout(layout);
        [FolderPaneIdModel::Upper, FolderPaneIdModel::Lower]
            .into_iter()
            .find_map(|pane| {
                self.cached_tree_rows(layout, &style, model, pane)
                    .iter()
                    .find(|row| row.rect.contains(point))
                    .map(|row| (pane, row.row_index))
            })
    }

    /// Return the folder pane whose header or rows band contains the point.
    pub(crate) fn folder_panel_at_point(
        &mut self,
        layout: &ShellLayout,
        model: &AppModel,
        point: Point,
    ) -> Option<FolderPaneIdModel> {
        let style = style_for_layout(layout);
        let sections = sidebar_sections(layout, &style, model);
        [FolderPaneIdModel::Upper, FolderPaneIdModel::Lower]
            .into_iter()
            .find(|pane| {
                let folder_sections = sections.folder_header(*pane);
                folder_sections.contains(point) || sections.tree_rows(*pane).contains(point)
            })
    }

    /// Return whether a point falls within either folder pane.
    pub(crate) fn folder_panel_contains_point(
        &mut self,
        layout: &ShellLayout,
        model: &AppModel,
        point: Point,
    ) -> bool {
        self.folder_panel_at_point(layout, model, point).is_some()
    }

    /// Resolve a rendered folder-row disclosure click target for a point within the sidebar.
    pub(crate) fn folder_row_disclosure_at_point(
        &mut self,
        layout: &ShellLayout,
        model: &AppModel,
        point: Point,
    ) -> Option<(FolderPaneIdModel, usize)> {
        let (pane, row_index) = self.folder_row_at_point(layout, model, point)?;
        let pane_model = model.sources.folder_pane(pane);
        if !pane_model.tree_search_query.trim().is_empty() {
            return None;
        }
        let style = style_for_layout(layout);
        let rendered_row = self
            .cached_tree_rows(layout, &style, model, pane)
            .iter()
            .find(|row| row.row_index == row_index)?;
        let row = pane_model.tree_rows.get(row_index)?;
        if row_has_disclosure_target(row) {
            return None;
        }
        let depth_indent =
            compute_sidebar_folder_row_depth_indent(rendered_row.rect, style.sizing, row.depth);
        let disclosure_rect =
            compute_sidebar_folder_row_layout(rendered_row.rect, style.sizing, depth_indent)
                .disclosure_rect;
        disclosure_rect.contains(point).then_some((pane, row_index))
    }

    /// Return one rendered folder-row disclosure gutter rect for tests.
    #[cfg(test)]
    pub(crate) fn folder_row_disclosure_rect(
        &mut self,
        layout: &ShellLayout,
        model: &AppModel,
        row_index: usize,
    ) -> Option<Rect> {
        let style = style_for_layout(layout);
        let pane = model.sources.active_folder_pane;
        let row = model
            .sources
            .active_folder_pane_model()
            .tree_rows
            .get(row_index)?;
        let row_rect = self
            .cached_tree_rows(layout, &style, model, pane)
            .iter()
            .find(|rendered_row| rendered_row.row_index == row_index)?
            .rect;
        let depth_indent =
            compute_sidebar_folder_row_depth_indent(row_rect, style.sizing, row.depth);
        Some(
            compute_sidebar_folder_row_layout(row_rect, style.sizing, depth_indent).disclosure_rect,
        )
    }

    /// Return rendered folder-row rectangles for geometry tests.
    #[cfg(test)]
    pub(crate) fn rendered_folder_row_rects(
        &mut self,
        layout: &ShellLayout,
        model: &AppModel,
    ) -> Vec<Rect> {
        let style = style_for_layout(layout);
        let pane = model.sources.active_folder_pane;
        self.cached_tree_rows(layout, &style, model, pane)
            .iter()
            .map(|row| row.rect)
            .collect()
    }
}

fn row_has_disclosure_target(row: &FolderRowModel) -> bool {
    matches!(
        row.kind,
        FolderRowKind::CreateDraft | FolderRowKind::RenameDraft
    ) || row.is_root
        || !row.has_children
}
