//! Wire DTOs, response parsing, and status mapping for the issue gateway.

use serde::{Deserialize, Serialize};

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
    BadRequest(String),
    /// Gateway rate limit was hit.
    #[error("Rate limited; try again later")]
    RateLimited,
    /// Gateway returned a server error.
    #[error("Server error: {0}")]
    ServerError(String),
    /// Transport error when calling the gateway.
    #[error("HTTP error: {0}")]
    Transport(String),
    /// JSON parsing/serialization error.
    #[error("JSON error: {0}")]
    Json(String),
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
    match code {
        400 => CreateIssueError::BadRequest(body),
        401 => CreateIssueError::Unauthorized,
        429 => CreateIssueError::RateLimited,
        500..=599 => CreateIssueError::ServerError(body),
        _ => CreateIssueError::Transport(format!("HTTP {code}: {body}")),
    }
}

pub(super) fn parse_create_issue_response(
    body: &str,
) -> Result<CreateIssueResponse, CreateIssueError> {
    let trimmed = body.trim();
    if trimmed.is_empty() {
        return Err(CreateIssueError::Json("Empty response body".to_string()));
    }
    let parsed: CreateIssueResponseWire = serde_json::from_str(trimmed)
        .map_err(|err| CreateIssueError::Json(format!("{err}: {trimmed}")))?;

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
    Err(CreateIssueError::Json(message))
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
        return Err(IssueAuthError::ServerError(err));
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
    fn reports_error_field() {
        let err = parse_create_issue_response(r#"{ "error": "nope" }"#).unwrap_err();
        assert!(err.to_string().contains("nope"));
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
