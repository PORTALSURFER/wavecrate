use crate::logging::{DbDebugEvent, emit_db_debug_event};
use crate::sample_sources::SourceId;
use std::collections::HashMap;
use std::path::Path;
use std::time::{Duration, Instant};

/// Defines claim error log interval.
const CLAIM_ERROR_LOG_INTERVAL: Duration = Duration::from_secs(30);

#[derive(Clone, Debug, Default)]
/// Stores state for claim error log state.
pub(super) struct ClaimErrorLogState {
    failures: usize,
    #[cfg_attr(not(test), allow(dead_code))]
    last_logged: Option<Instant>,
}

/// Handles record claim error.
pub(super) fn record_claim_error(
    claim_error_logs: &mut HashMap<SourceId, ClaimErrorLogState>,
    source_id: &SourceId,
    source_root: &Path,
    err: &str,
) {
    let now = Instant::now();
    let state = claim_error_logs.entry(source_id.clone()).or_default();
    state.failures = state.failures.saturating_add(1);
    let should_log = state
        .last_logged
        .is_none_or(|last_logged| now.duration_since(last_logged) >= CLAIM_ERROR_LOG_INTERVAL);
    if !should_log {
        return;
    }
    state.last_logged = Some(now);
    let source = source_root.display().to_string();
    let busy = is_busy_error(err);
    tracing::warn!(
        target: "perf::source_db",
        action = "analysis_claim_source_error",
        source_id = source_id.as_str(),
        source_root = %source_root.display(),
        failures = state.failures,
        busy,
        error = err,
        "Analysis claim source failed; continuing with other sources"
    );
    emit_db_debug_event(DbDebugEvent {
        operation: "analysis_claim_jobs.source",
        source: Some(&source),
        outcome: "error",
        elapsed: Duration::ZERO,
        error: Some(err),
    });
}

/// Handles is busy error.
fn is_busy_error(err: &str) -> bool {
    let lowered = err.to_ascii_lowercase();
    lowered.contains("busy") || lowered.contains("locked")
}

#[cfg(test)]
/// Contains focused regression coverage for this module.
mod tests {
    use super::*;
    use crate::app::controller::library::analysis_jobs::db as analysis_db;
    use crate::app::controller::library::analysis_jobs::pool::job_claim::claim::SourceClaimDb;
    use crate::app::controller::library::analysis_jobs::pool::job_claim::selection::{
        ClaimSelection, ClaimSelector,
    };
    use crate::app::controller::library::analysis_jobs::wakeup::ClaimWakeup;
    use crate::sample_sources::SampleSource;
    use std::collections::HashSet;
    use std::io;
    use std::sync::{Arc, Mutex};
    use tracing_subscriber::fmt::MakeWriter;

    #[derive(Clone, Default)]
    /// Stores state for shared buffer.
    struct SharedBuffer(Arc<Mutex<Vec<u8>>>);

    impl SharedBuffer {
        /// Handles captured.
        fn captured(&self) -> String {
            String::from_utf8(self.0.lock().unwrap().clone()).unwrap()
        }
    }

    impl<'a> MakeWriter<'a> for SharedBuffer {
        /// Names the writer type.
        type Writer = SharedBufferWriter;

        /// Handles make writer.
        fn make_writer(&'a self) -> Self::Writer {
            SharedBufferWriter(self.0.clone())
        }
    }

    /// Stores state for shared buffer writer.
    struct SharedBufferWriter(Arc<Mutex<Vec<u8>>>);

    impl io::Write for SharedBufferWriter {
        /// Handles write.
        fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
            self.0.lock().unwrap().extend_from_slice(buf);
            Ok(buf.len())
        }

        /// Handles flush.
        fn flush(&mut self) -> io::Result<()> {
            Ok(())
        }
    }

    /// Handles capture debug logs.
    fn capture_debug_logs(run: impl FnOnce()) -> String {
        let buffer = SharedBuffer::default();
        let subscriber = tracing_subscriber::fmt()
            .with_ansi(false)
            .without_time()
            .with_max_level(tracing::Level::DEBUG)
            .with_writer(buffer.clone())
            .finish();
        crate::logging::set_debug_logging_enabled_for_tests(true);
        tracing::subscriber::with_default(subscriber, run);
        crate::logging::set_debug_logging_enabled_for_tests(false);
        buffer.captured()
    }

    #[test]
    /// Handles repeated claim errors are rate limited and source scoped.
    fn repeated_claim_errors_are_rate_limited_and_source_scoped() {
        let dir = tempfile::tempdir().unwrap();
        let source = SampleSource::new(dir.path().join("source"));
        let mut logs = HashMap::new();

        let first = capture_debug_logs(|| {
            record_claim_error(&mut logs, &source.id, &source.root, "database is locked");
        });
        assert!(
            first.contains("Analysis claim source failed; continuing with other sources"),
            "first claim error should be visible: {first}"
        );
        assert!(
            first.contains("action=\"analysis_claim_source_error\""),
            "claim error should use a stable action: {first}"
        );
        assert!(
            first.contains("busy=true"),
            "locked claim errors should be classified as busy: {first}"
        );
        assert!(
            first.contains("source_root=") && first.contains("source"),
            "claim error should include source-root context: {first}"
        );

        let second = capture_debug_logs(|| {
            record_claim_error(&mut logs, &source.id, &source.root, "database is locked");
        });
        assert!(
            !second.contains("Analysis claim source failed"),
            "immediate repeated claim errors should be rate-limited: {second}"
        );

        logs.get_mut(&source.id).unwrap().last_logged =
            Some(Instant::now() - CLAIM_ERROR_LOG_INTERVAL);
        let third = capture_debug_logs(|| {
            record_claim_error(&mut logs, &source.id, &source.root, "constraint failed");
        });
        assert!(
            third.contains("Analysis claim source failed; continuing with other sources"),
            "claim errors should log again after the interval: {third}"
        );
        assert!(
            third.contains("busy=false"),
            "non-lock claim errors should not be classified as busy: {third}"
        );
    }

    #[test]
    /// Handles empty claim sources stay quiet.
    fn empty_claim_sources_stay_quiet() {
        let dir = tempfile::tempdir().unwrap();
        let source = SampleSource::new(dir.path().join("source"));
        std::fs::create_dir_all(&source.root).unwrap();
        let conn = analysis_db::open_source_db(&source.root).unwrap();
        let mut selector = ClaimSelector::with_sources_for_tests(
            vec![SourceClaimDb { source, conn }],
            1,
            Arc::new(Mutex::new(HashSet::new())),
        );
        let claim_wakeup = ClaimWakeup::new();

        let captured = capture_debug_logs(|| {
            assert!(matches!(
                selector.select_next(None, &claim_wakeup),
                ClaimSelection::Idle
            ));
        });

        assert!(
            !captured.contains("analysis_claim_source_error"),
            "empty queues should not emit claim-error diagnostics: {captured}"
        );
    }
}
