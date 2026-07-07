//! Minimal builders for constructing test inputs.
//!
//! These create the minimum viable `ByopOutputInput` and `RequestParams`
//! for testing chat_stream functions without wiring up the full app.

use std::sync::Arc;

use futures::channel::oneshot;
use warp_multi_agent_api as api;

use crate::ai::agent::api::RequestParams;
use crate::ai::agent::{AIAgentInput, UserQueryMode};
use crate::ai::agent_providers::attachment_caps::AttachmentCaps;
use crate::ai::agent_providers::chat_stream::{ByopOutputInput, TitleGenInput};
use crate::settings::{AgentProviderApiType, ReasoningEffortSetting};

/// Build a minimal `ByopOutputInput` pointing at a yakbak replay server.
///
/// # Arguments
/// * `base_url` - The yakbak server URL (e.g., "http://127.0.0.1:PORT/")
/// * `api_type` - The provider API type to simulate
pub fn minimal_byop_input(base_url: &str, api_type: AgentProviderApiType) -> ByopOutputInput {
    let (_cancel_tx, cancel_rx) = oneshot::channel::<()>();

    ByopOutputInput {
        params: minimal_request_params(),
        base_url: base_url.to_string(),
        api_key: "test-key-not-real".to_string(),
        model_id: "test-model".to_string(),
        api_type,
        reasoning_effort: ReasoningEffortSetting::Auto,
        extra_headers: Vec::new(),
        task_id: "test-task-1".to_string(),
        target_task_id: "test-task-1".to_string(),
        needs_create_task: true,
        lrc_command_id: None,
        lrc_should_spawn_subagent: false,
        context_window: Some(128_000),
        cancellation_rx: cancel_rx,
        attachment_caps: AttachmentCaps::default(),
    }
}

/// Build a `ByopOutputInput` with a cancellation sender returned for test control.
pub fn byop_input_with_cancel(
    base_url: &str,
    api_type: AgentProviderApiType,
) -> (ByopOutputInput, oneshot::Sender<()>) {
    let (cancel_tx, cancel_rx) = oneshot::channel::<()>();

    let input = ByopOutputInput {
        params: minimal_request_params(),
        base_url: base_url.to_string(),
        api_key: "test-key-not-real".to_string(),
        model_id: "test-model".to_string(),
        api_type,
        reasoning_effort: ReasoningEffortSetting::Auto,
        extra_headers: Vec::new(),
        task_id: "test-task-1".to_string(),
        target_task_id: "test-task-1".to_string(),
        needs_create_task: true,
        lrc_command_id: None,
        lrc_should_spawn_subagent: false,
        context_window: Some(128_000),
        cancellation_rx: cancel_rx,
        attachment_caps: AttachmentCaps::default(),
    };

    (input, cancel_tx)
}

/// Build a minimal `ByopOutputInput` for real provider tests.
///
/// Reads configuration from environment variables:
/// - `BYOP_TEST_BASE_URL` (default: "https://ds-api.xnurta.com/")
/// - `BYOP_TEST_API_KEY` (required)
/// - `BYOP_TEST_MODEL` (default: "claude-opus-4-8")
pub fn real_provider_input() -> Option<ByopOutputInput> {
    let api_key = std::env::var("BYOP_TEST_API_KEY").ok()?;
    let base_url =
        std::env::var("BYOP_TEST_BASE_URL").unwrap_or_else(|_| "https://ds-api.xnurta.com/".into());
    let model_id =
        std::env::var("BYOP_TEST_MODEL").unwrap_or_else(|_| "claude-opus-4-8".into());

    let (_cancel_tx, cancel_rx) = oneshot::channel::<()>();

    Some(ByopOutputInput {
        params: minimal_request_params_with_user_message("Say hello in exactly 3 words."),
        base_url,
        api_key,
        model_id,
        api_type: AgentProviderApiType::Anthropic,
        reasoning_effort: ReasoningEffortSetting::Auto,
        extra_headers: Vec::new(),
        task_id: "real-test-task-1".to_string(),
        target_task_id: "real-test-task-1".to_string(),
        needs_create_task: true,
        lrc_command_id: None,
        lrc_should_spawn_subagent: false,
        context_window: Some(200_000),
        cancellation_rx: cancel_rx,
        attachment_caps: AttachmentCaps::default(),
    })
}

/// Build a minimal `TitleGenInput` for real provider tests.
pub fn real_title_input() -> Option<TitleGenInput> {
    let api_key = std::env::var("BYOP_TEST_API_KEY").ok()?;
    let base_url =
        std::env::var("BYOP_TEST_BASE_URL").unwrap_or_else(|_| "https://ds-api.xnurta.com/".into());
    let model_id =
        std::env::var("BYOP_TEST_MODEL").unwrap_or_else(|_| "claude-opus-4-8".into());

    Some(TitleGenInput {
        base_url,
        api_key,
        model_id,
        api_type: AgentProviderApiType::Anthropic,
        reasoning_effort: ReasoningEffortSetting::Auto,
    })
}

/// Create a minimal `RequestParams` with a simple user message.
pub fn minimal_request_params() -> RequestParams {
    minimal_request_params_with_user_message("Hello")
}

/// Create a `RequestParams` with a specific user message.
pub fn minimal_request_params_with_user_message(msg: &str) -> RequestParams {
    let task = api::Task {
        id: "test-task-1".to_string(),
        messages: vec![api::Message {
            id: "msg-user-1".to_string(),
            task_id: "test-task-1".to_string(),
            message: Some(api::message::Message::UserQuery(api::message::UserQuery {
                query: msg.to_string(),
                ..Default::default()
            })),
            ..Default::default()
        }],
        ..Default::default()
    };

    let input = vec![AIAgentInput::UserQuery {
        query: msg.to_string(),
        context: Arc::from(vec![]),
        static_query_type: None,
        referenced_attachments: Default::default(),
        user_query_mode: UserQueryMode::Normal,
        running_command: None,
        intended_agent: None,
    }];

    RequestParams::new_for_test(input, vec![task])
}

/// Create a `RequestParams` with multi-turn conversation history.
pub fn params_with_history(messages: Vec<api::Message>) -> RequestParams {
    let task = api::Task {
        id: "test-task-1".to_string(),
        messages,
        ..Default::default()
    };

    RequestParams::new_for_test(Vec::new(), vec![task])
}
