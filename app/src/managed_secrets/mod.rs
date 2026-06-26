pub mod client;
mod gcp;
mod manager;
mod secret_value;

pub use client::TaskIdentityToken;
pub use client::{AuthSecretEntry, SecretListEntry, SecretMetadata};
pub use gcp::{
    GcpCredentials, GcpFederationConfig, GcpWorkloadIdentityFederationError,
    GcpWorkloadIdentityFederationToken, PrepareGcpCredentialsError,
};
pub use manager::{ActorProvider, ManagedSecretManager};
pub use secret_value::ManagedSecretValue;
