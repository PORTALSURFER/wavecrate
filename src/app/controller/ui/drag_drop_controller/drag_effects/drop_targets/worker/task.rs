use std::path::PathBuf;

use crate::app::controller::jobs::{DropTargetTransferKind, DropTargetTransferRequest};
use crate::sample_sources::SourceId;

pub(in crate::app::controller::ui::drag_drop_controller::drag_effects::drop_targets) struct DropTargetTransferTask
{
    pub(in crate::app::controller::ui::drag_drop_controller::drag_effects::drop_targets) kind:
        DropTargetTransferKind,
    pub(in crate::app::controller::ui::drag_drop_controller::drag_effects::drop_targets) target_source_id:
        SourceId,
    pub(in crate::app::controller::ui::drag_drop_controller::drag_effects::drop_targets) target_root:
        PathBuf,
    pub(in crate::app::controller::ui::drag_drop_controller::drag_effects::drop_targets) target_relative_folder:
        PathBuf,
    pub(in crate::app::controller::ui::drag_drop_controller::drag_effects::drop_targets) requests:
        Vec<DropTargetTransferRequest>,
    pub(in crate::app::controller::ui::drag_drop_controller::drag_effects::drop_targets) errors:
        Vec<String>,
}
