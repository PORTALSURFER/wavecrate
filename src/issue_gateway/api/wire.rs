//! Wire DTOs, response parsing, and status mapping for the issue gateway.

use regex::Regex;
use serde::{Deserialize, Serialize};
use std::fmt;
use std::sync::LazyLock;

const REDACTED_TEXT_LIMIT: usize = 240;

static BEARER_TOKEN_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"(?i)\bBearer\s+[A-Za-z0-9._~+/=-]+").expect("bearer regex"));
static SECRET_KEY_VALUE_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?i)\b(authorization|token|secret|api[_-]?key|password)\s*[:=]\s*\S+")
        .expect("secret key/value regex")
});
static LOCAL_PATH_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r#"(?i)(?:[A-Z]:\\[^\s"'<>]+|/(?:Users|Volumes|private|var|tmp|home)/[^\s"'<>]+)"#)
        .expect("local path regex")
});
static TOKEN_LIKE_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"\b[A-Za-z0-9_-]{32,}\b").expect("token-like string regex"));

/// Payload sent to the issue gateway when creating a GitHub issue.
#[derive(Clone, Debug, Serialize)]
pub struct CreateIssueRequest {
    /// Issue title, including any prefix for routing.
    pub title: String,
    /// Optional issue body supplied by the user.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub body: Option<String>,
}

/// Successful issue creation response returned by the gateway.
#[derive(Clone, Debug, Deserialize)]
pub struct CreateIssueResponse {
    /// Whether the gateway reported success.
    pub ok: bool,
    /// URL for the created GitHub issue.
    pub issue_url: String,
    /// GitHub issue number.
    pub number: u64,
}

/// Error states surfaced when creating a GitHub issue.
#[derive(Debug, thiserror::Error)]
pub enum CreateIssueError {
    /// Token is invalid or expired.
    #[error("Token invalid or expired")]
    Unauthorized,
    /// Request payload was invalid.
    #[error("Invalid input: {0}")]
    BadRequest(GatewayErrorContext),
    /// Gateway rate limit was hit.
    #[error("Rate limited; try again later")]
    RateLimited,
    /// Gateway returned a server error.
    #[error("Server error: {0}")]
    ServerError(GatewayErrorContext),
    /// Gateway returned an unrecognized HTTP status.
    #[error("Unexpected gateway status: {0}")]
    UnexpectedStatus(GatewayErrorContext),
    /// Transport error when calling the gateway.
    #[error("HTTP error: {0}")]
    Transport(String),
    /// JSON parsing/serialization error.
    #[error("JSON error: {0}")]
    Json(String),
}

/// Safe, bounded metadata extracted from an issue-gateway error response.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct GatewayErrorContext {
    status_code: u16,
    class: Option<String>,
    request_id: Option<String>,
}

impl GatewayErrorContext {
    fn from_body(status_code: u16, body: &str) -> Self {
        let parsed = serde_json::from_str::<serde_json::Value>(body).ok();
        Self {
            status_code,
            class: parsed
                .as_ref()
                .and_then(gateway_error_class)
                .or_else(|| Some(default_gateway_error_class(status_code).to_string())),
            request_id: parsed.as_ref().and_then(gateway_request_id),
        }
    }
}

impl CreateIssueError {
    #[cfg(test)]
    pub(crate) fn status_for_tests(code: u16, body: impl Into<String>) -> Self {
        map_status_error(code, body.into())
    }
}

impl fmt::Display for GatewayErrorContext {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "HTTP {}", self.status_code)?;
        if let Some(class) = self.class.as_deref() {
            write!(formatter, " ({class})")?;
        }
        if let Some(request_id) = self.request_id.as_deref() {
            write!(formatter, ", request id {request_id}")?;
        }
        Ok(())
    }
}

/// Error states surfaced when fetching or polling auth tokens.
#[derive(Debug, thiserror::Error)]
pub enum IssueAuthError {
    /// Auth response was malformed or missing data.
    #[error("Invalid auth response: {0}")]
    InvalidResponse(String),
    /// Auth polling exceeded the configured time or attempt limit.
    #[error("Auth polling timed out after {attempts} attempts ({elapsed_seconds}s)")]
    TimedOut {
        /// Number of poll attempts made before timing out.
        attempts: u32,
        /// Seconds elapsed before timing out.
        elapsed_seconds: u64,
    },
    /// Gateway returned a server error.
    #[error("Server error: {0}")]
    ServerError(String),
    /// Transport error when calling the gateway.
    #[error("HTTP error: {0}")]
    Transport(String),
}

