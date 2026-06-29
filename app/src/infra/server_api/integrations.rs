use anyhow::Result;
use async_trait::async_trait;
#[cfg(test)]
use mockall::automock;

use warp_types::ServerTimestamp;

#[cfg(not(target_family = "wasm"))]
pub trait IntegrationsClientBounds: Send + Sync {}

#[cfg(not(target_family = "wasm"))]
impl<T: 'static + Send + Sync> IntegrationsClientBounds for T {}

#[cfg(target_family = "wasm")]
pub trait IntegrationsClientBounds {}

#[cfg(target_family = "wasm")]
impl<T: 'static> IntegrationsClientBounds for T {}

// Types previously defined in warp_graphql, now local to this module.

#[derive(Debug, Clone)]
pub struct UserRepoAuthStatusOutput {
    pub statuses: Vec<RepoAuthResult>,
    pub auth_url: Option<String>,
    pub tx_id: Option<String>,
}

#[derive(Debug, Clone)]
pub struct RepoAuthResult {
    pub owner: String,
    pub repo: String,
    pub status: UserRepoAuthStatusEnum,
    pub is_public: bool,
}

#[derive(Debug, Clone, Copy)]
pub enum UserRepoAuthStatusEnum {
    NoInstallationOrAccessForRepo,
    UserNotConnectedToGithub,
    Success,
}

#[derive(Debug, Clone)]
pub struct CreateSimpleIntegrationOutput {
    pub auth_url: Option<String>,
    pub success: bool,
    pub message: String,
    pub tx_id: Option<String>,
}

#[derive(Debug, Clone)]
pub struct SimpleIntegrationsOutput {
    pub integrations: Vec<SimpleIntegration>,
    pub message: Option<String>,
}

#[derive(Debug, Clone)]
pub struct SimpleIntegration {
    pub provider_slug: String,
    pub description: String,
    pub connection_status: SimpleIntegrationConnectionStatus,
    pub integration_config: Option<ListedSimpleIntegrationConfig>,
    pub created_at: Option<ServerTimestamp>,
    pub updated_at: Option<ServerTimestamp>,
}

#[derive(Debug, Clone)]
pub struct ListedSimpleIntegrationConfig {
    pub environment_uid: String,
    pub base_prompt: String,
    pub model_id: String,
    pub mcp_servers_json: String,
}

#[derive(Debug, Clone, Copy)]
pub enum SimpleIntegrationConnectionStatus {
    NotConnected,
    ConnectionError,
    IntegrationNotConfigured,
    NotEnabled,
    Active,
}

#[derive(Debug, Clone, Copy)]
pub enum OauthConnectTxStatus {
    Completed,
    Expired,
    Failed,
    InProgress,
    Pending,
}

#[derive(Debug, Clone)]
pub struct GetIntegrationsUsingEnvironmentOutput {
    pub provider_names: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct GithubConnectedOutput {
    pub username: Option<String>,
    pub installed_repos: Vec<GithubRepoResult>,
    pub app_install_link: String,
}

#[derive(Debug, Clone)]
pub struct GithubAuthRequiredOutput {
    pub auth_url: String,
    pub tx_id: String,
    pub app_install_link: String,
}

#[derive(Debug, Clone)]
pub struct GithubRepoResult {
    pub owner: String,
    pub repo: String,
    pub is_public: bool,
}

#[derive(Debug, Clone)]
pub enum UserGithubInfoResult {
    GithubConnectedOutput(GithubConnectedOutput),
    GithubAuthRequiredOutput(GithubAuthRequiredOutput),
    Unknown,
}

#[derive(Debug, Clone)]
pub struct SuggestCloudEnvironmentImageOutput {
    pub detected_languages: Vec<GithubReposLanguageStat>,
    pub image: String,
    pub needs_custom_image: bool,
    pub reason: String,
}

#[derive(Debug, Clone)]
pub struct SuggestCloudEnvironmentImageAuthRequiredOutput {
    pub auth_url: String,
    pub tx_id: String,
}

#[derive(Debug, Clone)]
pub struct GithubReposLanguageStat {
    pub bytes: i32,
    pub language: String,
    pub percentage: f64,
}

#[derive(Debug, Clone)]
pub enum SuggestCloudEnvironmentImageResult {
    SuggestCloudEnvironmentImageAuthRequiredOutput(SuggestCloudEnvironmentImageAuthRequiredOutput),
    SuggestCloudEnvironmentImageOutput(SuggestCloudEnvironmentImageOutput),
    UserFacingError(String),
    Unknown,
}

#[cfg_attr(test, automock)]
#[cfg_attr(target_family = "wasm", allow(dead_code))]
#[cfg_attr(target_family = "wasm", async_trait(?Send))]
#[cfg_attr(not(target_family = "wasm"), async_trait)]
pub trait IntegrationsClient: 'static + IntegrationsClientBounds {
    /// Checks the user's GitHub authorization status for the given repositories.
    async fn check_user_repo_auth_status(
        &self,
        repos: Vec<(String, String)>,
    ) -> Result<UserRepoAuthStatusOutput>;

