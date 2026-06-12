use super::*;

pub(super) fn project_source_rows(ui: &UiState) -> RetainedVec<SourceRowModel> {
    ui.sources
        .rows
        .iter()
        .enumerate()
        .map(|(row_index, row)| {
            let upper_assigned = ui
                .sources
                .folder_pane(FolderPaneId::Upper)
                .source_id
                .as_ref()
                .is_some_and(|source_id| *source_id == row.id);
            let lower_assigned = ui
                .sources
                .folder_pane(FolderPaneId::Lower)
                .source_id
                .as_ref()
                .is_some_and(|source_id| *source_id == row.id);
            SourceRowModel::new(
                row.name.clone(),
                row.path.clone(),
                ui.sources
                    .selected
                    .is_some_and(|selected| selected == row_index),
                row.missing,
            )
            .with_pane_assignment(upper_assigned, lower_assigned)
        })
        .collect::<Vec<_>>()
        .into()
}

pub(super) fn project_loading_source_row(ui: &UiState) -> Option<usize> {
    ui.sources
        .loading_source_id
        .as_ref()
        .and_then(|source_id| ui.sources.rows.iter().position(|row| row.id == *source_id))
}

pub(super) fn project_mutation_busy_source_row(controller: &AppController) -> Option<usize> {
    controller
        .ui
        .sources
        .rows
        .iter()
        .position(|row| controller.source_has_pending_file_mutations(&row.id))
}
