mod backfill;
mod changed_samples;
mod duration_probe;
mod missing_features;
mod staged_samples;

pub(crate) use backfill::{
    enqueue_jobs_for_source_backfill, enqueue_jobs_for_source_backfill_full,
};
pub(crate) use changed_samples::enqueue_jobs_for_source;
pub(crate) use duration_probe::update_missing_durations_for_source;
#[cfg(test)]
pub(crate) use missing_features::enqueue_jobs_for_source_missing_features;

use super::enqueue_helpers::now_epoch_seconds;
use super::{invalidate, persist, scan};
use crate::app::controller::library::analysis_jobs::db;
use crate::app::controller::library::analysis_jobs::types::AnalysisProgress;
use crate::app::controller::library::analysis_jobs::wakeup;
use crate::logging::{ActionDebugEvent, emit_action_debug_event};
use rusqlite::params;
use std::time::Instant;
use tracing::{info, warn};
