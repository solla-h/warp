use std::collections::HashMap;
use std::future::Future;
use std::sync::Arc;
use std::time::Duration;

use vec1::vec1;
use warp_cli::agent::Harness;
use warp_core::features::FeatureFlag;
use warpui_core::{Entity, SingletonEntity};

use super::ManagedSecretValue;
use super::client::{
    IdentityTokenOptions, ManagedSecretsClient, SecretOwner, TaskIdentityToken,
};
use super::gcp::{GcpWorkloadIdentityFederationError, GcpWorkloadIdentityFederationToken};

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
        Self {
            client,
            actor_provider,
        }
    }

    pub fn create_secret(
        &self,
        _owner: SecretOwner,
        _name: String,
        _value: ManagedSecretValue,
        _description: Option<String>,
    ) -> impl Future<Output = anyhow::Result<super::client::SecretMetadata>> + use<> {
        async move {
            Err(anyhow::anyhow!("Managed secrets service is not available"))
        }
    }

    pub fn delete_secret(
        &self,
        _owner: SecretOwner,
        _name: String,
    ) -> impl Future<Output = anyhow::Result<()>> + use<> {
        async move {
            Err(anyhow::anyhow!("Managed secrets service is not available"))
        }
    }

    pub fn update_secret(
        &self,
        _owner: SecretOwner,
        _name: String,
        _value: Option<ManagedSecretValue>,
        _description: Option<String>,
    ) -> impl Future<Output = anyhow::Result<super::client::SecretMetadata>> + use<> {
        async move {
            Err(anyhow::anyhow!("Managed secrets service is not available"))
        }
    }

    pub fn list_secrets(
        &self,
    ) -> impl Future<Output = anyhow::Result<Vec<super::SecretListEntry>>> + use<> {
        async move {
            Err(anyhow::anyhow!("Managed secrets listing: GraphQL backend removed, needs REST migration"))
        }
    }

    pub fn list_harness_auth_secrets(
        &self,
        harness: warp_cli::agent::Harness,
    ) -> impl Future<Output = anyhow::Result<Vec<super::AuthSecretEntry>>> + use<> {
        let client = self.client.clone();
        async move {
            if !FeatureFlag::WarpManagedSecrets.is_enabled() {
                return Err(anyhow::anyhow!("This feature is not enabled"));
            }
            client.list_harness_auth_secrets(harness).await
        }
    }

    pub fn get_task_secrets(
        &self,
        _task_id: String,
    ) -> impl Future<Output = anyhow::Result<HashMap<String, ManagedSecretValue>>> + use<> {
        async move {
            Err(anyhow::anyhow!("get_task_secrets: GraphQL backend removed, needs REST migration"))
        }
    }

    pub fn issue_task_identity_token(
        &self,
        options: IdentityTokenOptions,
    ) -> impl Future<Output = anyhow::Result<TaskIdentityToken>> + use<> {
        let client = self.client.clone();
        async move { client.issue_task_identity_token(options).await }
    }

    pub fn gcp_workload_identity_federation(
        &self,
        _task_id: &str,
        audience: &str,
        requested_duration: Duration,
    ) -> impl Future<Output = Result<GcpWorkloadIdentityFederationToken, GcpWorkloadIdentityFederationError>>
           + use<> {
        let client = self.client.clone();
        let audience = audience.to_owned();
        async move {
            let options = IdentityTokenOptions {
                audience,
                requested_duration,
                subject_template: vec1!["run_id".into()],
            };
            let token = client
                .issue_task_identity_token(options)
                .await
                .map_err(|e| GcpWorkloadIdentityFederationError::new(e.to_string()))?;
            Ok(GcpWorkloadIdentityFederationToken::new(token))
        }
    }

    pub fn gcp_refresh_identity_token(
        &self,
        task_id: &str,
        audience: &str,
        requested_duration: Duration,
    ) -> impl Future<Output = Result<GcpWorkloadIdentityFederationToken, GcpWorkloadIdentityFederationError>>
           + use<> {
        self.gcp_workload_identity_federation(task_id, audience, requested_duration)
    }

    pub fn issue_gcp_workload_identity_federation_token(
        &self,
        audience: String,
        _token_type: String,
        requested_duration: Duration,
    ) -> impl Future<Output = Result<GcpWorkloadIdentityFederationToken, GcpWorkloadIdentityFederationError>>
           + use<> {
        self.gcp_workload_identity_federation("", &audience, requested_duration)
    }
}

impl Entity for ManagedSecretManager {
    type Event = ();
}
impl SingletonEntity for ManagedSecretManager {}

