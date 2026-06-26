use std::collections::HashMap;
use std::future::Future;
use std::sync::Arc;
use std::time::Duration;

use vec1::vec1;
use warp_core::features::FeatureFlag;
use warpui_core::{Entity, SingletonEntity};

use crate::ManagedSecretValue;
use crate::client::{
    IdentityTokenOptions, ManagedSecretsClient, SecretListEntry, SecretMetadata, SecretOwner,
    TaskIdentityToken,
};
use crate::gcp::{self, GcpWorkloadIdentityFederationError, GcpWorkloadIdentityFederationToken};

/// Singleton model for working with Warp-managed secrets.
pub struct ManagedSecretManager {
    client: Arc<dyn ManagedSecretsClient>,
    actor_provider: Arc<dyn ActorProvider>,
}

pub trait ActorProvider: Send + Sync + 'static {
    fn actor_uid(&self) -> Option<String>;
}

impl ManagedSecretManager {
    pub fn new(
        client: Arc<dyn ManagedSecretsClient>,
        actor_provider: Arc<dyn ActorProvider>,
    ) -> Self {
        crate::envelope::init();
        Self {
            client,
            actor_provider,
        }
    }

    pub fn delete_secret(
        &self,
        owner: SecretOwner,
        name: String,
    ) -> impl Future<Output = anyhow::Result<()>> + use<> {
        let client = self.client.clone();
        async move {
            if !FeatureFlag::WarpManagedSecrets.is_enabled() {
                return Err(anyhow::anyhow!("This feature is not enabled"));
            }

            client.delete_managed_secret(owner, name).await?;
            Ok(())
        }
    }

    /// Issue a short-lived OIDC identity token for the current task.
    pub fn issue_task_identity_token(
        &self,
        options: IdentityTokenOptions,
    ) -> impl Future<Output = anyhow::Result<TaskIdentityToken>> + use<> {
        let client = self.client.clone();
        async move { client.issue_task_identity_token(options).await }
    }

    /// Issue a short-lived OIDC identity token in the JSON shape expected by
    /// GCP executable-sourced Workload Identity Federation credentials.
    pub fn issue_gcp_workload_identity_federation_token(
        &self,
        audience: String,
        token_type: String,
        requested_duration: Duration,
    ) -> impl Future<
        Output = Result<GcpWorkloadIdentityFederationToken, GcpWorkloadIdentityFederationError>,
    > + use<> {
        let client = self.client.clone();
        async move {
            match token_type.as_str() {
                gcp::TOKEN_TYPE_ID_TOKEN | gcp::TOKEN_TYPE_JWT => (),
                other => {
                    return Err(GcpWorkloadIdentityFederationError::new(format!(
                        "Unsupported token type `{other}`"
                    )));
                }
            }

            match client
                .issue_task_identity_token(IdentityTokenOptions {
                    audience,
                    requested_duration,
                    subject_template: vec1!["principal".to_owned()],
                })
                .await
            {
                Ok(token) => Ok(GcpWorkloadIdentityFederationToken::new(token, token_type)),
                Err(err) => Err(GcpWorkloadIdentityFederationError::new(err.to_string())),
            }
        }
    }

    pub fn get_task_secrets(
        &self,
        _task_id: String,
    ) -> impl Future<Output = anyhow::Result<HashMap<String, ManagedSecretValue>>> + use<> {
        async move { todo!("GraphQL backend removed") }
    }

    pub fn create_secret(
        &self,
        _owner: SecretOwner,
        _name: String,
        _value: ManagedSecretValue,
        _description: Option<String>,
    ) -> impl Future<Output = anyhow::Result<SecretMetadata>> + use<> {
        async move { todo!("GraphQL backend removed") }
    }

    pub fn list_secrets(
        &self,
    ) -> impl Future<Output = anyhow::Result<Vec<SecretListEntry>>> + use<> {
        async move { todo!("GraphQL backend removed") }
    }

    pub fn update_secret(
        &self,
        _owner: SecretOwner,
        _name: String,
        _value: Option<ManagedSecretValue>,
        _description: Option<String>,
    ) -> impl Future<Output = anyhow::Result<SecretMetadata>> + use<> {
        async move { todo!("GraphQL backend removed") }
    }
}

impl Entity for ManagedSecretManager {
    type Event = ();
}

impl SingletonEntity for ManagedSecretManager {}
