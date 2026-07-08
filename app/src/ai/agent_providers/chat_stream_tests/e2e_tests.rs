//! End-to-end tests for `generate_byop_output`.
//!
//! These tests exercise the full pipeline:
//! RequestParams → build_chat_request → genai Client → SSE stream → ResponseEvent
//!
//! Offline tests use yakbak replay; online tests hit real providers (marked #[ignore]).

use std::collections::HashMap;
use std::sync::Arc;

use futures::channel::oneshot;
use warp_multi_agent_api as api;

use crate::ai::agent::api::RequestParams;
use crate::ai::agent::{AIAgentContext, AIAgentInput, UserQueryMode};
use crate::ai::agent_providers::attachment_caps::AttachmentCaps;
use crate::ai::agent_providers::chat_stream::{generate_byop_output, ByopOutputInput};
use crate::settings::{AgentProviderApiType, ReasoningEffortSetting};

use super::stream_collector::CollectedStream;
use super::yakbak_harness::start_fixture_replay;

fn user_query_input(query: &str) -> AIAgentInput {
    AIAgentInput::UserQuery {
        query: query.to_owned(),
        context: Arc::<[AIAgentContext]>::from([]),
        static_query_type: None,
        referenced_attachments: HashMap::new(),
        user_query_mode: UserQueryMode::default(),
        running_command: None,
        intended_agent: None,
    }
}

fn minimal_byop_input(
    base_url: String,
    api_key: String,
    model_id: String,
    api_type: AgentProviderApiType,
    query: &str,
) -> ByopOutputInput {
    let (_, cancel_rx) = oneshot::channel::<()>();
    let params = RequestParams::new_for_test(vec![user_query_input(query)], vec![]);
    ByopOutputInput {
        params,
        base_url,
        api_key,
        model_id,
        api_type,
        reasoning_effort: ReasoningEffortSetting::Off,
        extra_headers: vec![],
        task_id: "task-e2e".to_owned(),
        target_task_id: "task-e2e".to_owned(),
        needs_create_task: true,
        lrc_command_id: None,
        lrc_should_spawn_subagent: false,
        context_window: None,
        cancellation_rx: cancel_rx,
        attachment_caps: AttachmentCaps::default(),
    }
}

// ---------------------------------------------------------------------------
// Offline E2E: yakbak replay
// NOTE: Currently ignored — genai's Anthropic adapter rejects our simplified
// cassette format (missing HTTP response headers). Fixing this requires recording
// real responses with all headers intact, or implementing a more faithful HTTP mock.
// ---------------------------------------------------------------------------

#[tokio::test]
#[ignore]
async fn e2e_yakbak_simple_text_stream() {
    let mut server = start_fixture_replay("anthropic", "simple_text")
        .await
        .expect("failed to start yakbak");

    let input = minimal_byop_input(
        server.base_url(),
        "sk-ant-api03-test-key-000000".to_owned(),
        "claude-opus-4-8".to_owned(),
        AgentProviderApiType::Anthropic,
        "Hello",
    );

    let stream = generate_byop_output(input).await.expect("generate_byop_output failed");
    let collected = CollectedStream::collect_from(stream).await;

    // The simple_text cassette emits "Hello! How can I help you today?"
    // Text may arrive via AppendToMessageContent (streaming) or AddMessagesToTask (final)
    assert!(
        collected.text_content.contains("Hello") || collected.text_content.contains("help"),
        "expected text containing 'Hello' or 'help', got: {:?}",
        collected.text_content
    );
    assert!(
        collected.finished_done || collected.finished_error,
        "stream should finish, events={:?}",
        collected.events.len()
    );

    server.shutdown().await;
}

