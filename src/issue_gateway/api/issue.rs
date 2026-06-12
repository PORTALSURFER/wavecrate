//! Issue creation request orchestration for the gateway.

use super::transport;
use super::wire::{
    CreateIssueError, CreateIssueRequest, CreateIssueResponse, map_status_error,
    parse_create_issue_response,
};
use uuid::Uuid;

pub(super) fn create_issue(
    base_url: &str,
    token: &str,
    request: &CreateIssueRequest,
) -> Result<CreateIssueResponse, CreateIssueError> {
    let url = format!("{base_url}/issue");
    let idempotency_key = format!("issue-{}", Uuid::new_v4());
    let response = match transport::post_json_with_retry(&url, token, request, &idempotency_key) {
        Ok(response) => response,
        Err(ureq::Error::Status(code, response)) => {
            let body =
                transport::read_issue_response_text(response).unwrap_or_else(|err| err.to_string());
            return Err(map_status_error(code, body));
        }
        Err(ureq::Error::Transport(err)) => {
            return Err(CreateIssueError::Transport(err.to_string()));
        }
    };

    let body = transport::read_issue_response_text(response)
        .map_err(|err| CreateIssueError::Json(err.to_string()))?;
    parse_create_issue_response(&body)
}
