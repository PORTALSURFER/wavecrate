//! Token authorization flow for the issue gateway.

use super::transport;
use super::wire::{self, IssueAuthError, map_auth_status_error, redact_issue_gateway_text};

pub(super) fn fetch_issue_token(auth_start_url: &str) -> Result<String, IssueAuthError> {
    let response = match transport::get_with_retry(auth_start_url) {
        Ok(response) => response,
        Err(ureq::Error::Status(code, response)) => {
            let body =
                transport::read_auth_response_text(response).unwrap_or_else(|err| err.to_string());
            return Err(map_auth_status_error(code, body));
        }
        Err(ureq::Error::Transport(err)) => {
            return Err(IssueAuthError::Transport(redact_issue_gateway_text(
                &err.to_string(),
            )));
        }
    };

    let body = transport::read_auth_response_text(response).map_err(|err| {
        IssueAuthError::InvalidResponse(redact_issue_gateway_text(&err.to_string()))
    })?;
    wire::parse_issue_token(&body)
}

pub(super) fn poll_issue_token(
    base_url: &str,
    request_id: &str,
) -> Result<Option<String>, IssueAuthError> {
    let url = format!(
        "{base_url}/auth/poll?requestId={}",
        encode_uri_component(request_id)
    );
    let response = match transport::get_with_retry(&url) {
        Ok(response) => response,
        Err(ureq::Error::Status(202, _)) => return Ok(None),
        Err(ureq::Error::Status(code, response)) => {
            let body =
                transport::read_auth_response_text(response).unwrap_or_else(|err| err.to_string());
            return Err(map_auth_status_error(code, body));
        }
        Err(ureq::Error::Transport(err)) => {
            return Err(IssueAuthError::Transport(redact_issue_gateway_text(
                &err.to_string(),
            )));
        }
    };

    let body = transport::read_auth_response_text(response).map_err(|err| {
        IssueAuthError::InvalidResponse(redact_issue_gateway_text(&err.to_string()))
    })?;
    wire::parse_poll_issue_token_response(&body)
}

fn encode_uri_component(s: &str) -> String {
    url::form_urlencoded::byte_serialize(s.as_bytes()).collect()
}

#[cfg(test)]
mod tests {
    use super::encode_uri_component;

    #[test]
    fn request_id_encoding_preserves_query_boundary() {
        assert_eq!(encode_uri_component("abc 123&x=y"), "abc+123%26x%3Dy");
    }
}
