use std::fmt;
use std::sync::Arc;
use std::time::Duration;

use anyhow::{Result, anyhow};
use async_trait::async_trait;
use chrono::{DateTime, Utc};
use thiserror::Error;
use warp_graphql::mutations::expire_api_key::ExpireApiKeyResult;
use warp_graphql::mutations::generate_api_key::GenerateApiKeyResult;
use warp_graphql::queries::api_keys::ApiKeyProperties;
use warp_graphql::queries::get_user::UserOutput;

use crate::auth::credentials::{Credentials, LoginToken};
use warp_graphql::mutations::create_anonymous_user::AnonymousUserType;
use crate::server::ids::ApiKeyUid;
use warp_graphql::mutations::update_user_settings::UpdateUserSettingsInput;

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
    pub is_telemetry_enabled: bool,
}

#[derive(Debug)]
pub struct FetchUserResult {
    pub user_output: UserOutput,
    pub credentials: Credentials,
    pub from_refresh: bool,
}

#[derive(Error, Debug)]
pub enum UserAuthenticationError {
    #[error("Failed to authenticate user")]
    AuthenticationFailed,
    #[error("Network error during authentication")]
    NetworkError,
    #[error("Invalid state parameter")]
    InvalidStateParameter,
    #[error("Missing state parameter")]
    MissingStateParameter,
    #[error("Access token denied: {0}")]
    DeniedAccessToken(String),
    #[error("User account disabled: {0}")]
    UserAccountDisabled(String),
    #[error("Unexpected error: {0}")]
    Unexpected(String),
}

impl UserAuthenticationError {
    pub fn is_actionable(&self) -> bool {
        matches!(
            self,
            Self::DeniedAccessToken(_) | Self::UserAccountDisabled(_) | Self::Unexpected(_)
        )
    }
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
    ) -> Result<crate::auth::credentials::AuthToken>;
    async fn list_api_keys(&self) -> Result<Vec<ApiKeyProperties>>;
    async fn create_api_key(
        &self,
        name: String,
        team_id: Option<cynic::Id>,
        agent_uid: Option<cynic::Id>,
        expires_at: Option<warp_graphql::scalars::Time>,
    ) -> Result<GenerateApiKeyResult>;
    async fn expire_api_key(&self, uid: &ApiKeyUid) -> Result<ExpireApiKeyResult>;
    async fn fetch_user(
        &self,
        token: LoginToken,
        for_refresh: bool,
    ) -> std::result::Result<FetchUserResult, UserAuthenticationError>;
    async fn request_device_code(
        &self,
    ) -> std::result::Result<oauth2::StandardDeviceAuthorizationResponse, UserAuthenticationError>;
    async fn exchange_device_access_token(
        &self,
        details: &oauth2::StandardDeviceAuthorizationResponse,
        timeout: Duration,
    ) -> std::result::Result<crate::auth::credentials::FirebaseToken, UserAuthenticationError>;
    async fn create_anonymous_user(
        &self,
        referral_code: Option<String>,
        anonymous_user_type: AnonymousUserType,
    ) -> Result<warp_graphql::mutations::create_anonymous_user::CreateAnonymousUserResult>;
    async fn fetch_new_custom_token(
        &self,
    ) -> std::result::Result<String, MintCustomTokenError>;
    fn on_custom_token_fetched(
        &self,
        response: std::result::Result<String, MintCustomTokenError>,
    ) -> std::result::Result<String, MintCustomTokenError>;
    async fn get_user_settings(&self) -> Result<Option<SyncedUserSettings>>;
    async fn set_user_is_onboarded(&self) -> Result<()>;
    async fn set_is_crash_reporting_enabled(&self, enabled: bool) -> Result<()>;
    async fn set_is_telemetry_enabled(&self, enabled: bool) -> Result<()>;
    async fn set_is_cloud_conversation_storage_enabled(&self, enabled: bool) -> Result<()>;
    async fn update_user_settings(&self, input: UpdateUserSettingsInput) -> Result<()>;
}
#[cfg(any(test, feature = "test-util"))]
mockall::mock! {
    pub AuthClient {}

    #[async_trait]
    impl AuthClient for AuthClient {
        async fn list_agent_identities(&self) -> Result<Vec<AgentIdentity>>;
        async fn get_or_refresh_access_token(&self) -> Result<crate::auth::credentials::AuthToken>;
        async fn list_api_keys(&self) -> Result<Vec<ApiKeyProperties>>;
        async fn create_api_key(
            &self,
            name: String,
            team_id: Option<cynic::Id>,
            agent_uid: Option<cynic::Id>,
            expires_at: Option<warp_graphql::scalars::Time>,
        ) -> Result<GenerateApiKeyResult>;
        async fn expire_api_key(&self, uid: &ApiKeyUid) -> Result<ExpireApiKeyResult>;
        async fn fetch_user(
            &self,
            token: LoginToken,
            for_refresh: bool,
        ) -> std::result::Result<FetchUserResult, UserAuthenticationError>;
        async fn request_device_code(
            &self,
        ) -> std::result::Result<oauth2::StandardDeviceAuthorizationResponse, UserAuthenticationError>;
        async fn exchange_device_access_token(
            &self,
            details: &oauth2::StandardDeviceAuthorizationResponse,
            timeout: Duration,
        ) -> std::result::Result<crate::auth::credentials::FirebaseToken, UserAuthenticationError>;
        async fn create_anonymous_user(
            &self,
            referral_code: Option<String>,
            anonymous_user_type: AnonymousUserType,
        ) -> Result<warp_graphql::mutations::create_anonymous_user::CreateAnonymousUserResult>;
        async fn fetch_new_custom_token(
            &self,
        ) -> std::result::Result<String, MintCustomTokenError>;
        fn on_custom_token_fetched(
            &self,
            response: std::result::Result<String, MintCustomTokenError>,
        ) -> std::result::Result<String, MintCustomTokenError>;
        async fn get_user_settings(&self) -> Result<Option<SyncedUserSettings>>;
        async fn set_user_is_onboarded(&self) -> Result<()>;
        async fn set_is_crash_reporting_enabled(&self, enabled: bool) -> Result<()>;
        async fn set_is_telemetry_enabled(&self, enabled: bool) -> Result<()>;
        async fn set_is_cloud_conversation_storage_enabled(&self, enabled: bool) -> Result<()>;
        async fn update_user_settings(&self, input: UpdateUserSettingsInput) -> Result<()>;
    }
}

