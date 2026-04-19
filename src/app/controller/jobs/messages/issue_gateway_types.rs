//! Issue-gateway creation/auth/token DTOs for controller background jobs.

/// Request payload for creating a new issue through the gateway worker.
#[derive(Debug)]
pub(crate) struct IssueGatewayJob {
    /// Bearer token used to authorize issue creation.
    pub(crate) token: String,
    /// Serialized issue request body sent to the gateway API.
    pub(crate) request: crate::issue_gateway::api::CreateIssueRequest,
}

/// Poll request for gateway-issued auth completion by request id.
#[derive(Debug)]
pub(crate) struct IssueGatewayPollJob {
    /// Opaque request identifier returned by the create/auth kickoff flow.
    pub(crate) request_id: String,
}

/// Outcome of an issue-create request sent through the gateway worker.
#[derive(Debug)]
pub(crate) struct IssueGatewayCreateResult {
    /// API result payload or domain error returned by the gateway client.
    pub(crate) result: Result<
        crate::issue_gateway::api::CreateIssueResponse,
        crate::issue_gateway::api::CreateIssueError,
    >,
}

/// Outcome of polling the gateway for an authenticated issue token.
#[derive(Debug)]
pub(crate) struct IssueGatewayAuthResult {
    /// Auth token when polling succeeds, or the terminal polling error.
    pub(crate) result: Result<String, crate::issue_gateway::api::IssueAuthError>,
}

/// Request to save a GitHub issue token to persistent storage.
#[derive(Debug)]
pub(crate) struct IssueTokenSaveJob {
    /// Token value to persist.
    pub(crate) token: String,
    /// Whether the token modal should reopen after save completion.
    pub(crate) reopen_modal: bool,
}

/// Result from attempting to load a GitHub issue token.
#[derive(Debug)]
pub(crate) struct IssueTokenLoadResult {
    pub(crate) result: Result<Option<String>, crate::issue_gateway::IssueTokenStoreError>,
}

/// Result from attempting to save a GitHub issue token.
#[derive(Debug)]
pub(crate) struct IssueTokenSaveResult {
    pub(crate) token: String,
    pub(crate) reopen_modal: bool,
    pub(crate) result: Result<(), crate::issue_gateway::IssueTokenStoreError>,
}

/// Result from attempting to delete a GitHub issue token.
#[derive(Debug)]
pub(crate) struct IssueTokenDeleteResult {
    pub(crate) result: Result<(), crate::issue_gateway::IssueTokenStoreError>,
}
