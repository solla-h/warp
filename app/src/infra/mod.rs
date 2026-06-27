#![cfg_attr(feature = "local-only", allow(dead_code, unused_imports, unused_variables))]

pub mod server_api;

pub use server_api::ai;
pub use server_api::auth;
pub use server_api::block;
pub use server_api::harness_support;
pub use server_api::integrations;
pub use server_api::object;
pub(crate) use server_api::presigned_upload;
pub use server_api::referral;
pub use server_api::team;
pub use server_api::workspace;

pub use server_api::{
    AIApiError, AIOutputStream, ClientError, CloudAgentCapacityError, DeserializationError,
    ServerApi, ServerTime, TranscribeError, FETCH_CHANNEL_VERSIONS_TIMEOUT,
};
pub(crate) use server_api::{
    AGENT_SOURCE_HEADER, AMBIENT_WORKLOAD_TOKEN_HEADER, CLOUD_AGENT_ID_HEADER,
};
#[cfg(feature = "agent_mode_evals")]
pub use server_api::EVAL_USER_ID_HEADER;

pub use server_api::ServerApiProvider as ServiceProvider;