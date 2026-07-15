use super::super::{FileColumn, FileColumnKind, FolderBrowserState};

impl FolderBrowserState {
    pub(in crate::native_app) fn visible_file_columns(&self) -> Vec<&FileColumn> {
        let collection_active = self.collection_focus_active();
        let curation_active = self.curation_mode_enabled();
        let harvest_active = self.harvest_mode_active();
        self.sample_list
            .file_columns
            .iter()
            .filter(|column| {
                file_column_visible_in_context(
                    column.kind,
                    collection_active,
                    curation_active,
                    harvest_active,
                )
            })
            .collect()
    }

    pub(in crate::native_app) fn file_sort(&self) -> &radiant::application::DetailsSort {
        &self.sample_list.file_sort
    }

    pub(super) fn visible_file_column_placements(
        &self,
    ) -> Vec<radiant::application::DetailsColumnPlacement> {
        details_column_placements(self.visible_file_columns())
    }
}

pub(super) fn file_column_visible_in_context(
    kind: FileColumnKind,
    collection_active: bool,
    curation_active: bool,
    harvest_active: bool,
) -> bool {
    match kind {
        FileColumnKind::Curation => curation_active,
        FileColumnKind::Harvest => harvest_active,
        FileColumnKind::SourceFolder => collection_active,
        _ => true,
    }
}

pub(super) fn details_column_placements<'a>(
    columns: impl IntoIterator<Item = &'a FileColumn>,
) -> Vec<radiant::application::DetailsColumnPlacement> {
    columns
        .into_iter()
        .map(|column| {
            radiant::application::DetailsColumnPlacement::new(column.id.as_str(), column.width)
        })
        .collect()
}
