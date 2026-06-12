//! Gateway-specific HTTP retry and response-size bounds.

use crate::http_client;
use serde::Serialize;
use std::io;
use std::time::Duration;

const MAX_AUTH_RESPONSE_BYTES: usize = 64 * 1024;
const MAX_ISSUE_RESPONSE_BYTES: usize = 256 * 1024;
const ISSUE_RETRY_CONFIG: http_client::RetryConfig = http_client::RetryConfig {
    max_attempts: 3,
    base_delay: Duration::from_millis(200),
    max_delay: Duration::from_secs(2),
};

pub(super) fn read_auth_response_text(response: ureq::Response) -> Result<String, io::Error> {
    http_client::read_response_text(response, MAX_AUTH_RESPONSE_BYTES)
}

pub(super) fn read_issue_response_text(response: ureq::Response) -> Result<String, io::Error> {
    http_client::read_response_text(response, MAX_ISSUE_RESPONSE_BYTES)
}

pub(super) fn get_with_retry(url: &str) -> Result<ureq::Response, ureq::Error> {
    http_client::retry_with_backoff(
        ISSUE_RETRY_CONFIG,
        || http_client::agent().get(url).call(),
        is_gateway_retryable_error,
    )
}

pub(super) fn post_json_with_retry<T: Serialize>(
    url: &str,
    token: &str,
    request: &T,
    idempotency_key: &str,
) -> Result<ureq::Response, ureq::Error> {
    http_client::retry_with_backoff(
        ISSUE_RETRY_CONFIG,
        || {
            http_client::agent()
                .post(url)
                .set("Accept", "application/json")
                .set("Content-Type", "application/json")
                .set("Authorization", &format!("Bearer {}", token.trim()))
                .set("Idempotency-Key", idempotency_key)
                .send_json(request)
        },
        is_gateway_retryable_error,
    )
}

fn is_gateway_retryable_error(err: &ureq::Error) -> bool {
    match err {
        ureq::Error::Transport(_) => true,
        ureq::Error::Status(code, _) => (500..=599).contains(code),
    }
}

#[cfg(test)]
mod tests {
    use super::is_gateway_retryable_error;

    #[test]
    fn status_retry_policy_retries_only_server_errors() {
        assert!(is_gateway_retryable_error(&ureq::Error::Status(
            500,
            synthetic_response()
        )));
        assert!(!is_gateway_retryable_error(&ureq::Error::Status(
            400,
            synthetic_response()
        )));
    }

    fn synthetic_response() -> ureq::Response {
        ureq::Response::new(500, "Synthetic", "").expect("synthetic response")
    }
}
