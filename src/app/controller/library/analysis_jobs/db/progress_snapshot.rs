/// Row decoding for cached progress snapshot aggregates.
mod decode;
/// Pure status-transition delta math plus cached delta persistence.
mod delta;
/// Wav-paths revision freshness checks for analyze-sample snapshots.
mod freshness;
/// Cached progress snapshot aggregate reads and writes.
mod read_write;
/// Snapshot table schema creation.
mod schema;
/// Bounded analysis-job state and running-count queries.
mod states;

use crate::app::controller::library::analysis_jobs::types::AnalysisProgress;
use rusqlite::{Connection, types::Value};
use std::collections::HashMap;

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum CachedProgressSnapshot {
    Fresh(AnalysisProgress),
    Missing,
    Stale,
}

/// Persisted progress state for one analysis-job row while computing snapshot deltas.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct SnapshotJobState {
    pub(crate) job_type: String,
    pub(crate) status: String,
    pub(crate) countable: bool,
}

/// Read one cached aggregate progress snapshot, bootstrapping the table on demand.
pub(crate) fn read_progress_snapshot(
    conn: &Connection,
    job_type: &str,
) -> Result<CachedProgressSnapshot, String> {
    read_write::read_progress_snapshot(conn, job_type)
}

/// Seed missing snapshot rows from the current on-disk job state before a mutation.
pub(crate) fn ensure_all_progress_snapshot_rows(conn: &Connection) -> Result<(), String> {
    read_write::ensure_all_progress_snapshot_rows(conn)
}

/// Write the full snapshot row for one job type.
pub(crate) fn write_progress_snapshot(
    conn: &Connection,
    job_type: &str,
    progress: AnalysisProgress,
) -> Result<(), String> {
    read_write::write_progress_snapshot(conn, job_type, progress)
}

/// Apply a bounded set of row-state transitions to the cached snapshots.
pub(crate) fn apply_state_transitions(
    conn: &Connection,
    transitions: impl IntoIterator<Item = (Option<SnapshotJobState>, Option<SnapshotJobState>)>,
) -> Result<(), String> {
    delta::apply_state_transitions(conn, transitions)
}

/// Load the current snapshot-relevant states for a bounded set of sample ids.
pub(crate) fn sample_states_for_job_type(
    conn: &Connection,
    job_type: &str,
    sample_ids: &[String],
) -> Result<HashMap<String, SnapshotJobState>, String> {
    states::sample_states_for_job_type(conn, job_type, sample_ids)
}

/// Load the snapshot-relevant state for one job id, if it still exists.
pub(crate) fn job_state_by_id(
    conn: &Connection,
    job_id: i64,
) -> Result<Option<SnapshotJobState>, String> {
    states::job_state_by_id(conn, job_id)
}

/// Count currently running rows that should affect snapshots, grouped by job type.
pub(crate) fn running_counts_by_job_type(
    conn: &Connection,
    where_sql: &str,
    params: Vec<Value>,
) -> Result<HashMap<String, usize>, String> {
    states::running_counts_by_job_type(conn, where_sql, params)
}