#[derive(Clone, Debug, Deserialize)]
struct CreateIssueResponseWire {
    #[serde(default)]
    ok: bool,
    issue_url: Option<String>,
    number: Option<u64>,
    error: Option<String>,
    message: Option<String>,
}

#[derive(Deserialize)]
struct PollResponseWire {
    #[serde(rename = "sessionId")]
    session_id: Option<String>,
    error: Option<String>,
}

pub(super) fn map_status_error(code: u16, body: String) -> CreateIssueError {
    let context = GatewayErrorContext::from_body(code, &body);
    match code {
        400 => CreateIssueError::BadRequest(context),
        401 => CreateIssueError::Unauthorized,
        429 => CreateIssueError::RateLimited,
        500..=599 => CreateIssueError::ServerError(context),
        _ => CreateIssueError::UnexpectedStatus(context),
    }
}

pub(super) fn map_auth_status_error(code: u16, body: String) -> IssueAuthError {
    let context = GatewayErrorContext::from_body(code, &body);
    IssueAuthError::ServerError(context.to_string())
}

pub(super) fn parse_create_issue_response(
    body: &str,
) -> Result<CreateIssueResponse, CreateIssueError> {
    let trimmed = body.trim();
    if trimmed.is_empty() {
        return Err(CreateIssueError::Json("Empty response body".to_string()));
    }
    let parsed: CreateIssueResponseWire = serde_json::from_str(trimmed)
        .map_err(|err| CreateIssueError::Json(redact_issue_gateway_text(&err.to_string())))?;

    let ok = parsed.ok;
    if let (Some(issue_url), Some(number)) = (parsed.issue_url, parsed.number) {
        return Ok(CreateIssueResponse {
            ok: true,
            issue_url,
            number,
        });
    }
    let message = parsed
        .error
        .or(parsed.message)
        .unwrap_or_else(|| format!("Missing issue_url/number in response (ok={ok})"));
    if is_session_expired_message(&message) {
        return Err(CreateIssueError::Unauthorized);
    }
    Err(CreateIssueError::Json(safe_create_issue_response_error(
        &message,
    )))
}

pub(super) fn parse_poll_issue_token_response(
    body: &str,
) -> Result<Option<String>, IssueAuthError> {
    let parsed: PollResponseWire = serde_json::from_str(body)
        .map_err(|err| IssueAuthError::InvalidResponse(err.to_string()))?;

    if let Some(token) = parsed.session_id
        && looks_like_issue_token(&token)
    {
        return Ok(Some(token));
    }

    if let Some(err) = parsed.error {
        return Err(IssueAuthError::ServerError(safe_auth_response_error(&err)));
    }

    Ok(None)
}

pub(super) fn parse_issue_token(body: &str) -> Result<String, IssueAuthError> {
    let trimmed = body.trim();
    if trimmed.is_empty() {
        return Err(IssueAuthError::InvalidResponse(
            "Empty response body".to_string(),
        ));
    }
    if looks_like_issue_token(trimmed) {
        return Ok(trimmed.to_string());
    }
    if trimmed.starts_with('{')
        && let Ok(value) = serde_json::from_str::<serde_json::Value>(trimmed)
        && let Some(token) = value.get("token").and_then(|token| token.as_str())
        && looks_like_issue_token(token)
    {
        return Ok(token.to_string());
    }

    let mut saw_marker = false;
    for line in trimmed.lines() {
        let candidate = line.trim();
        let lowered = candidate.to_ascii_lowercase();
        if lowered.contains("copy this token") || lowered.contains("paste this token") {
            saw_marker = true;
            continue;
        }
        if saw_marker && looks_like_issue_token(candidate) {
            return Ok(candidate.to_string());
        }
    }
    Err(IssueAuthError::InvalidResponse(
        "Token not found in response".to_string(),
    ))
}

pub(crate) fn looks_like_issue_token(token: &str) -> bool {
    let trimmed = token.trim();
    if trimmed.len() < 20 || trimmed.len() > 200 {
        return false;
    }
    trimmed
        .chars()
        .all(|ch| ch.is_ascii_alphanumeric() || ch == '_' || ch == '-')
}

