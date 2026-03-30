use super::*;
use crate::app::state::BrowserDuplicateCleanupState;
use crate::app_core::state::{InlineFolderEdit, InlineFolderEditKind};

mod dirty_graph;
mod projection;
mod pull_prep;
mod waveform_queue;

fn browser_row_bucket_label(
    model: &crate::app_core::actions::NativeAppModel,
    row_label: &str,
) -> Option<String> {
    model
        .browser
        .rows
        .iter()
        .find(|row| row.label == row_label)
        .and_then(|row| row.bucket_label.clone())
}
