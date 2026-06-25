mod session;

pub use session::*;
pub use user_uid::{TEST_USER_EMAIL, TEST_USER_UID, UserUid};
use warp_server_auth::credentials::{AuthToken, Credentials, FirebaseToken, LoginToken};
pub use warp_server_auth::user_uid;

/// Header key used to associate unauthenticated requests with an experiment identity.
pub const EXPERIMENT_ID_HEADER: &str = "X-Warp-Experiment-Id";

/// Errors that can occur during user authentication flows.
#[derive(Debug, thiserror::Error)]
pub enum UserAuthenticationError {
    #[error("access token denied: {0}")]
    DeniedAccessToken(#[source] firebase::FirebaseError),
    #[error("invalid state parameter")]
    InvalidStateParameter,
    #[error("missing state parameter")]
    MissingStateParameter,
    #[error("user account disabled: {0}")]
    UserAccountDisabled(String),
    #[error("{0}")]
    Unexpected(#[from] anyhow::Error),
}

impl UserAuthenticationError {
    pub fn is_actionable(&self) -> bool {
        matches!(
            self,
            Self::InvalidStateParameter | Self::MissingStateParameter
        )
    }
}

impl From<firebase::FirebaseError> for UserAuthenticationError {
    fn from(err: firebase::FirebaseError) -> Self {
        Self::DeniedAccessToken(err)
    }
}

/// A named agent identity from the public API.
#[derive(Clone, Debug, serde::Deserialize)]
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
