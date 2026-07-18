use super::super::persistence;
use std::path::Path;
use std::time::Duration;

#[test]
fn backfill_retry_succeeds_after_failures() {
    let mut attempts = 0;
    let result = persistence::retry_backfill_write_with(
        Path::new("/tmp"),
        "test_backfill_retry",
        || {
            attempts += 1;
            if attempts < 3 {
                Err("nope".to_string())
            } else {
                Ok(())
            }
        },
        4,
        Duration::from_millis(0),
    );
    assert!(result.is_ok());
    assert_eq!(attempts, 3);
}

#[test]
fn backfill_retry_stops_after_limit() {
    let mut attempts = 0;
    let result = persistence::retry_backfill_write_with(
        Path::new("/tmp"),
        "test_backfill_retry",
        || {
            attempts += 1;
            Err("nope".to_string())
        },
        3,
        Duration::from_millis(0),
    );
    assert!(result.is_err());
    assert_eq!(attempts, 3);
}
