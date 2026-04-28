use super::*;
use crate::app::FolderRowKind;

impl NativeShellState {
    /// Return the projected inline folder-edit row index in the active pane, when present.
    pub(crate) fn folder_create_row_index(&self, model: &AppModel) -> Option<usize> {
        let pane_model = model.sources.active_folder_pane_model();
        pane_model
            .folder_rows
            .iter()
            .position(|row| row.kind == FolderRowKind::RenameDraft)
            .or_else(|| {
                pane_model
                    .folder_rows
                    .iter()
                    .position(|row| row.kind == FolderRowKind::CreateDraft)
            })
    }

    /// Return the folder-create input field rect for the active inline edit row.
    pub(crate) fn folder_create_input_rect(
        &mut self,
        layout: &ShellLayout,
        model: &AppModel,
    ) -> Option<Rect> {
        let (row_rect, row_depth, sizing) = active_folder_edit_row(layout, model, self)?;
        Some(folder_create_field_rect(row_rect, sizing, row_depth))
    }

    /// Return the folder-create input text rect for the active inline edit row.
    pub(crate) fn folder_create_text_rect(
        &mut self,
        layout: &ShellLayout,
        model: &AppModel,
    ) -> Option<Rect> {
        let (row_rect, row_depth, sizing) = active_folder_edit_row(layout, model, self)?;
        let field_rect = folder_create_field_rect(row_rect, sizing, row_depth);
        Some(folder_create_text_rect(field_rect, sizing))
    }

    /// Return whether a point falls inside the inline folder editor input field.
    pub(crate) fn folder_create_input_at_point(
        &mut self,
        layout: &ShellLayout,
        model: &AppModel,
        point: Point,
    ) -> bool {
        self.folder_create_input_rect(layout, model)
            .is_some_and(|rect| rect.contains(point))
    }
}

fn active_folder_edit_row(
    layout: &ShellLayout,
    model: &AppModel,
    shell_state: &mut NativeShellState,
) -> Option<(Rect, usize, SizingTokens)> {
    let style = style_for_layout(layout);
    let pane = model.sources.active_folder_pane;
    let row_index = shell_state.folder_create_row_index(model)?;
    let row = model
        .sources
        .active_folder_pane_model()
        .folder_rows
        .get(row_index)?;
    let row_rect = shell_state
        .cached_folder_rows(layout, &style, model, pane)
        .iter()
        .find(|rendered_row| rendered_row.row_index == row_index)?
        .rect;
    Some((row_rect, row.depth, style.sizing))
}

pub(in crate::gui::native_shell::state) fn folder_create_field_rect(
    row_rect: Rect,
    sizing: SizingTokens,
    depth: usize,
) -> Rect {
    let depth_indent = compute_sidebar_folder_row_depth_indent(row_rect, sizing, depth);
    let label_rect = compute_sidebar_folder_row_layout(row_rect, sizing, depth_indent).label_rect;
    let horizontal_inset = sizing.text_inset_x.max(4.0) * 0.5;
    let vertical_inset = sizing.text_inset_y.max(2.0) * 0.5;
    Rect::from_min_max(
        Point::new(
            (label_rect.min.x - horizontal_inset).max(row_rect.min.x),
            row_rect.min.y + vertical_inset,
        ),
        Point::new(
            row_rect.max.x - horizontal_inset,
            row_rect.max.y - vertical_inset,
        ),
    )
}

pub(in crate::gui::native_shell::state) fn folder_create_text_rect(
    field_rect: Rect,
    sizing: SizingTokens,
) -> Rect {
    compute_action_button_text_rect(field_rect, sizing)
}