    /// Creates or updates a simple integration on the server.
    #[allow(clippy::too_many_arguments)]
    async fn create_or_update_simple_integration(
        &self,
        integration_type: String,
        is_update: bool,
        environment_uid: Option<String>,
        base_prompt: Option<String>,
        model_id: Option<String>,
        mcp_servers_json: Option<String>,
        remove_mcp_server_names: Option<Vec<String>>,
        worker_host: Option<String>,
        enabled: bool,
    ) -> Result<CreateSimpleIntegrationOutput>;

    /// Lists simple integrations for a fixed set of provider slugs.
    async fn list_simple_integrations(
        &self,
        providers: Vec<String>,
    ) -> Result<SimpleIntegrationsOutput>;

    /// Polls the status of an OAuth connect transaction.
    async fn poll_oauth_connect_status(&self, tx_id: String) -> Result<OauthConnectTxStatus>;

    /// Gets the list of integration provider names that are using the specified environment.
    async fn get_integrations_using_environment(
        &self,
        environment_id: String,
    ) -> Result<GetIntegrationsUsingEnvironmentOutput>;

    /// Gets the user's GitHub connection info, including accessible repos.
    async fn get_user_github_info(&self) -> Result<UserGithubInfoResult>;

    /// Suggests a Docker image for a cloud environment based on the provided repos.
    async fn suggest_cloud_environment_image(
        &self,
        repos: Vec<(String, String)>,
    ) -> Result<SuggestCloudEnvironmentImageResult>;
}

use super::ServerApi;

#[cfg_attr(not(target_family = "wasm"), async_trait)]
#[cfg_attr(target_family = "wasm", async_trait(?Send))]
impl IntegrationsClient for ServerApi {
    async fn check_user_repo_auth_status(
        &self,
        _repos: Vec<(String, String)>,
    ) -> Result<UserRepoAuthStatusOutput> {
        Err(anyhow::anyhow!("cloud backend removed"))
    }

    async fn create_or_update_simple_integration(
        &self,
        _integration_type: String,
        _is_update: bool,
        _environment_uid: Option<String>,
        _base_prompt: Option<String>,
        _model_id: Option<String>,
        _mcp_servers_json: Option<String>,
        _remove_mcp_server_names: Option<Vec<String>>,
        _worker_host: Option<String>,
        _enabled: bool,
    ) -> Result<CreateSimpleIntegrationOutput> {
        Err(anyhow::anyhow!("cloud backend removed"))
    }

    async fn list_simple_integrations(
        &self,
        _providers: Vec<String>,
    ) -> Result<SimpleIntegrationsOutput> {
        Err(anyhow::anyhow!("cloud backend removed"))
    }

    async fn poll_oauth_connect_status(&self, _tx_id: String) -> Result<OauthConnectTxStatus> {
        Err(anyhow::anyhow!("cloud backend removed"))
    }

    async fn get_integrations_using_environment(
        &self,
        _environment_id: String,
    ) -> Result<GetIntegrationsUsingEnvironmentOutput> {
        Err(anyhow::anyhow!("cloud backend removed"))
    }

    async fn get_user_github_info(&self) -> Result<UserGithubInfoResult> {
        Err(anyhow::anyhow!("cloud backend removed"))
    }

    async fn suggest_cloud_environment_image(
        &self,
        _repos: Vec<(String, String)>,
    ) -> Result<SuggestCloudEnvironmentImageResult> {
        Err(anyhow::anyhow!("cloud backend removed"))
    }
}

