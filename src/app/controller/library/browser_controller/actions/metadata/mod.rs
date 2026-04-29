//! Browser metadata actions split by lane-focused ownership.
//!
//! Mutation batching, rename orchestration, and auto-rename planning stay in
//! one controller surface, but each slice now lives in a focused module.

mod mutations;
mod planning;
mod rename;

#[cfg(test)]
mod tests;

use super::super::helpers::TriageSampleContext;
use super::super::{
    auto_rename::{AutoRenameInput, build_auto_rename_stem},
    helpers::{SampleAutoRenameRequest, run_sample_auto_rename_job},
};
use super::common::format_bpm_label;
use super::*;
use crate::app::controller::jobs::{
    AnalysisMetadataMutationOp, FileOpResult, SampleAutoRenameResult,
};
use crate::app::controller::state::runtime::{
    BrowserRenameBusyDecision, BrowserRenameIntentKey, MetadataRollback,
    PendingBrowserAutoRenameIntent,
};
use std::collections::{BTreeSet, HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::sync::{Arc, atomic::AtomicBool};
use std::time::Instant;
use tracing::{info, warn};
