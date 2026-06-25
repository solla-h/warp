use std::collections::HashMap;
use std::ffi::OsString;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::time::Duration;

use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use tempfile::NamedTempFile;

use super::client::TaskIdentityToken;

#[derive(Debug, Clone)]
pub struct GcpFederationConfig {
    pub project_number: String,
    pub pool_id: String,
    pub provider_id: String,
    pub service_account_email: Option<String>,
    pub token_lifetime: Option<Duration>,
}

pub struct GcpCredentials {
    config_file: NamedTempFile,
    output_file: NamedTempFile,
}

impl GcpCredentials {
    pub fn federated(
        task_id: &str,
        config: &GcpFederationConfig,
    ) -> Result<Self, PrepareGcpCredentialsError> {
        let oz_binary_path =
            std::env::current_exe().map_err(|_| PrepareGcpCredentialsError::NoBinaryPath)?;

        let output_file = NamedTempFile::new()
            .map_err(|source| PrepareGcpCredentialsError::FileCreate { source })?;

        let cred_config_json =
            generate_gcp_credential_config(task_id, config, &oz_binary_path, output_file.path())?;

        let json_bytes = serde_json::to_vec_pretty(&cred_config_json)
            .map_err(PrepareGcpCredentialsError::SerializeConfig)?;

        let mut config_file = NamedTempFile::new()
            .map_err(|source| PrepareGcpCredentialsError::FileCreate { source })?;
        config_file.write_all(&json_bytes).map_err(|source| {
            PrepareGcpCredentialsError::FileWrite {
                path: config_file.path().to_path_buf(),
                source,
            }
        })?;

        Ok(Self {
            config_file,
            output_file,
        })
    }

    pub fn env_vars(&self) -> HashMap<OsString, OsString> {
        let config_file_path = self.config_file.path().as_os_str();
        let mut vars = HashMap::with_capacity(3);
        vars.insert(
            OsString::from("GOOGLE_EXTERNAL_ACCOUNT_ALLOW_EXECUTABLES"),
            OsString::from("1"),
        );
        vars.insert(
            OsString::from("GOOGLE_APPLICATION_CREDENTIALS"),
            config_file_path.to_owned(),
        );
        vars.insert(
            OsString::from("CLOUDSDK_AUTH_CREDENTIAL_FILE_OVERRIDE"),
            config_file_path.to_owned(),
        );
        vars
    }

    pub fn cleanup(self) -> std::io::Result<()> {
        self.config_file.close()?;
        self.output_file.close()?;
        Ok(())
    }
}

#[derive(Debug, thiserror::Error)]
pub enum PrepareGcpCredentialsError {
    #[error("Could not determine the current executable path")]
    NoBinaryPath,
    #[error("Cannot use executable {} for GCP executable-sourced credentials", path.display())]
    InvalidBinaryPath { path: PathBuf },
    #[error("Cannot use run {task_id} for GCP executable-sourced credentials")]
    InvalidTaskId { task_id: String },
    #[error("Failed to create credential config file: {source}")]
    FileCreate {
        #[source]
        source: std::io::Error,
    },
    #[error("Failed to serialize credential config: {0}")]
    SerializeConfig(#[source] serde_json::Error),
    #[error("Failed to write credential config to {path}: {source}")]
    FileWrite {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct GcpWorkloadIdentityFederationToken {
    pub version: u8,
    pub success: bool,
    pub token_type: String,
    pub id_token: String,
    pub expiration_time: i64,
}

impl GcpWorkloadIdentityFederationToken {
    pub(crate) fn new(token: TaskIdentityToken) -> Self {
        Self {
            version: 1,
            success: true,
            token_type: "urn:ietf:params:oauth:token-type:id_token".to_owned(),
            id_token: token.token,
            expiration_time: token.expires_at.timestamp(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct GcpWorkloadIdentityFederationError {
    pub version: u8,
    pub success: bool,
    pub code: String,
    pub message: String,
}

impl GcpWorkloadIdentityFederationError {
    pub(crate) fn new(message: impl Into<String>) -> Self {
        Self {
            version: 1,
            success: false,
            code: "TOKEN_ISSUANCE_FAILED".into(),
            message: message.into(),
        }
    }
}

fn generate_gcp_credential_config(
    task_id: &str,
    config: &GcpFederationConfig,
    oz_binary_path: &Path,
    output_file: &Path,
) -> Result<Value, PrepareGcpCredentialsError> {
    let audience = format!(
        "//iam.googleapis.com/projects/{}/locations/global/workloadIdentityPools/{}/providers/{}",
        config.project_number, config.pool_id, config.provider_id
    );

    let oz_binary_display = oz_binary_path.display().to_string();
    if oz_binary_display.contains(' ') {
        return Err(PrepareGcpCredentialsError::InvalidBinaryPath {
            path: oz_binary_path.to_path_buf(),
        });
    }
    if task_id.contains(' ') {
        return Err(PrepareGcpCredentialsError::InvalidTaskId {
            task_id: task_id.to_owned(),
        });
    }

    let mut command = format!("{oz_binary_display} federate issue-gcp-token --run-id {task_id}");
    if let Some(lifetime) = config.token_lifetime {
        command.push_str(&format!(" --duration {}s", lifetime.as_secs()));
    }

    let mut cred_config = json!({
        "type": "external_account",
        "audience": audience,
        "subject_token_type": "urn:ietf:params:oauth:token-type:id_token",
        "token_url": "https://sts.googleapis.com/v1/token",
        "credential_source": {
            "executable": {
                "command": command,
                "timeout_millis": 30000,
                "output_file": output_file.display().to_string()
            }
        }
    });

    if let Some(email) = &config.service_account_email {
        let impersonation_url = format!(
            "https://iamcredentials.googleapis.com/v1/projects/-/serviceAccounts/{email}:generateAccessToken"
        );
        cred_config["service_account_impersonation_url"] = json!(impersonation_url);

        if let Some(lifetime) = config.token_lifetime {
            cred_config["service_account_impersonation"] = json!({
                "token_lifetime_seconds": lifetime.as_secs()
            });
        }
    }

    Ok(cred_config)
}

