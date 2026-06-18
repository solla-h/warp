//! Transport types for MCP (Model Context Protocol) server configuration.
//!
//! These describe *how* an MCP server is reached — a local CLI subprocess or a
//! Server-Sent-Events HTTP endpoint. They are intentionally dependency-light
//! (serde only) so they can be shared by:
//!
//! - the local MCP runtime ([`mcp`] crate), which spawns the servers, and
//! - cloud storage ([`cloud_object_models`]), which persists MCP server configs.
//!
//! Keeping them in a standalone crate lets the local runtime depend on these
//! types without pulling in any cloud machinery.

use serde::{Deserialize, Serialize};

/// How an MCP server is reached.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum TransportType {
    /// A server spawned as a local child process speaking JSON-RPC over stdio.
    CLIServer(CLIServer),
    /// A server reached over HTTP via Server-Sent Events.
    ServerSentEvents(ServerSentEvents),
}

/// A locally-spawned MCP CLI server.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CLIServer {
    pub command: String,
    #[serde(default)]
    pub args: Vec<String>,
    pub cwd_parameter: Option<String>,
    /// Static env vars added via editor inputs.
    pub static_env_vars: Vec<StaticEnvVar>,
}

/// An MCP server reached over HTTP via Server-Sent Events.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ServerSentEvents {
    pub url: String,
    /// Static headers added via editor inputs.
    #[serde(default)]
    pub headers: Vec<StaticHeader>,
}

/// A static environment variable added to an MCP CLI server.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct StaticEnvVar {
    pub name: String,
    /// To avoid leaking environment variables, we ensure that values are not
    /// serialized before being sent to our servers.
    #[serde(skip_serializing, default)]
    pub value: String,
}

/// A static header added to an MCP SSE server request.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct StaticHeader {
    pub name: String,
    /// To avoid leaking header values (which may contain secrets), we ensure that values are not
    /// serialized before being sent to our servers.
    #[serde(skip_serializing, default)]
    pub value: String,
}
