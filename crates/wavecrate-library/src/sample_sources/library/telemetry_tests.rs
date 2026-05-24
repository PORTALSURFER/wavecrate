use std::time::Duration;

use super::telemetry::{TEST_SLOW_LIBRARY_DB_EVENT_THRESHOLD, library_db_debug_outcome};

#[test]
fn fast_successful_library_events_are_suppressed() {
    assert_eq!(
        library_db_debug_outcome(
            true,
            TEST_SLOW_LIBRARY_DB_EVENT_THRESHOLD.saturating_sub(Duration::from_millis(1)),
        ),
        None
    );
}

#[test]
fn slow_successful_library_events_are_marked_slow() {
    assert_eq!(
        library_db_debug_outcome(true, TEST_SLOW_LIBRARY_DB_EVENT_THRESHOLD),
        Some("slow")
    );
}

#[test]
fn failed_library_events_are_kept() {
    assert_eq!(
        library_db_debug_outcome(false, Duration::from_millis(1)),
        Some("error")
    );
}
