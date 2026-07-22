use std::{
    thread,
    time::{Duration, Instant, SystemTime, UNIX_EPOCH},
};

use crate::sample_sources::readiness::ReadinessView;

use super::FixtureManifest;

pub(crate) fn wait_for_readiness(
    manifest: &FixtureManifest,
    timeout: Duration,
    mut advance_runtime: impl FnMut(),
) -> Result<(), String> {
    let deadline = Instant::now() + timeout;
    let mut last_diagnostics = Vec::new();
    loop {
        advance_runtime();
        let mut observed_targets = 0usize;
        let mut all_ready = true;
        last_diagnostics.clear();
        for source in &manifest.sources {
            let connection = match rusqlite::Connection::open_with_flags(
                source.root.join(".wavecrate.db"),
                rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY
                    | rusqlite::OpenFlags::SQLITE_OPEN_NO_MUTEX,
            ) {
                Ok(connection) => connection,
                Err(_) => {
                    all_ready = false;
                    continue;
                }
            };
            let now = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs() as i64;
            match ReadinessView::new(&connection).reconcile(&source.source_id, now) {
                Ok(snapshot) => {
                    observed_targets = observed_targets.saturating_add(snapshot.entries.len());
                    last_diagnostics.push(format!(
                        "{}: activity={:?}, stages={:?}",
                        source.source_id, snapshot.activity, snapshot.stage_counts
                    ));
                    all_ready &= snapshot.entries.len() == source.expected_readiness_target_count
                        && snapshot.is_fully_ready();
                }
                Err(_) => all_ready = false,
            }
        }
        if all_ready && observed_targets == manifest.expected_readiness_target_count {
            return Ok(());
        }
        if Instant::now() >= deadline {
            return Err(format!(
                "native fixture {} did not converge: expected {} readiness targets, observed {observed_targets}; {}",
                manifest.fixture,
                manifest.expected_readiness_target_count,
                last_diagnostics.join("; ")
            ));
        }
        thread::sleep(Duration::from_millis(20));
    }
}