fn is_session_expired_message(message: &str) -> bool {
    let lowered = message.trim().to_ascii_lowercase();
    lowered.contains("session") && lowered.contains("expired")
}

fn safe_create_issue_response_error(_message: &str) -> String {
    "Gateway response was missing issue details".to_string()
}

fn safe_auth_response_error(_message: &str) -> String {
    "Gateway auth failed".to_string()
}

pub(crate) fn redact_issue_gateway_text(input: &str) -> String {
    let body = input.replace(['\r', '\n', '\t'], " ");
    let body = BEARER_TOKEN_RE.replace_all(&body, "Bearer [redacted]");
    let body = SECRET_KEY_VALUE_RE.replace_all(&body, "$1=[redacted]");
    let body = LOCAL_PATH_RE.replace_all(&body, "[local-path]");
    let body = TOKEN_LIKE_RE.replace_all(&body, "[redacted-token]");
    bound_redacted_text(body.trim())
}

fn bound_redacted_text(input: &str) -> String {
    let mut output = String::new();
    for ch in input.chars() {
        if output.len() >= REDACTED_TEXT_LIMIT {
            output.push_str("...");
            break;
        }
        output.push(ch);
    }
    output
}

fn gateway_error_class(value: &serde_json::Value) -> Option<String> {
    ["error_class", "errorClass", "code", "type"]
        .into_iter()
        .filter_map(|key| value.get(key).and_then(|value| value.as_str()))
        .find_map(safe_metadata_token)
}

fn gateway_request_id(value: &serde_json::Value) -> Option<String> {
    ["request_id", "requestId", "trace_id", "traceId"]
        .into_iter()
        .filter_map(|key| value.get(key).and_then(|value| value.as_str()))
        .find_map(safe_metadata_token)
}

fn safe_metadata_token(value: &str) -> Option<String> {
    let trimmed = value.trim();
    if trimmed.is_empty() || trimmed.len() > 96 {
        return None;
    }
    if !trimmed
        .chars()
        .all(|ch| ch.is_ascii_alphanumeric() || matches!(ch, '-' | '_' | '.' | ':'))
    {
        return None;
    }
    let redacted = redact_issue_gateway_text(trimmed);
    if redacted.contains("[redacted") || redacted.contains("[local-path]") {
        return None;
    }
    Some(redacted)
}

