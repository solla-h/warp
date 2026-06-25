use std::fmt;
use std::sync::Arc;

use anyhow::Result;
use async_trait::async_trait;
use thiserror::Error;

/// Authentication and authenticated-transport conditions observed by shared client code.
#[derive(Clone)]
pub enum AuthEvent {
    StagingAccessBlocked,
    NeedsReauth,
    UserAccountDisabled,
    AccessTokenRefreshed {
        #[cfg_attr(target_family = "wasm", allow(dead_code))]
        token: String,
    },
    IapChallengeReceived,
}

impl fmt::Debug for AuthEvent {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::StagingAccessBlocked => f.write_str("StagingAccessBlocked"),
            Self::NeedsReauth => f.write_str("NeedsReauth"),
            Self::UserAccountDisabled => f.write_str("UserAccountDisabled"),
            Self::AccessTokenRefreshed { .. } => f
                .debug_struct("AccessTokenRefreshed")
                .field("token", &"<redacted>")
                .finish(),
            Self::IapChallengeReceived => f.write_str("IapChallengeReceived"),
        }
    }
}

/// Header key used to associate unauthenticated requests with an experiment identity.
pub const EXPERIMENT_ID_HEADER: &str = "X-Warp-Experiment-Id";

/// A named agent identity from the public API.
#[derive(Debug, Clone, serde::Deserialize)]
pub struct AgentIdentity {
    pub uid: String,
    pub name: String,
    pub available: bool,
}

/// User settings that are stored server-side on a per-user basis.
#[derive(Copy, Clone, Debug, Default)]
pub struct SyncedUserSettings {
    pub is_cloud_conversation_storage_enabled: bool,
    pub is_crash_reporting_enabled: bool,
}

#[derive(Debug, Clone)]
pub struct FetchUserResult {
    pub email: Option<String>,
    pub name: Option<String>,
}

#[derive(Error, Debug)]
pub enum UserAuthenticationError {
    #[error("Failed to authenticate user")]
    AuthenticationFailed,
    #[error("Network error during authentication")]
    NetworkError,
}

#[derive(Error, Debug)]
pub enum MintCustomTokenError {
    #[error("Failed to mint custom token")]
    Failed,
}

/// Trait for server authentication operations.
#[cfg_attr(not(target_family = "wasm"), async_trait)]
#[cfg_attr(target_family = "wasm", async_trait(?Send))]
pub trait AuthClient: Send + Sync {
    async fn list_agent_identities(&self) -> Result<Vec<AgentIdentity>>;
    async fn get_or_refresh_access_token(
        &self,
    ) -> Result<warp_server_auth::credentials::AuthToken>;
}

#[cfg(any(test, feature = "test-util"))]
mockall::mock! {
    pub AuthClient {}

    #[async_trait]
    impl AuthClient for AuthClient {
        async fn list_agent_identities(&self) -> Result<Vec<AgentIdentity>>;
    }
}

/// Placeholder for AuthClientImpl — cloud auth operations removed with warp_server_client.
pub struct AuthClientImpl;

/// Reusable authentication-session mechanics.
pub struct AuthSession {
    pub(crate) event_sender: async_channel::Sender<AuthEvent>,
}

impl AuthSession {
    pub fn new(
        _client: Arc<http_client::Client>,
        _auth_state: Arc<warp_server_auth::auth_state::AuthState>,
        event_sender: async_channel::Sender<AuthEvent>,
    ) -> Self {
        Self { event_sender }
    }

    pub fn allowed_to_refresh_token(&self) -> bool {
        false
    }

    pub async fn get_or_refresh_access_token(
        &self,
    ) -> Result<warp_server_auth::credentials::AuthToken> {
        Ok(warp_server_auth::credentials::AuthToken::NoAuth)
    }
}

#[derive(Error, Debug)]
/// Error type when creating anonymous users.
pub enum AnonymousUserCreationError {
    #[error("The network request to create the anonymous user failed")]
    CreationFailed,

    #[error("Received a user facing error: {0}")]
    UserFacingError(String),

    /// Failure that occurs after the user is created, but the ID token could not be fetched.
    #[error("The user was created, but the ID token could not be fetched")]
    UserAuthenticationFailed(#[from] UserAuthenticationError),

    #[error("Failed to create anonymous user with unknown error")]
    Unknown,
}

#[cfg(test)]
#[path = "auth_tests.rs"]
mod tests;
