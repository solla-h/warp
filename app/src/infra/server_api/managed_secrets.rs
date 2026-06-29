use anyhow::Result;
use async_trait::async_trait;
pub use crate::managed_secrets::client::{ManagedSecretConfigs, ManagedSecretsClient};
use crate::managed_secrets::client::{SecretOwner, TaskIdentityToken};

use super::ServerApi;

#[cfg_attr(not(target_family = "wasm"), async_trait)]
#[cfg_attr(target_family = "wasm", async_trait(?Send))]
impl ManagedSecretsClient for ServerApi {
    async fn get_managed_secret_configs(&self) -> Result<ManagedSecretConfigs> {
        Err(anyhow::anyhow!("cloud backend removed"))
    }

    async fn delete_managed_secret(&self, _owner: SecretOwner, _name: String) -> Result<()> {
        Err(anyhow::anyhow!("cloud backend removed"))
    }

    async fn issue_task_identity_token(
        &self,
        _options: crate::managed_secrets::client::IdentityTokenOptions,
    ) -> Result<TaskIdentityToken> {
        Err(anyhow::anyhow!("cloud backend removed"))
    }
}
