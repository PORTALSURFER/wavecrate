use super::IssueGatewayAuthResult;
use std::{
    sync::atomic::{AtomicBool, Ordering},
    time::{Duration, Instant},
};

/// Schema token for deferred source-db maintenance metadata markers.
pub(super) const DEFERRED_MAINTENANCE_SCHEMA_TOKEN: u32 = 1;
/// Maximum fixed-delay retry attempts for deferred source-db maintenance.
pub(super) const DEFERRED_MAINTENANCE_MAX_ATTEMPTS: usize = 3;
/// Delay between deferred source-db maintenance retry attempts.
pub(super) const DEFERRED_MAINTENANCE_RETRY_DELAY: Duration = Duration::from_millis(250);

/// Polling backoff limits for issue-gateway auth token retrieval.
#[derive(Debug, Clone, Copy)]
pub(super) struct IssueGatewayPollConfig {
    pub(super) max_attempts: u32,
    pub(super) max_duration: Duration,
    pub(super) initial_delay: Duration,
    pub(super) max_delay: Duration,
}

/// Return default polling limits for issue-gateway auth token retrieval.
pub(super) fn issue_gateway_poll_config() -> IssueGatewayPollConfig {
    IssueGatewayPollConfig {
        max_attempts: 40,
        max_duration: Duration::from_secs(120),
        initial_delay: Duration::from_secs(1),
        max_delay: Duration::from_secs(10),
    }
}

/// Poll the issue gateway until token arrival, error, timeout, or cancellation.
pub(super) fn poll_issue_gateway_with_backoff(
    request_id: &str,
    cancel: &AtomicBool,
    mut poller: impl FnMut(&str) -> Result<Option<String>, crate::issue_gateway::api::IssueAuthError>,
    config: IssueGatewayPollConfig,
    mut sleep: impl FnMut(Duration),
) -> Option<IssueGatewayAuthResult> {
    let start = Instant::now();
    let mut attempts = 0u32;
    loop {
        if cancel.load(Ordering::Relaxed) {
            return None;
        }
        attempts += 1;
        match poller(request_id) {
            Ok(Some(token)) => {
                return Some(IssueGatewayAuthResult { result: Ok(token) });
            }
            Ok(None) => {}
            Err(err) => {
                return Some(IssueGatewayAuthResult { result: Err(err) });
            }
        }
        if attempts >= config.max_attempts || start.elapsed() >= config.max_duration {
            return Some(IssueGatewayAuthResult {
                result: Err(crate::issue_gateway::api::IssueAuthError::TimedOut {
                    attempts,
                    elapsed_seconds: start.elapsed().as_secs(),
                }),
            });
        }

        let delay = crate::http_client::backoff_delay(
            config.initial_delay,
            config.max_delay,
            attempts as usize,
        );
        sleep(delay);
    }
}