pub struct AuthClientImpl;

impl AuthClientImpl {
    pub fn new(
        _server_api: Arc<super::ServerApi>,
        _auth_session: Arc<AuthSession>,
    ) -> Self {
        Self
    }
}
#[cfg_attr(not(target_family = "wasm"), async_trait)]
#[cfg_attr(target_family = "wasm", async_trait(?Send))]
impl AuthClient for AuthClientImpl {
    async fn list_agent_identities(&self) -> Result<Vec<AgentIdentity>> {
        Ok(vec![])
    }
    async fn get_or_refresh_access_token(&self) -> Result<crate::auth::credentials::AuthToken> {
        Ok(crate::auth::credentials::AuthToken::NoAuth)
    }
    async fn list_api_keys(&self) -> Result<Vec<ApiKeyProperties>> {
        Err(anyhow!("API keys require cloud backend"))
    }
    async fn create_api_key(
        &self,
        _name: String,
        _team_id: Option<cynic::Id>,
        _agent_uid: Option<cynic::Id>,
        _expires_at: Option<warp_graphql::scalars::Time>,
    ) -> Result<GenerateApiKeyResult> {
        Err(anyhow!("API keys require cloud backend"))
    }
    async fn expire_api_key(&self, _uid: &ApiKeyUid) -> Result<ExpireApiKeyResult> {
        Err(anyhow!("API keys require cloud backend"))
    }
    async fn fetch_user(
        &self,
        _token: LoginToken,
        _for_refresh: bool,
    ) -> std::result::Result<FetchUserResult, UserAuthenticationError> {
        Err(UserAuthenticationError::AuthenticationFailed)
    }
    async fn request_device_code(
        &self,
    ) -> std::result::Result<oauth2::StandardDeviceAuthorizationResponse, UserAuthenticationError> {
        Err(UserAuthenticationError::AuthenticationFailed)
    }
    async fn exchange_device_access_token(
        &self,
        _details: &oauth2::StandardDeviceAuthorizationResponse,
        _timeout: Duration,
    ) -> std::result::Result<crate::auth::credentials::FirebaseToken, UserAuthenticationError> {
        Err(UserAuthenticationError::AuthenticationFailed)
    }
    async fn create_anonymous_user(
        &self,
        _referral_code: Option<String>,
        _anonymous_user_type: AnonymousUserType,
    ) -> Result<warp_graphql::mutations::create_anonymous_user::CreateAnonymousUserResult> {
        Err(anyhow!("Anonymous user creation requires cloud backend"))
    }
    async fn fetch_new_custom_token(
        &self,
    ) -> std::result::Result<String, MintCustomTokenError> {
        Err(MintCustomTokenError::Failed)
    }
    fn on_custom_token_fetched(
        &self,
        response: std::result::Result<String, MintCustomTokenError>,
    ) -> std::result::Result<String, MintCustomTokenError> {
        response
    }
    async fn get_user_settings(&self) -> Result<Option<SyncedUserSettings>> {
        Ok(None)
    }
    async fn set_user_is_onboarded(&self) -> Result<()> {
        Ok(())
    }
    async fn set_is_crash_reporting_enabled(&self, _enabled: bool) -> Result<()> {
        Ok(())
    }
    async fn set_is_telemetry_enabled(&self, _enabled: bool) -> Result<()> {
        Ok(())
    }
    async fn set_is_cloud_conversation_storage_enabled(&self, _enabled: bool) -> Result<()> {
        Ok(())
    }
    async fn update_user_settings(&self, _input: UpdateUserSettingsInput) -> Result<()> {
        Ok(())
    }
}
/// Reusable authentication-session mechanics.
pub struct AuthSession {
    pub(crate) event_sender: async_channel::Sender<AuthEvent>,
}

impl AuthSession {
    pub fn new(
        _client: Arc<http_client::Client>,
        _auth_state: Arc<crate::auth::auth_state::AuthState>,
        event_sender: async_channel::Sender<AuthEvent>,
    ) -> Self {
        Self { event_sender }
    }

    pub fn allowed_to_refresh_token(&self) -> bool {
        false
    }

    pub async fn get_or_refresh_access_token(
        &self,
    ) -> Result<crate::auth::credentials::AuthToken> {
        Ok(crate::auth::credentials::AuthToken::NoAuth)
    }
}

#[derive(Error, Debug)]
/// Error type when creating anonymous users.
pub enum AnonymousUserCreationError {
    #[error("The network request to create the anonymous user failed")]
    CreationFailed,

    #[error("Received a user facing error: {0}")]
    UserFacingError(String),

    #[error("The user was created, but the ID token could not be fetched")]
    UserAuthenticationFailed(#[from] UserAuthenticationError),

    #[error("Failed to create anonymous user with unknown error")]
    Unknown,
}

#[cfg(test)]
#[path = "auth_tests.rs"]
mod tests;