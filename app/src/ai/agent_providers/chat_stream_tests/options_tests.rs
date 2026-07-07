//! Unit tests for `chat_stream::options` module.

use genai::chat::{CacheControl, ChatMessage, ChatRole};

use crate::ai::agent::api::RequestParams;
use crate::ai::agent::{AIAgentInput, UserQueryMode};
use crate::ai::agent_providers::chat_stream::options::{
    apply_caching_anthropic, available_tool_names, build_chat_options, build_tools_array,
    dashscope_needs_enable_thinking, is_plan_mode_turn, PLAN_MODE_BLOCKED_TOOLS,
};
use crate::settings::{AgentProviderApiType, ReasoningEffortSetting as R};
use std::sync::Arc;

use super::builders::minimal_request_params;

// ---------------------------------------------------------------------------
// build_chat_options tests
// ---------------------------------------------------------------------------

/// Auto effort setting → no reasoning params sent (reasoning_effort = None).
#[test]
fn test_build_chat_options_auto_effort() {
    let opts = build_chat_options(
        AgentProviderApiType::Anthropic,
        "https://api.anthropic.com/v1/",
        "claude-opus-4-7",
        R::Auto,
        vec![],
        None,
    );
    assert!(
        opts.reasoning_effort.is_none(),
        "Auto should not inject reasoning_effort"
    );
    assert!(opts.extra_body.is_none());
}

/// DeepSeek + Off → extra_body thinking.type=disabled.
#[test]
fn test_build_chat_options_deepseek_off() {
    let opts = build_chat_options(
        AgentProviderApiType::DeepSeek,
        "https://api.deepseek.com/v1/",
        "deepseek-v4-flash",
        R::Off,
        vec![],
        None,
    );
    assert!(
        opts.reasoning_effort.is_none(),
        "DeepSeek+Off should not set reasoning_effort"
    );
    let body = opts.extra_body.as_ref().expect("extra_body must be set");
    assert_eq!(
        body.pointer("/thinking/type"),
        Some(&serde_json::Value::String("disabled".to_string())),
    );
}

/// Anthropic + High → budget_tokens / reasoning_effort=High set.
#[test]
fn test_build_chat_options_anthropic_high_effort() {
    let opts = build_chat_options(
        AgentProviderApiType::Anthropic,
        "https://api.anthropic.com/v1/",
        "claude-opus-4-7",
        R::High,
        vec![],
        None,
    );
    assert!(
        matches!(opts.reasoning_effort, Some(genai::chat::ReasoningEffort::High)),
        "Anthropic+High should inject reasoning_effort=High"
    );
}

// ---------------------------------------------------------------------------
// build_tools_array / available_tool_names tests
// ---------------------------------------------------------------------------

fn make_plan_mode_params() -> RequestParams {
    let task = warp_multi_agent_api::Task {
        id: "test-task-1".to_string(),
        messages: vec![],
        ..Default::default()
    };
    let input = vec![AIAgentInput::UserQuery {
        query: "plan something".to_string(),
        context: Arc::from(vec![]),
        static_query_type: None,
        referenced_attachments: Default::default(),
        user_query_mode: UserQueryMode::Plan,
        running_command: None,
        intended_agent: None,
    }];
    RequestParams::new_for_test(input, vec![task])
}

/// Plan mode → write/exec tools removed from tools array.
#[test]
fn test_tools_array_filters_plan_mode() {
    let params = make_plan_mode_params();
    let tools = build_tools_array(&params);
    let names: Vec<&str> = tools.iter().map(|t| t.name.as_str()).collect();
    for blocked in PLAN_MODE_BLOCKED_TOOLS {
        assert!(
            !names.contains(blocked),
            "Plan mode should filter out tool: {blocked}"
        );
    }
    // Verify at least some tools remain
    assert!(!tools.is_empty(), "Some tools should remain after filtering");
}

