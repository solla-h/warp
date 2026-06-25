use std::sync::Arc;

use anyhow::Result;
use async_trait::async_trait;
use instant::Duration;

use crate::auth::AgentIdentity;

/// Application-provided transport and platform capabilities used by API clients.
#[cfg_attr(not(target_family = "wasm"), async_trait)]
#[cfg_attr(target_family = "wasm", async_trait(?Send))]
pub trait BaseClient: Send + Sync {
    /// Returns the HTTP transport used to send server API requests.
    ///
    /// API clients should use this client rather than constructing their own
    /// transport so application-level HTTP setup remains shared.
    fn http_client(&self) -> Arc<http_client::Client>;

    fn anonymous_id(&self) -> String;

    /// Lists public agent identities available to API-key creation flows.
    ///
    /// This is a base capability until its public REST endpoint is extracted alongside
    /// the GraphQL-backed API client methods that consume its result.
    async fn list_agent_identities(&self) -> Result<Vec<AgentIdentity>>;

    /// Returns an ambient workload token when the current runtime supports issuing one.
    ///
    /// Extracted clients surface this for ambient-agent authentication while leaving
    /// workload-token caching and platform integration in the application.
    async fn get_or_create_ambient_workload_token(&self) -> Result<Option<String>>;

    /// Returns whether authentication failures may be handled as refreshable user-session failures.
    ///
    /// The shared GraphQL request helper uses this distinction to avoid emitting
    /// user-session events for externally managed credentials.
    fn is_auth_refresh_allowed(&self) -> bool;

    /// Notifies the application that a GraphQL request was blocked by staging access controls.
    ///
    /// The shared request helper calls this hook instead of depending on application event types.
    fn on_graphql_staging_access_blocked(&self);

    /// Notifies the application that a GraphQL request received an IAP challenge.
    ///
    /// The shared request helper calls this hook so the application can refresh
    /// its IAP state without exposing that state to extracted clients.
    fn on_graphql_iap_challenge_received(&self);

    /// Notifies the application that a GraphQL response indicates a disabled user account.
    ///
    /// The shared request helper only invokes this for refreshable user sessions;
    /// callers using externally managed credentials receive an authentication error instead.
    fn on_graphql_user_account_disabled(&self);
}
