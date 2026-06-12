//! Gateway API client for creating GitHub issues.

#![allow(clippy::result_large_err)]

mod auth;
mod issue;
mod transport;
mod wire;

pub use wire::{CreateIssueError, CreateIssueRequest, CreateIssueResponse, IssueAuthError};

/// Base URL for the issue gateway API.
pub const BASE_URL: &str = "https://wavecrate-gitissue-gateway.portalsurfer.workers.dev";
/// Direct URL for starting the auth flow in a browser.
pub const AUTH_START_URL: &str =
    "https://wavecrate-gitissue-gateway.portalsurfer.workers.dev/auth/start";

/// The kind of issue the user is submitting.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum IssueKind {
    /// Request a new feature.
    FeatureRequest,
    /// Report a bug.
    Bug,
}

impl IssueKind {
    /// Return the title prefix used when creating an issue of this kind.
    pub fn title_prefix(self) -> &'static str {
        match self {
            Self::FeatureRequest => "FR: ",
            Self::Bug => "Bug: ",
        }
    }
}

/// Start an auth session and return the token produced by the gateway.
pub fn fetch_issue_token() -> Result<String, IssueAuthError> {
    auth::fetch_issue_token(AUTH_START_URL)
}

/// Poll for a token using a request ID.
pub fn poll_issue_token(request_id: &str) -> Result<Option<String>, IssueAuthError> {
    auth::poll_issue_token(BASE_URL, request_id)
}

/// Create a GitHub issue through the gateway with idempotent retry support.
pub fn create_issue(
    token: &str,
    request: &CreateIssueRequest,
) -> Result<CreateIssueResponse, CreateIssueError> {
    issue::create_issue(BASE_URL, token, request)
}

#[cfg(test)]
mod tests {
    use super::IssueKind;

    #[test]
    fn issue_kind_prefixes_match_spec_examples() {
        assert_eq!(IssueKind::Bug.title_prefix(), "Bug: ");
        assert_eq!(IssueKind::FeatureRequest.title_prefix(), "FR: ");
    }
}
