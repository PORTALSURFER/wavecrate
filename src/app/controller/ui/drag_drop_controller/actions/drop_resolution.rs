use super::*;
use crate::app_core::state::TriageFlagColumn;

/// Resolved drop-target metadata reused across payload-specific finish handlers.
#[derive(Clone, Debug, Default)]
pub(super) struct ResolvedDropTarget {
    pub(super) source_target: Option<SourceId>,
    pub(super) browser_list_target: bool,
    pub(super) triage_target: Option<TriageFlagColumn>,
    pub(super) folder_source_target: Option<SourceId>,
    pub(super) folder_target: Option<PathBuf>,
    pub(super) over_folder_panel: bool,
    pub(super) drop_target_path: Option<PathBuf>,
    pub(super) drop_targets_panel: bool,
}

pub(super) fn resolve_drop_target(
    controller: &AppController,
    active_target: &DragTarget,
) -> ResolvedDropTarget {
    let source_target = match active_target {
        DragTarget::SourcesRow(id) => Some(id.clone()),
        _ => None,
    };
    let browser_list_target = matches!(active_target, DragTarget::BrowserList);
    let (triage_target, folder_source_target, folder_target, over_folder_panel) =
        match active_target {
            DragTarget::BrowserTriage(column) => (Some(*column), None, None, false),
            DragTarget::FolderPanel { pane, folder } => {
                let target_source = controller.folder_pane_source(*pane);
                (None, target_source, folder.clone(), true)
            }
            _ => (None, None, None, false),
        };
    let drop_target_path = match active_target {
        DragTarget::DropTarget { path } => Some(path.clone()),
        _ => None,
    };
    let drop_targets_panel = matches!(active_target, DragTarget::DropTargetsPanel);

    ResolvedDropTarget {
        source_target,
        browser_list_target,
        triage_target,
        folder_source_target,
        folder_target,
        over_folder_panel,
        drop_target_path,
        drop_targets_panel,
    }
}
