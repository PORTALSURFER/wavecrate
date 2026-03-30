use super::super::test_support::{dummy_controller, sample_entry, write_test_wav};
use super::super::*;
use crate::app::controller::jobs::{FolderEntryMove, FolderMoveResult, FolderSampleMoveResult};
use crate::app::state::{
    DragPayload, DragSample, DragSource, DragTarget, SampleBrowserActionPrompt,
};
use crate::app_dirs::ConfigBaseGuard;
use std::path::{Path, PathBuf};
use tempfile::tempdir;

mod folder_moves;
mod result_apply;
mod sample_moves;
