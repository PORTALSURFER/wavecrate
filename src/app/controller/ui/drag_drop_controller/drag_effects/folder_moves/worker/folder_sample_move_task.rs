//! Explicit transaction stages for in-source folder sample moves.

use super::super::super::move_transaction::{
    PreparedStagedMove, SampleMoveMetadata, load_sample_move_metadata, move_sample_file,
    prepare_staged_move, report_staged_move_failure,
};
use crate::app::controller::jobs::{FolderEntryMove, FolderSampleMoveRequest};
use crate::sample_sources::SourceDatabase;
use std::path::Path;

mod finalize;
mod journal;
mod metadata_stage;
mod prepare;
mod rollback;
mod success;

pub(super) use prepare::prepare_folder_sample_move_transaction;

/// Prepared folder-sample move that can commit DB stages and finalize the staged file.
pub(super) struct FolderSampleMoveTransaction<'a> {
    request: FolderSampleMoveRequest,
    db: &'a SourceDatabase,
    metadata: SampleMoveMetadata,
    prepared: PreparedStagedMove,
}