fn default_gateway_error_class(status_code: u16) -> &'static str {
    match status_code {
        400 => "bad_request",
        500..=599 => "server_error",
        _ => "unexpected_status",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_success_without_ok_field() {
        let body = r#"{ "issue_url": "https://github.com/PORTALSURFER/wavecrate/issues/123", "number": 123 }"#;
        let parsed = parse_create_issue_response(body).unwrap();
        assert!(parsed.ok);
        assert_eq!(parsed.number, 123);
    }

    #[test]
    fn reports_error_field_without_echoing_gateway_body() {
        let err = parse_create_issue_response(r#"{ "error": "nope" }"#).unwrap_err();
        assert_eq!(
            err.to_string(),
            "JSON error: Gateway response was missing issue details"
        );
    }

    #[test]
    fn maps_session_expired_to_unauthorized() {
        let err = parse_create_issue_response(r#"{ "error": "Session expired. Reconnect." }"#)
            .unwrap_err();
        assert!(matches!(err, CreateIssueError::Unauthorized));
    }

    #[test]
    fn maps_status_errors_by_gateway_contract() {
        assert!(matches!(
            map_status_error(401, String::from("expired")),
            CreateIssueError::Unauthorized
        ));
        assert!(matches!(
            map_status_error(429, String::from("slow down")),
            CreateIssueError::RateLimited
        ));
        assert!(matches!(
            map_status_error(503, String::from("unavailable")),
            CreateIssueError::ServerError(_)
        ));
        assert!(matches!(
            map_status_error(418, String::from("teapot")),
            CreateIssueError::UnexpectedStatus(_)
        ));
    }

    #[test]
    fn status_errors_do_not_display_raw_gateway_bodies() {
        let body = r#"{
            "error": "Authorization: Bearer secret-token-12345678901234567890",
            "message": "Issue body: private production note from /Users/portal/secret.wav",
            "request_id": "req_public_42"
        }"#;

        for status in [400, 500, 599, 418] {
            let message = map_status_error(status, body.to_string()).to_string();
            assert!(!message.contains("secret-token"));
            assert!(!message.contains("Bearer secret"));
            assert!(!message.contains("private production note"));
            assert!(!message.contains("/Users/portal"));
            assert!(message.contains(&format!("HTTP {status}")));
            assert!(message.contains("req_public_42"));
        }
    }

    #[test]
    fn auth_status_errors_do_not_display_raw_gateway_bodies() {
        let body = r#"{
            "error": "Authorization: Bearer secret-token-12345678901234567890",
            "message": "Issue body: private production note from /Users/portal/secret.wav",
            "request_id": "req_public_42"
        }"#;

        let message = map_auth_status_error(503, body.to_string()).to_string();

        assert!(!message.contains("secret-token"));
        assert!(!message.contains("Bearer secret"));
        assert!(!message.contains("private production note"));
        assert!(!message.contains("/Users/portal"));
        assert!(message.contains("HTTP 503"));
        assert!(message.contains("req_public_42"));
    }

    #[test]
    fn poll_response_error_does_not_display_raw_gateway_body_field() {
        let message = parse_poll_issue_token_response(
            r#"{ "error": "Authorization: Bearer secret-token-12345678901234567890 /Users/portal/private.wav" }"#,
        )
        .unwrap_err()
        .to_string();

        assert!(!message.contains("secret-token"));
        assert!(!message.contains("/Users/portal"));
        assert_eq!(message, "Server error: Gateway auth failed");
    }

    #[test]
    fn malformed_create_issue_response_does_not_display_raw_body() {
        let body = r#"not json Authorization: Bearer secret-token-12345678901234567890 /Users/portal/private.wav"#;

        let message = parse_create_issue_response(body).unwrap_err().to_string();

        assert!(!message.contains("secret-token"));
        assert!(!message.contains("/Users/portal"));
        assert!(!message.contains("not json"));
        assert!(message.contains("JSON error:"));
    }

    #[test]
    fn redacts_gateway_text_defensively() {
        let redacted = redact_issue_gateway_text(
            "Authorization: Bearer secret-token-12345678901234567890 token=abcDEF1234567890abcDEF1234567890abc /Users/portal/private.wav C:\\Users\\portal\\private.wav",
        );

        assert!(!redacted.contains("secret-token"));
        assert!(!redacted.contains("abcDEF1234567890abc"));
        assert!(!redacted.contains("/Users/portal"));
        assert!(!redacted.contains("C:\\Users"));
        assert!(redacted.contains("Authorization=[redacted]"));
        assert!(redacted.contains("token=[redacted]"));
        assert!(redacted.contains("[local-path]"));
    }

    #[test]
    fn parses_issue_token_from_auth_body() {
        let body = "GitHub connected\n\nCopy this token into the app:\n\nabcDEF123_-xyz000000\n\nYou can close this tab.";
        let token = parse_issue_token(body).unwrap();
        assert_eq!(token, "abcDEF123_-xyz000000");
    }

    #[test]
    fn parses_issue_token_from_json_body() {
        let token = parse_issue_token(r#"{ "token": "abcDEF123_-xyz000000" }"#).unwrap();
        assert_eq!(token, "abcDEF123_-xyz000000");
    }

    #[test]
    fn rejects_auth_body_without_token() {
        let err = parse_issue_token("No token here").unwrap_err();
        assert!(err.to_string().contains("Token not found"));
    }

    #[test]
    fn parses_poll_response_with_session_id() {
        let token =
            parse_poll_issue_token_response(r#"{ "sessionId": "abcDEF123_-xyz000000" }"#).unwrap();
        assert_eq!(token.as_deref(), Some("abcDEF123_-xyz000000"));
    }

    #[test]
    fn poll_response_without_session_id_is_pending() {
        assert!(parse_poll_issue_token_response(r#"{}"#).unwrap().is_none());
    }

    #[test]
    fn poll_response_error_maps_to_server_error() {
        let err = parse_poll_issue_token_response(r#"{ "error": "denied" }"#).unwrap_err();
        assert!(matches!(err, IssueAuthError::ServerError(_)));
    }
}
