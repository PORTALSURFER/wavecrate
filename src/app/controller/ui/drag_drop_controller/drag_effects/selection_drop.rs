use super::super::DragDropController;
use crate::app::controller::StatusTone;
use crate::app::controller::library::selection_export::SelectionClipExportRequest;
use crate::app::state::TriageFlagColumn;
use crate::sample_sources::{Rating, SourceId};
use crate::selection::SelectionRange;
use std::path::{Path, PathBuf};

/// Browser/folder destination details resolved for one selection drop.
pub(crate) struct SelectionDropDestination {
    /// Whether the sample browser list itself is the active drop target.
    pub browser_list_target: bool,
    /// Optional triage bucket target when dropping into triage columns.
    pub triage_target: Option<TriageFlagColumn>,
    /// Optional folder target when dropping into the source tree.
    pub folder_target: Option<PathBuf>,
}

impl DragDropController<'_> {
    pub(crate) fn handle_selection_drop(
        &mut self,
        source_id: SourceId,
        relative_path: PathBuf,
        bounds: SelectionRange,
        destination: SelectionDropDestination,
        keep_source_focused: bool,
    ) {
        if !destination.browser_list_target
            && destination.triage_target.is_none()
            && destination.folder_target.is_none()
        {
            self.set_status(
                "Drag the selection onto Samples or a folder to save it",
                StatusTone::Warning,
            );
            return;
        }
        let target_tag = destination.triage_target.map(|column| match column {
            TriageFlagColumn::Trash => Rating::TRASH_1,
            TriageFlagColumn::Neutral => Rating::NEUTRAL,
            TriageFlagColumn::Keep => Rating::KEEP_1,
        });
        if let Some(folder) = destination.folder_target.as_deref()
            && !folder.as_os_str().is_empty()
        {
            self.handle_selection_drop_to_folder(
                &source_id,
                &relative_path,
                bounds,
                folder,
                keep_source_focused,
            );
            return;
        }
        self.handle_selection_drop_to_browser(
            &source_id,
            &relative_path,
            bounds,
            target_tag,
            keep_source_focused,
        );
    }

    fn handle_selection_drop_to_folder(
        &mut self,
        source_id: &SourceId,
        relative_path: &Path,
        bounds: SelectionRange,
        folder: &Path,
        keep_source_focused: bool,
    ) {
        let Some(source) = self
            .library
            .sources
            .iter()
            .find(|s| &s.id == source_id)
            .cloned()
        else {
            self.set_status(
                "Source not available for selection export",
                StatusTone::Error,
            );
            return;
        };
        if self
            .selection_state
            .ctx
            .selected_source
            .as_ref()
            .is_some_and(|selected| selected != &source.id)
        {
            self.set_status(
                "Switch to the sample's source before saving into its folders",
                StatusTone::Warning,
            );
            return;
        }
        let destination = source.root.join(folder);
        if !destination.is_dir() {
            self.set_status(
                format!("Folder not found: {}", folder.display()),
                StatusTone::Error,
            );
            return;
        }
        match self.export_selection_clip_in_folder(
            SelectionClipExportRequest {
                source_id,
                relative_path,
                bounds,
                target_tag: None,
                add_to_browser: true,
                register_in_source: true,
            },
            folder,
        ) {
            Ok(entry) => {
                if !keep_source_focused {
                    self.ui.browser.selection.autoscroll = true;
                    self.selection_state.suppress_autoplay_once = true;
                    self.select_from_browser(&entry.relative_path);
                }
                self.set_status(
                    format!("Saved clip {}", entry.relative_path.display()),
                    StatusTone::Info,
                );
            }
            Err(err) => self.set_status(err, StatusTone::Error),
        }
    }

    fn handle_selection_drop_to_browser(
        &mut self,
        source_id: &SourceId,
        relative_path: &Path,
        bounds: SelectionRange,
        target_tag: Option<Rating>,
        keep_source_focused: bool,
    ) {
        let folder_override = self
            .selection_state
            .ctx
            .selected_source
            .as_ref()
            .is_some_and(|selected| selected == source_id)
            .then(|| {
                self.ui.sources.folders.focused.and_then(|idx| {
                    self.ui
                        .sources
                        .folders
                        .rows
                        .get(idx)
                        .map(|row| row.path.clone())
                })
            })
            .flatten()
            .filter(|path| !path.as_os_str().is_empty());
        let export = if let Some(folder) = folder_override.as_deref() {
            self.export_selection_clip_in_folder(
                SelectionClipExportRequest {
                    source_id,
                    relative_path,
                    bounds,
                    target_tag,
                    add_to_browser: true,
                    register_in_source: true,
                },
                folder,
            )
        } else {
            self.export_selection_clip(SelectionClipExportRequest {
                source_id,
                relative_path,
                bounds,
                target_tag,
                add_to_browser: true,
                register_in_source: true,
            })
        };
        match export {
            Ok(entry) => {
                if !keep_source_focused {
                    self.ui.browser.selection.autoscroll = true;
                    self.selection_state.suppress_autoplay_once = true;
                    self.select_from_browser(&entry.relative_path);
                }
                let status = format!("Saved clip {}", entry.relative_path.display());
                self.set_status(status, StatusTone::Info);
            }
            Err(err) => self.set_status(err, StatusTone::Error),
        }
    }
}
