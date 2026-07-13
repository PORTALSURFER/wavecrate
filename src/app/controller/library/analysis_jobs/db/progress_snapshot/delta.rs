use super::{SnapshotJobState, schema};
use rusqlite::{Connection, params};
use std::collections::HashMap;

type ProgressDeltas = HashMap<String, (i64, i64, i64, i64)>;

pub(super) fn apply_state_transitions(
    conn: &Connection,
    transitions: impl IntoIterator<Item = (Option<SnapshotJobState>, Option<SnapshotJobState>)>,
) -> Result<(), String> {
    schema::ensure_snapshot_schema(conn)?;
    for (job_type, (pending, running, done, failed)) in state_deltas(transitions) {
        if pending == 0 && running == 0 && done == 0 && failed == 0 {
            continue;
        }
        conn.execute(
            "INSERT INTO analysis_job_progress_snapshots (job_type, pending, running, done, failed)
             VALUES (?1, ?2, ?3, ?4, ?5)
             ON CONFLICT(job_type) DO UPDATE SET
                 pending = MAX(0, pending + excluded.pending),
                 running = MAX(0, running + excluded.running),
                 done = MAX(0, done + excluded.done),
                 failed = MAX(0, failed + excluded.failed)",
            params![job_type, pending, running, done, failed],
        )
        .map_err(|err| err.to_string())?;
    }
    Ok(())
}

fn state_deltas(
    transitions: impl IntoIterator<Item = (Option<SnapshotJobState>, Option<SnapshotJobState>)>,
) -> ProgressDeltas {
    let mut deltas = HashMap::new();
    for (before, after) in transitions {
        apply_state_delta(&mut deltas, before.as_ref(), -1);
        apply_state_delta(&mut deltas, after.as_ref(), 1);
    }
    deltas
}

fn apply_state_delta(
    deltas: &mut ProgressDeltas,
    state: Option<&SnapshotJobState>,
    direction: i64,
) {
    let Some(state) = state.filter(|state| state.countable) else {
        return;
    };
    let entry = deltas.entry(state.job_type.clone()).or_insert((0, 0, 0, 0));
    match state.status.as_str() {
        "pending" => entry.0 += direction,
        "running" => entry.1 += direction,
        "done" => entry.2 += direction,
        "failed" => entry.3 += direction,
        _ => {}
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn state(job_type: &str, status: &str, countable: bool) -> SnapshotJobState {
        SnapshotJobState {
            job_type: job_type.to_string(),
            status: status.to_string(),
            countable,
        }
    }

    #[test]
    fn state_deltas_apply_countable_status_transitions_only() {
        let deltas = state_deltas([
            (
                Some(state("analyze", "pending", true)),
                Some(state("analyze", "running", true)),
            ),
            (
                Some(state("analyze", "running", false)),
                Some(state("analyze", "done", true)),
            ),
            (
                Some(state("embed", "failed", true)),
                Some(state("embed", "unknown", true)),
            ),
        ]);

        assert_eq!(deltas.get("analyze"), Some(&(-1, 1, 1, 0)));
        assert_eq!(deltas.get("embed"), Some(&(0, 0, 0, -1)));
    }

    #[test]
    fn state_deltas_cancel_intermediate_states_across_one_batch() {
        let deltas = state_deltas([
            (
                Some(state("analyze", "pending", true)),
                Some(state("analyze", "running", true)),
            ),
            (
                Some(state("analyze", "running", true)),
                Some(state("analyze", "done", true)),
            ),
            (
                Some(state("embed", "failed", true)),
                Some(state("embed", "failed", false)),
            ),
        ]);

        assert_eq!(deltas.get("analyze"), Some(&(-1, 0, 1, 0)));
        assert_eq!(deltas.get("embed"), Some(&(0, 0, 0, -1)));
    }
}
