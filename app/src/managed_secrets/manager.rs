use std::collections::HashMap;
use std::future::Future;
use std::sync::Arc;
use std::time::Duration;

use vec1::vec1;
use warp_core::features::FeatureFlag;
use warp_graphql::managed_secrets::ManagedSecret;
use warp_graphql::queries::task_secrets::ManagedSecretValue as GqlManagedSecretValue;
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
    ) -> impl Future<Output = anyhow::Result<ManagedSecret>> + use<> {
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
    ) -> impl Future<Output = anyhow::Result<ManagedSecret>> + use<> {
        async move {
            Err(anyhow::anyhow!("Managed secrets service is not available"))
        }
    }

    pub fn list_secrets(
        &self,
    ) -> impl Future<Output = anyhow::Result<Vec<ManagedSecret>>> + use<> {
        let client = self.client.clone();
        async move {
            if !FeatureFlag::WarpManagedSecrets.is_enabled() {
                return Err(anyhow::anyhow!("This feature is not enabled"));
            }
            client.list_secrets().await
        }
    }

    pub fn list_harness_auth_secrets(
        &self,
        harness: warp_graphql::ai::AgentHarness,
    ) -> impl Future<Output = anyhow::Result<Vec<ManagedSecret>>> + use<> {
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
        task_id: String,
    ) -> impl Future<Output = anyhow::Result<HashMap<String, ManagedSecretValue>>> + use<> {
        let client = self.client.clone();
        async move {
            let workload_token =
                warp_isolation_platform::issue_workload_token(Some(Duration::from_secs(300)))
                    .await?;
            let gql_secrets = client
                .get_task_secrets(task_id, workload_token.token)
                .await?;

            let mut secrets = HashMap::new();
            for (name, gql_value) in gql_secrets {
                let value = match gql_value {
                    GqlManagedSecretValue::ManagedSecretRawValue(raw) => {
                        ManagedSecretValue::raw_value(raw.value)
                    }
                    GqlManagedSecretValue::ManagedSecretAnthropicApiKeyValue(v) => {
                        ManagedSecretValue::anthropic_api_key(v.api_key)
                    }
                    GqlManagedSecretValue::ManagedSecretAnthropicBedrockAccessKeyValue(v) => {
                        ManagedSecretValue::anthropic_bedrock_access_key(
                            v.aws_access_key_id,
                            v.aws_secret_access_key,
                            v.aws_session_token,
                            v.aws_region,
                        )
                    }
                    GqlManagedSecretValue::ManagedSecretAnthropicBedrockApiKeyValue(v) => {
                        ManagedSecretValue::anthropic_bedrock_api_key(
                            v.aws_bearer_token_bedrock,
                            v.aws_region,
                        )
                    }
                    GqlManagedSecretValue::ManagedSecretOpenAiApiKeyValue(v) => {
                        ManagedSecretValue::openai_api_key(v.api_key, v.base_url)
                    }
                    GqlManagedSecretValue::Unknown => {
                        return Err(anyhow::anyhow!(
                            "Unknown secret value type for secret: {}",
                            name
                        ));
                    }
                };
                secrets.insert(name, value);
            }
            Ok(secrets)
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

