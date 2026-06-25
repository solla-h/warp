use warp_graphql::object_permissions::OwnerType;

use super::user::FirebaseAuthTokens;

#[derive(Clone, Debug)]
pub enum Credentials {
    Firebase(FirebaseAuthTokens),
    ApiKey {
        key: String,
        owner_type: Option<OwnerType>,
    },
    Bearer(String),
    SessionCookie,
    #[cfg(any(test, feature = "integration_tests", feature = "skip_login"))]
    Test,
}

impl Credentials {
    pub fn as_firebase(&self) -> Option<&FirebaseAuthTokens> {
        match self {
            Credentials::Firebase(tokens) => Some(tokens),
            Credentials::ApiKey { .. } => None,
            Credentials::Bearer(_) => None,
            Credentials::SessionCookie => None,
            #[cfg(any(test, feature = "integration_tests", feature = "skip_login"))]
            Credentials::Test => None,
        }
    }

    pub fn as_api_key(&self) -> Option<&str> {
        match self {
            Credentials::ApiKey { key, .. } => Some(key),
            Credentials::Firebase(_) => None,
            Credentials::Bearer(_) => None,
            Credentials::SessionCookie => None,
            #[cfg(any(test, feature = "integration_tests", feature = "skip_login"))]
            Credentials::Test => None,
        }
    }

    pub fn api_key_owner_type(&self) -> Option<OwnerType> {
        match self {
            Credentials::ApiKey { owner_type, .. } => *owner_type,
            Credentials::Firebase(_) => None,
            Credentials::Bearer(_) => None,
            Credentials::SessionCookie => None,
            #[cfg(any(test, feature = "integration_tests", feature = "skip_login"))]
            Credentials::Test => None,
        }
    }

    pub fn refresh_token(&self) -> Option<&str> {
        match self {
            Credentials::Firebase(tokens) => Some(&tokens.refresh_token),
            Credentials::ApiKey { .. } => None,
            Credentials::Bearer(_) => None,
            Credentials::SessionCookie => None,
            #[cfg(any(test, feature = "integration_tests", feature = "skip_login"))]
            Credentials::Test => None,
        }
    }

    pub fn bearer_token(&self) -> AuthToken {
        match self {
            Credentials::Firebase(tokens) => AuthToken::Firebase(tokens.id_token.clone()),
            Credentials::ApiKey { key, .. } => AuthToken::ApiKey(key.clone()),
            Credentials::Bearer(token) => AuthToken::Bearer(token.clone()),
            Credentials::SessionCookie => AuthToken::NoAuth,
            #[cfg(any(test, feature = "integration_tests", feature = "skip_login"))]
            Credentials::Test => AuthToken::NoAuth,
        }
    }

    pub fn is_externally_managed(&self) -> bool {
        match self {
            Credentials::Bearer(_) => true,
            Credentials::Firebase(_) | Credentials::ApiKey { .. } | Credentials::SessionCookie => {
                false
            }
            #[cfg(any(test, feature = "integration_tests", feature = "skip_login"))]
            Credentials::Test => false,
        }
    }

    pub fn login_token(&self) -> Option<LoginToken> {
        match self {
            Credentials::Firebase(tokens) => Some(LoginToken::Firebase(FirebaseToken::Refresh(
                RefreshToken::new(&tokens.refresh_token),
            ))),
            Credentials::ApiKey { key, .. } => Some(LoginToken::ApiKey(key.clone())),
            Credentials::Bearer(_) => None,
            Credentials::SessionCookie => Some(LoginToken::SessionCookie),
            #[cfg(any(test, feature = "integration_tests", feature = "skip_login"))]
            Credentials::Test => None,
        }
    }
}

#[derive(Debug, Clone)]
pub enum AuthToken {
    Firebase(String),
    ApiKey(String),
    Bearer(String),
    #[cfg_attr(
        not(any(
            test,
            feature = "integration_tests",
            feature = "skip_login",
            feature = "test-util"
        )),
        allow(dead_code)
    )]
    NoAuth,
}

impl AuthToken {
    pub fn as_bearer_token(&self) -> Option<&str> {
        match self {
            AuthToken::Firebase(token) => Some(token),
            AuthToken::ApiKey(key) => Some(key),
            AuthToken::Bearer(token) => Some(token),
            AuthToken::NoAuth => None,
        }
    }

    pub fn bearer_token(&self) -> Option<String> {
        match self {
            AuthToken::Firebase(token) => Some(token.clone()),
            AuthToken::ApiKey(key) => Some(key.clone()),
            AuthToken::Bearer(token) => Some(token.clone()),
            AuthToken::NoAuth => None,
        }
    }
}

#[derive(Debug)]
pub enum LoginToken {
    Firebase(FirebaseToken),
    ApiKey(String),
    SessionCookie,
}

#[derive(Debug)]
pub enum FirebaseToken {
    Refresh(RefreshToken),
    Custom(String),
}

impl FirebaseToken {
    pub fn access_token_url(&self, api_key: &str) -> String {
        match self {
            FirebaseToken::Refresh(_) => {
                format!("https://securetoken.googleapis.com/v1/token?key={api_key}")
            }
            FirebaseToken::Custom(_) => {
                format!(
                    "https://identitytoolkit.googleapis.com/v1/accounts:signInWithCustomToken?key={api_key}"
                )
            }
        }
    }

    pub fn access_token_request_body(&self) -> Vec<(&str, &str)> {
        match self {
            FirebaseToken::Refresh(refresh_token) => vec![
                ("grant_type", "refresh_token"),
                ("refresh_token", refresh_token.get()),
            ],
            FirebaseToken::Custom(custom_token) => {
                vec![("returnSecureToken", "true"), ("token", custom_token)]
            }
        }
    }

    pub fn proxy_url(&self, server_root: &str, api_key: &str) -> String {
        match self {
            FirebaseToken::Refresh(_) => format!("{server_root}/proxy/token?key={api_key}"),
            FirebaseToken::Custom(_) => {
                format!("{server_root}/proxy/customToken?key={api_key}")
            }
        }
    }
}

#[derive(Debug, Clone)]
pub struct RefreshToken(String);

impl RefreshToken {
    pub fn new(token: impl Into<String>) -> Self {
        Self(token.into())
    }

    pub fn get(&self) -> &str {
        self.0.as_str()
    }
}
