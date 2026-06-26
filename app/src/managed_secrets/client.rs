use std::collections::HashMap;
use std::time::Duration;

use anyhow::Result;
use async_trait::async_trait;
use chrono::{DateTime, Utc};
use vec1::Vec1;
use warp_cli::agent::Harness;

use crate::ai::auth_secret_types::ManagedSecretType;

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
    pub user_public_key: Option<String>,
    pub team_public_keys: HashMap<String, String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum SecretOwner {
    CurrentUser,
    Team { team_uid: String },
}

/// Metadata returned after creating or updating a managed secret.
#[derive(Debug, Clone)]
pub struct SecretMetadata {
    pub name: String,
    pub owner: SecretOwner,
}

/// An entry in the list of managed secrets.
#[derive(Debug, Clone)]
pub struct SecretListEntry {
    pub name: String,
    pub owner: SecretOwner,
    pub secret_type: String,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
}

/// An entry representing an auth secret bound to a harness.
#[derive(Debug, Clone)]
pub struct AuthSecretEntry {
    pub name: String,
    pub owner: SecretOwner,
}

#[cfg_attr(not(target_family = "wasm"), async_trait)]
#[cfg_attr(target_family = "wasm", async_trait(?Send))]
pub trait ManagedSecretsClient: 'static + Send + Sync {
    async fn get_managed_secret_configs(&self) -> Result<ManagedSecretConfigs>;

    async fn delete_managed_secret(&self, owner: SecretOwner, name: String) -> Result<()>;

    /// Issue a short-lived OIDC identity token for the current task.
    ///
    /// The workload token is not passed explicitly - it's automatically provided
    /// as part of the client's cloud agent workload identity token support
    /// (see the `ServerApi` implementation).
    async fn issue_task_identity_token(
        &self,
        options: IdentityTokenOptions,
    ) -> Result<TaskIdentityToken>;

    async fn list_harness_auth_secrets(
        &self,
        _harness: Harness,
    ) -> Result<Vec<AuthSecretEntry>> {
        todo!("GraphQL backend removed")
    }
}
