use super::super::super::*;
use crate::app::controller::jobs::SourceHydrationKind;
use crate::app::state::FolderPaneId;

impl AppController {
    /// Return the folder pane that currently owns the browser/waveform source.
    pub(crate) fn active_folder_pane(&self) -> FolderPaneId {
        self.ui.sources.active_folder_pane
    }

    /// Return the source currently assigned to `pane`.
    pub(crate) fn folder_pane_source(&self, pane: FolderPaneId) -> Option<SourceId> {
        self.ui
            .sources
            .folder_pane(pane)
            .source_id
            .clone()
            .filter(|id| self.library.sources.iter().any(|source| source.id == *id))
    }

    /// Return the current source-row index assigned to `pane`, when any.
    pub(crate) fn source_index_for_pane(&self, pane: FolderPaneId) -> Option<usize> {
        let source_id = self.folder_pane_source(pane)?;
        self.library
            .sources
            .iter()
            .position(|source| source.id == source_id)
    }

    /// Return the source id for a visible source-row index, when it exists.
    pub(crate) fn source_id_for_index(&self, index: usize) -> Option<SourceId> {
        self.library
            .sources
            .get(index)
            .map(|source| source.id.clone())
    }

    /// Copy the active compatibility folder UI back into the retained pane slot.
    pub(crate) fn sync_active_folder_ui_to_pane(&mut self) {
        let pane = self.ui.sources.active_folder_pane;
        self.ui.sources.folder_pane_mut(pane).browser = self.ui.sources.folders.clone();
    }

    /// Load one retained pane UI into the active compatibility folder slot.
    pub(crate) fn load_active_folder_ui_from_pane(&mut self) {
        let pane = self.ui.sources.active_folder_pane;
        self.ui.sources.folders = self.ui.sources.folder_pane(pane).browser.clone();
    }

    /// Change which pane drives the sample browser and waveform.
    pub(crate) fn select_folder_pane(&mut self, pane: FolderPaneId) {
        if self.ui.sources.active_folder_pane == pane {
            return;
        }
        if let Some(pending) = self.runtime.source_lane.hydration.pending_active.clone() {
            self.finish_source_loading(pending.kind, pending.pane);
        }
        self.sync_active_folder_ui_to_pane();
        self.ui.sources.active_folder_pane = pane;
        self.selection_state.ctx.selected_source = self.folder_pane_source(pane);
        self.selection_state.ctx.last_selected_browsable_source =
            self.selection_state.ctx.selected_source.clone();
        self.load_active_folder_ui_from_pane();
        self.refresh_sources_ui();
        self.refresh_folder_browser();
        let _ = self.refresh_wavs();
        let _ = self.persist_config("Failed to save selection");
    }

    /// Assign a source to one pane without immediately changing the active pane.
    pub(crate) fn assign_source_to_folder_pane(
        &mut self,
        pane: FolderPaneId,
        id: Option<SourceId>,
    ) {
        self.ui.sources.folder_pane_mut(pane).source_id = id;
    }

    pub(super) fn queue_active_source_hydration(&mut self, id: Option<SourceId>) {
        self.queue_source_hydration(
            self.active_folder_pane(),
            SourceHydrationKind::ActiveSelection,
            id,
        );
    }

    pub(super) fn queue_inactive_pane_hydration(&mut self, pane: FolderPaneId, id: SourceId) {
        self.queue_source_hydration(pane, SourceHydrationKind::InactivePane, Some(id));
    }

    pub(super) fn finish_empty_inactive_pane_loading(&mut self, pane: FolderPaneId) {
        self.finish_source_loading(SourceHydrationKind::InactivePane, pane);
    }
}