/// web_search_enabled=false → webfetch/websearch removed.
#[test]
fn test_tools_array_filters_websearch_disabled() {
    // minimal_request_params has web_search_enabled=false by default
    let params = minimal_request_params();
    let tools = build_tools_array(&params);
    let names: Vec<&str> = tools.iter().map(|t| t.name.as_str()).collect();
    assert!(
        !names.contains(&"webfetch"),
        "webfetch should be filtered when web_search_enabled=false"
    );
    assert!(
        !names.contains(&"websearch"),
        "websearch should be filtered when web_search_enabled=false"
    );
}

/// available_tool_names returns a non-empty list.
#[test]
fn test_available_tool_names_basic() {
    let params = minimal_request_params();
    let names = available_tool_names(&params);
    assert!(
        !names.is_empty(),
        "available_tool_names should return at least one tool"
    );
    // Should contain some known tools
    assert!(
        names.iter().any(|n| n == "read_files"),
        "read_files should be in available tools"
    );
}

// ---------------------------------------------------------------------------
// apply_caching_anthropic tests
// ---------------------------------------------------------------------------

fn extract_cache_control(msg: &ChatMessage) -> Option<CacheControl> {
    msg.options.as_ref().and_then(|o| o.cache_control.clone())
}

/// System messages get Ephemeral1h cache control.
#[test]
fn test_caching_anthropic_system_gets_1h() {
    let mut msgs = vec![
        ChatMessage::system("You are a helpful assistant."),
        ChatMessage::user("Hello"),
        ChatMessage::assistant("Hi there"),
        ChatMessage::user("How are you?"),
    ];
    apply_caching_anthropic(&mut msgs);
    let sys_cc = extract_cache_control(&msgs[0]);
    assert_eq!(
        sys_cc,
        Some(CacheControl::Ephemeral1h),
        "System message should get Ephemeral1h"
    );
}

/// Last 2 non-system messages get Ephemeral5m cache control.
#[test]
fn test_caching_anthropic_last_two_get_5m() {
    let mut msgs = vec![
        ChatMessage::system("System prompt"),
        ChatMessage::user("First question"),
        ChatMessage::assistant("First answer"),
        ChatMessage::user("Second question"),
        ChatMessage::assistant("Second answer"),
        ChatMessage::user("Third question"),
    ];
    apply_caching_anthropic(&mut msgs);
    // Last 2 non-system = idx 4 (assistant) and idx 5 (user)
    let cc4 = extract_cache_control(&msgs[4]);
    let cc5 = extract_cache_control(&msgs[5]);
    assert_eq!(
        cc4,
        Some(CacheControl::Ephemeral5m),
        "Second-to-last non-system should get Ephemeral5m"
    );
    assert_eq!(
        cc5,
        Some(CacheControl::Ephemeral5m),
        "Last non-system should get Ephemeral5m"
    );
    // Middle messages should NOT have cache control
    let cc2 = extract_cache_control(&msgs[2]);
    assert!(
        cc2.is_none(),
        "Middle messages should not have cache control"
    );
}

// ---------------------------------------------------------------------------
// dashscope_needs_enable_thinking tests
// ---------------------------------------------------------------------------

/// DashScope model + non-Off effort → true.
#[test]
fn test_dashscope_needs_enable_thinking() {
    assert!(
        dashscope_needs_enable_thinking(
            AgentProviderApiType::OpenAi,
            "https://dashscope.aliyuncs.com/compatible-mode/v1/",
            "qwen3-235b-a22b",
            R::High,
        ),
        "DashScope + qwen3 + High effort should trigger enable_thinking"
    );
    // Negative case: non-DashScope URL
    assert!(
        !dashscope_needs_enable_thinking(
            AgentProviderApiType::OpenAi,
            "https://api.openai.com/v1/",
            "qwen3-30b",
            R::High,
        ),
        "Non-DashScope URL should not trigger"
    );
    // Negative case: Off effort
    assert!(
        !dashscope_needs_enable_thinking(
            AgentProviderApiType::OpenAi,
            "https://dashscope.aliyuncs.com/compatible-mode/v1/",
            "qwen3-30b",
            R::Off,
        ),
        "Off effort should not trigger"
    );
}