#[tokio::test]
#[ignore]
async fn e2e_yakbak_tool_call_response() {
    let mut server = start_fixture_replay("anthropic", "tool_call_single")
        .await
        .expect("failed to start yakbak");

    let input = minimal_byop_input(
        server.base_url(),
        "sk-ant-api03-test-key-000000".to_owned(),
        "claude-opus-4-8".to_owned(),
        AgentProviderApiType::Anthropic,
        "Read the file /tmp/test.txt",
    );

    let stream = generate_byop_output(input).await.expect("generate_byop_output failed");
    let collected = CollectedStream::collect_from(stream).await;

    // The tool_call_single cassette has text + a read_file tool call
    assert!(
        collected.text_content.contains("search"),
        "expected some text before tool call, got: {:?}",
        collected.text_content
    );
    // Tool call should be emitted
    assert!(
        !collected.tool_calls.is_empty(),
        "expected at least one tool call"
    );

    server.shutdown().await;
}

#[tokio::test]
#[ignore]
async fn e2e_yakbak_reasoning_response() {
    let mut server = start_fixture_replay("anthropic", "reasoning_with_text")
        .await
        .expect("failed to start yakbak");

    let input = minimal_byop_input(
        server.base_url(),
        "sk-ant-api03-test-key-000000".to_owned(),
        "claude-opus-4-8".to_owned(),
        AgentProviderApiType::Anthropic,
        "Think about this",
    );

    let stream = generate_byop_output(input).await.expect("generate_byop_output failed");
    let collected = CollectedStream::collect_from(stream).await;

    assert!(collected.finished_done, "stream should finish with Done");
    // Should have some text output (reasoning cassette has both reasoning + text)
    assert!(
        !collected.text_content.is_empty() || !collected.reasoning_content.is_empty(),
        "expected either text or reasoning content"
    );

    server.shutdown().await;
}

// ---------------------------------------------------------------------------
// Online E2E: real provider (requires env vars)
// ---------------------------------------------------------------------------

#[tokio::test]
#[ignore]
async fn e2e_real_provider_simple_response() {
    let api_key = std::env::var("BYOP_TEST_API_KEY")
        .expect("BYOP_TEST_API_KEY must be set for real provider tests");
    let base_url = std::env::var("BYOP_TEST_BASE_URL")
        .unwrap_or_else(|_| "https://api.anthropic.com".to_owned());
    let model_id = std::env::var("BYOP_TEST_MODEL")
        .unwrap_or_else(|_| "claude-opus-4-8".to_owned());

    let input = minimal_byop_input(
        base_url,
        api_key,
        model_id,
        AgentProviderApiType::Anthropic,
        "Reply with exactly: E2E_OK",
    );

    let stream = generate_byop_output(input).await.expect("generate_byop_output failed");
    let collected = CollectedStream::collect_from(stream).await;

    eprintln!("[e2e_real] event_count={} text_content_len={} reasoning_len={} finished_done={} finished_error={} tool_calls={}",
        collected.event_count(),
        collected.text_content.len(),
        collected.reasoning_content.len(),
        collected.finished_done,
        collected.finished_error,
        collected.tool_calls.len(),
    );
    for (i, event) in collected.events.iter().enumerate() {
        if let Some(api::response_event::Type::ClientActions(ca)) = &event.r#type {
            for (j, action) in ca.actions.iter().enumerate() {
                eprintln!("[e2e_real] event[{i}].action[{j}]: {:?}", action.action.as_ref().map(|a| std::mem::discriminant(a)));
                if let Some(api::client_action::Action::AppendToMessageContent(append)) = &action.action {
                    eprintln!("[e2e_real]   append message: {:?}", append.message.as_ref().map(|m| m.message.as_ref().map(|mm| std::mem::discriminant(mm))));
                }
                if let Some(api::client_action::Action::AddMessagesToTask(add)) = &action.action {
                    for msg in &add.messages {
                        eprintln!("[e2e_real]   add_msg: type={:?} text={}", msg.message.as_ref().map(|mm| std::mem::discriminant(mm)), &msg.server_message_data[..msg.server_message_data.len().min(100)]);
                    }
                }
            }
        }
    }

    assert!(collected.finished_done, "stream should finish with Done, error={}", collected.finished_error);
    assert!(
        !collected.text_content.is_empty(),
        "expected non-empty text response, events={}",
        collected.event_count()
    );
    eprintln!("[e2e_real] text_content = {:?}", collected.text_content);
}
