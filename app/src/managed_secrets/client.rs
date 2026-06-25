use std::collections::HashMap;
use std::time::Duration;

use anyhow::Result;
use async_trait::async_trait;
use chrono::{DateTime, Utc};
use vec1::Vec1;
use warp_graphql::managed_secrets::{ManagedSecret, ManagedSecretConfig, ManagedSecretType};
use warp_graphql::queries::task_secrets::ManagedSecretValue as GqlManagedSecretValue;

#[derive(Debug, Clone)]
pub struct TaskIdentityToken {
    pub token: String,
    pub expires_at: DateTime<Utc>,
    pub issuer: String,
}

pub struct IdentityTokenOptions {
    pub audience: String,
    pub requested_duration: Duration,
    pub subject_template: Vec1<String>,
}

#[derive(Debug)]
pub struct ManagedSecretConfigs {
    pub user_secrets: Option<ManagedSecretConfig>,
    pub team_secrets: HashMap<String, ManagedSecretConfig>,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum SecretOwner {
    CurrentUser,
    Team { team_uid: String },
}

#[cfg_attr(not(target_family = "wasm"), async_trait)]
#[cfg_attr(target_family = "wasm", async_trait(?Send))]
pub trait ManagedSecretsClient: 'static + Send + Sync {
    async fn get_managed_secret_configs(&self) -> Result<ManagedSecretConfigs>;

    async fn create_managed_secret(
        &self,
        owner: SecretOwner,
        name: String,
        secret_type: ManagedSecretType,
        encrypted_value: String,
        description: Option<String>,
    ) -> Result<ManagedSecret>;

    async fn update_managed_secret(
        &self,
        owner: SecretOwner,
        name: String,
        encrypted_value: Option<String>,
        description: Option<String>,
    ) -> Result<ManagedSecret>;

    async fn delete_managed_secret(&self, owner: SecretOwner, name: String) -> Result<()>;

    async fn list_secrets(&self) -> Result<Vec<ManagedSecret>>;

    async fn list_harness_auth_secrets(
        &self,
        harness: warp_graphql::ai::AgentHarness,
    ) -> Result<Vec<ManagedSecret>>;

    async fn get_task_secrets(
        &self,
        task_id: String,
        workload_token: String,
    ) -> Result<HashMap<String, GqlManagedSecretValue>>;

    async fn issue_task_identity_token(
        &self,
        options: IdentityTokenOptions,
    ) -> Result<TaskIdentityToken>;
}
