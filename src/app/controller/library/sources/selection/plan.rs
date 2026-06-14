use super::super::super::*;
use std::path::PathBuf;

pub(super) enum SourceSelectionPlan {
    RefreshCurrent(CurrentSourceRefreshPlan),
    ChangeActive(ActiveSourceChangePlan),
}

pub(super) struct CurrentSourceRefreshPlan {
    pub(super) id: Option<SourceId>,
    pub(super) pending_path: Option<PathBuf>,
}

pub(super) struct ActiveSourceChangePlan {
    pub(super) id: Option<SourceId>,
    pub(super) pending_path: Option<PathBuf>,
}

impl AppController {
    pub(super) fn plan_source_selection(
        &self,
        id: Option<SourceId>,
        pending_path: Option<PathBuf>,
    ) -> SourceSelectionPlan {
        if self.selection_state.ctx.selected_source == id {
            SourceSelectionPlan::RefreshCurrent(CurrentSourceRefreshPlan { id, pending_path })
        } else {
            SourceSelectionPlan::ChangeActive(ActiveSourceChangePlan { id, pending_path })
        }
    }
}
