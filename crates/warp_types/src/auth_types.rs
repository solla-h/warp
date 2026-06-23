use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum AnonymousUserType {
    NativeClientAnonymousUser,
    NativeClientAnonymousUserFeatureGated,
    WebClientAnonymousUser,
}

/// Type of principal making the authenticated request.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum PrincipalType {
    #[default]
    User,
    ServiceAccount,
}

#[derive(Debug, Copy, Clone, Serialize, Deserialize)]
pub struct PersonalObjectLimits {
    pub env_var_limit: usize,
    pub notebook_limit: usize,
    pub workflow_limit: usize,
}

/// Metadata about a user (email, display name, photo).
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct UserMetadata {
    pub email: String,
    pub display_name: Option<String>,
    pub photo_url: Option<String>,
}
