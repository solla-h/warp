//! Client construction and adapter routing for BYOP chat_stream.
//!
//! Extracted from `mod.rs` to isolate provider-specific URL normalization,
//! adapter kind mapping, and genai `Client` construction.

use genai::adapter::AdapterKind;
use genai::resolver::{AuthData, Endpoint, ServiceTargetResolver};
use genai::{Client, ModelIden, ServiceTarget, WebConfig};

use crate::ai::agent_providers::openai_compatible::OpenAiCompatibleError;
use crate::settings::AgentProviderApiType;

// ---------------------------------------------------------------------------
// Adapter routing
// ---------------------------------------------------------------------------

/// Maps [`AgentProviderApiType`] (user-facing settings enum) to the genai
/// [`AdapterKind`] that drives wire-format encoding/decoding.
pub(crate) fn adapter_kind_for(api_type: AgentProviderApiType) -> AdapterKind {
    match api_type {
        AgentProviderApiType::OpenAi => AdapterKind::OpenAI,
        AgentProviderApiType::OpenAiResp => AdapterKind::OpenAIResp,
        AgentProviderApiType::Gemini => AdapterKind::Gemini,
        AgentProviderApiType::Anthropic => AdapterKind::Anthropic,
        AgentProviderApiType::Ollama => AdapterKind::Ollama,
        AgentProviderApiType::DeepSeek => AdapterKind::DeepSeek,
    }
}

// ---------------------------------------------------------------------------
// URL normalization
// ---------------------------------------------------------------------------

/// Normalizes user-supplied `base_url` into a form genai adapters expect.
///
/// Three user input patterns:
/// 1. Bare host (`https://ai.zerx.dev`) -- appends the api_type default path segment.
/// 2. Full versioned path (`https://ai.zerx.dev/v1`) -- only ensures trailing `/`.
/// 3. Empty -- uses [`AgentProviderApiType::default_base_url`].
pub(crate) fn normalize_endpoint_url(api_type: AgentProviderApiType, base_url: &str) -> String {
    let trimmed = base_url.trim();
    if trimmed.is_empty() {
        return api_type.default_base_url().to_owned();
    }

    // Parse failure (malformed URL) -- degrade to "append trailing /" behavior.
    let parsed = match url::Url::parse(trimmed) {
        Ok(u) => u,
        Err(_) => {
            let stripped = trimmed.trim_end_matches('/');
            return format!("{stripped}/");
        }
    };

    // path == "/" or empty -- user only filled host; append default version path.
    if parsed.path() == "/" || parsed.path().is_empty() {
        let default_path = url::Url::parse(api_type.default_base_url())
            .ok()
            .map(|u| u.path().to_owned())
            .unwrap_or_else(|| "/".to_owned());
        let host_part = trimmed.trim_end_matches('/');
        return format!("{host_part}{default_path}");
    }

    // User already has a path -- just ensure trailing `/`.
    let stripped = trimmed.trim_end_matches('/');
    format!("{stripped}/")
}

// ---------------------------------------------------------------------------
// User-Agent header
// ---------------------------------------------------------------------------

/// Constructs the `User-Agent` header value for outbound BYOP requests.
///
/// Format: `<AppName>/<version>` when a release tag is available,
/// otherwise just `<AppName>`.
pub(crate) fn build_user_agent_header(
) -> Result<reqwest::header::HeaderValue, reqwest::header::InvalidHeaderValue> {
    let app_name = warp_core::channel::ChannelState::app_id()
        .application_name()
        .to_owned();
    let ua = match warp_core::channel::ChannelState::app_version() {
        Some(v) if !v.is_empty() => format!("{app_name}/{v}"),
        _ => app_name,
    };
    reqwest::header::HeaderValue::from_str(&ua)
}

// ---------------------------------------------------------------------------
// Client construction
// ---------------------------------------------------------------------------

/// Constructs a genai [`Client`] configured for the given provider.
///
/// Each request creates a fresh client (cheap -- just a reqwest::Client + adapter table).
/// A [`ServiceTargetResolver`] captures the endpoint/key/api_type and forces every
/// `exec_chat_stream` call to the specified [`AdapterKind`], completely bypassing
/// genai's default "identify adapter by model name" behavior.
pub(crate) fn build_client(
    api_type: AgentProviderApiType,
    base_url: String,
    api_key: String,
) -> Client {
    let adapter_kind = adapter_kind_for(api_type);
    let endpoint_url = normalize_endpoint_url(api_type, &base_url);
    log::info!("[byop] build_client: adapter={adapter_kind:?} endpoint_url={endpoint_url}");
    let key_for_resolver = api_key.clone();
    let resolver = ServiceTargetResolver::from_resolver_fn(
        move |service_target: ServiceTarget| -> Result<ServiceTarget, genai::resolver::Error> {
            let ServiceTarget { model, .. } = service_target;
            let endpoint = Endpoint::from_owned(endpoint_url.clone());
            let auth = AuthData::from_single(key_for_resolver.clone());
            let model = ModelIden::new(adapter_kind, model.model_name);
            Ok(ServiceTarget {
                endpoint,
                auth,
                model,
            })
        },
    );

    let mut headers = reqwest::header::HeaderMap::new();
    if let Ok(value) = build_user_agent_header() {
        headers.insert(reqwest::header::USER_AGENT, value);
    }
    let web_config = WebConfig {
        gzip: false,
        default_headers: Some(headers),
        ..WebConfig::default()
    };

    Client::builder()
        .with_web_config(web_config)
        .with_service_target_resolver(resolver)
        .build()
}

// ---------------------------------------------------------------------------
// Error mapping
// ---------------------------------------------------------------------------

/// Maps a [`genai::Error`] into our [`OpenAiCompatibleError`] enum for uniform
/// error handling across the BYOP pipeline.
pub(crate) fn map_genai_error(err: genai::Error) -> OpenAiCompatibleError {
    use genai::Error as G;
    match err {
        // Parse failures: JSON deserialization stage
        G::StreamParse { .. }
        | G::SerdeJson(_)
        | G::JsonValueExt(_)
        | G::InvalidJsonResponseElement { .. } => OpenAiCompatibleError::Decode(format!("{err}")),

        // Network / streaming failures (reqwest connection, TLS, DNS, timeout, stream break)
        G::WebStream { .. } | G::WebAdapterCall { .. } | G::WebModelCall { .. } => {
            OpenAiCompatibleError::Stream(format!("{err}"))
        }

        // HTTP error status returned by server
        G::HttpError {
            status,
            body,
            canonical_reason,
        } => OpenAiCompatibleError::Status {
            status: status.as_u16(),
            body: if canonical_reason.is_empty() {
                body
            } else {
                format!("{canonical_reason}: {body}")
            },
        },

        // Everything else (request construction, auth, capability unsupported)
        other => OpenAiCompatibleError::Other(format!("{other}")),
    }
}
