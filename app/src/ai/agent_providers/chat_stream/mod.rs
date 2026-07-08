#![allow(dead_code, unused_imports, unused_variables)]
//! BYOP 模式下 chat completion + tool calling 适配层(基于 genai 0.5.3)。
//!
//! 把 `RequestParams` 翻译为 genai `ChatRequest`,通过 `Client::exec_chat_stream`
//! 调用用户配置的 provider,响应翻译回 `warp_multi_agent_api::ResponseEvent`,
//! controller 自家逻辑(权限/弹窗/执行/result 回写/触发下一轮)接管闭环。
//!
//! ## 5 种 API 协议显式路由
//!
//! 不再把所有 provider 当作 OpenAI 兼容硬塞,通过 `ServiceTargetResolver` 把
//! 用户在 settings UI 选的 `AgentProviderApiType` 一对一映射到 genai 的 `AdapterKind`:
//!
//! | ApiType        | AdapterKind  | 默认 endpoint                                  |
//! |----------------|--------------|------------------------------------------------|
//! | OpenAi         | OpenAI       | https://api.openai.com/v1                      |
//! | OpenAiResp     | OpenAIResp   | https://api.openai.com/v1 (走 /v1/responses)   |
//! | Gemini         | Gemini       | https://generativelanguage.googleapis.com/v1beta |
//! | Anthropic      | Anthropic    | https://api.anthropic.com                      |
//! | Ollama         | Ollama       | http://localhost:11434                         |
//!
//! 用户填的 `base_url` 始终覆盖默认。这样:
//! - DeepSeek / SiliconFlow / OpenRouter 等 OpenAI 兼容 provider 选 `OpenAi`,自定义 base_url
//! - 显式选定 adapter 完全绕过 genai 的"按模型名识别"默认行为,避免误识别
//!
//! ## 多轮 message 转换
//!
//! - system prompt: `ChatRequest::with_system()`(不进 messages 数组)
//! - user query: `ChatMessage::user(text)`
//! - assistant text: `ChatMessage::assistant(text)`
//! - assistant tool_calls: `ChatMessage::from(Vec<ToolCall>)`(自动 assistant role)
//! - tool result: `ChatMessage::from(ToolResponse::new(call_id, content))`(自动 tool role)
//!
//! ## 流式实现
//!
//! `Client::exec_chat_stream` 返回 `ChatStreamResponse`,其 `stream` 字段实现了
//! `futures_core::Stream<Item = Result<ChatStreamEvent>>`。事件:
//! - `Start` / `Chunk(text)` / `ReasoningChunk(text)` / `ToolCallChunk(tool_call)` / `End(StreamEnd)`
//!
//! 我们对 Chunk/ReasoningChunk 立即 emit `AppendToMessageContent`(打字机效果),
//! 对 ToolCallChunk 累积 buffer(按 call_id),流末统一 emit `Message::ToolCall`,
//! controller 自动接管。

use std::collections::{HashMap, HashSet};
use std::sync::Arc;

use futures::StreamExt;
use instant::Instant;
use serde_json::{json, Value};
use uuid::Uuid;
use warp_multi_agent_api as api;

use genai::chat::{
    Binary, BinarySource, CacheControl, ChatMessage, ChatOptions, ChatRequest, ChatRole,
    ChatStreamEvent, ContentPart, MessageContent, Tool as GenaiTool, ToolCall, ToolResponse,
};

use crate::ai::agent::api::{RequestParams, ResponseStream};
use crate::ai::agent::{AIAgentActionResult, AIAgentInput, RunningCommand, UserQueryMode};
use crate::ai::api_error::AIApiError;
use crate::ai::byop_compaction;
use crate::ai::byop_readiness::{
    classify_projection, AcceptedRepair, BlockedByopReadinessError, LiveToolCall,
    LiveToolCallState, ProjectedToolCall, ProjectedToolResult, ProjectionItem, ReadinessCategory,
    ReadinessContext, ReadinessDiagnosticCoalescer, ReadinessDiagnosticContext,
    ReadinessDiagnosticLevel, ReadinessReport, ReadinessState, ReadinessTriggerLayer,
    RedactedToolKind, RepairSource, RepairStateStatus, TerminalResultKind, ToolCallKey,
    ToolCallRef, ToolResultSource,
};
use crate::settings::AgentProviderApiType;
use ai::agent::convert::ConvertToAPITypeError;

use super::openai_compatible::OpenAiCompatibleError;
use super::tools;

// ---------------------------------------------------------------------------
// System prompt
// ---------------------------------------------------------------------------
// system prompt 由 `prompt_renderer::render_system` 通过 minijinja 模板生成,
// 按 LLMId 模型族选 system/{anthropic,gpt,beast,gemini,kimi,codex,trinity,default}.j2,
// 并把 warp 客户端已经收集好的 AIAgentContext(env / git / skills / project_rules / codebase / current_time)
// 渲染进 system,让 BYOP 路径也能拥有跟 warp 自家路径相当的环境信息。

use super::attachment_caps;
use super::prompt_renderer;
use super::user_context;
use crate::ai::agent::AIAgentContext;

pub(crate) mod context;
pub(crate) mod client;
pub(crate) mod diagnostics;
pub(crate) mod events;
pub(crate) mod options;
pub(crate) mod serialization;

pub(crate) use client::{adapter_kind_for, normalize_endpoint_url, build_user_agent_header, map_genai_error, build_client};
use context::{
    latest_input_context, render_lrc_request_context, render_running_command_context,
    render_running_command_id_context, render_ssh_session_block, xml_attr, xml_text,
};
use serialization::{
    flush_assistant_buffer, build_user_message_with_binaries, collect_linearized_task_messages,
    mime_to_modality, serialize_outgoing_tool_call,
    AssistantBuffer, OutboundAssistantToolGroup, REASONING_ECHO_PLACEHOLDER,
};
#[cfg(test)]
pub(super) use serialization::serialize_outgoing_tool_call_for_test;
pub use options::available_tool_names;
use options::{
    apply_caching_anthropic, build_chat_options, build_tools_array, dashscope_needs_enable_thinking,
    is_plan_mode_turn, PLAN_MODE_BLOCKED_TOOLS,
};
use events::{
    make_add_messages_event, make_update_message_event, make_append_event, AppendKind,
    make_reasoning_message, make_agent_output_message, make_user_query_message,
    make_web_search_searching_message, extract_search_pages_from_exa_results,
    make_web_search_status_from_result, make_web_fetch_fetching_message,
    make_web_fetch_status_from_result, make_tool_call_result_message,
    make_tool_call_carrier_message, make_tool_call_message, create_task_event,
    create_subtask_event, make_finished_done,
};

struct SerializerProjectionBuilder {
    items: Vec<ProjectionItem>,
    pending_tool_calls: Vec<ProjectedToolCall>,
    pending_task_id: Option<String>,
    pending_assistant_message_id: Option<String>,
    skipped_tool_results: HashSet<(String, String)>,
}

impl SerializerProjectionBuilder {
    fn new() -> Self {
        Self {
            items: Vec::new(),
            pending_tool_calls: Vec::new(),
            pending_task_id: None,
            pending_assistant_message_id: None,
            skipped_tool_results: HashSet::new(),
        }
    }

    fn push_user_boundary(&mut self, task_id: String, message_id: String) {
        self.flush_tool_calls();
        self.items
            .push(ProjectionItem::user_boundary(task_id, message_id));
    }

    fn push_assistant_boundary(&mut self, task_id: String, message_id: String) {
        self.flush_tool_calls();
        self.items
            .push(ProjectionItem::assistant_boundary(task_id, message_id));
    }

    fn push_tool_call(
        &mut self,
        task_id: &str,
        message_id: &str,
        tool_call: &api::message::ToolCall,
    ) {
        use crate::ai::agent::task::helper::ToolCallExt;

        if tool_call.subagent().is_some() {
            self.skipped_tool_results
                .insert((task_id.to_owned(), tool_call.tool_call_id.clone()));
            return;
        }

        if self
            .pending_task_id
            .as_deref()
            .is_some_and(|pending_task_id| pending_task_id != task_id)
        {
            self.flush_tool_calls();
        }

        if self.pending_tool_calls.is_empty() {
            self.pending_task_id = Some(task_id.to_owned());
            self.pending_assistant_message_id = Some(message_id.to_owned());
        }

        let assistant_message_id = self
            .pending_assistant_message_id
            .clone()
            .unwrap_or_else(|| message_id.to_owned());
        self.pending_tool_calls.push(ProjectedToolCall::new(
            task_id,
            assistant_message_id,
            tool_call.tool_call_id.clone(),
            redacted_tool_kind_for_tool_call(tool_call),
        ));
    }

    fn push_tool_result(&mut self, result: ProjectedToolResult) {
        self.flush_tool_calls();
        self.items.push(ProjectionItem::tool_result(result));
    }

    fn should_skip_tool_result(&self, task_id: &str, tool_call_id: &str) -> bool {
        self.skipped_tool_results
            .contains(&(task_id.to_owned(), tool_call_id.to_owned()))
    }

    fn finish(mut self) -> Vec<ProjectionItem> {
        self.flush_tool_calls();
        self.items
    }

    fn flush_tool_calls(&mut self) {
        if self.pending_tool_calls.is_empty() {
            return;
        }

        let task_id = self.pending_task_id.take().unwrap_or_default();
        let assistant_message_id = self.pending_assistant_message_id.take().unwrap_or_default();
        self.items.push(ProjectionItem::assistant_tool_calls(
            task_id,
            assistant_message_id,
            std::mem::take(&mut self.pending_tool_calls),
        ));
    }
}

fn redacted_tool_kind_for_tool_call(tool_call: &api::message::ToolCall) -> RedactedToolKind {
    use crate::ai::agent::task::helper::ToolExt;

    tool_call
        .tool
        .as_ref()
        .map(|tool| RedactedToolKind::new(tool.name()))
        .unwrap_or_default()
}

fn current_input_result_kind(result: &AIAgentActionResult) -> TerminalResultKind {
    if result.result.is_cancelled() {
        TerminalResultKind::Cancellation
    } else {
        TerminalResultKind::Real
    }
}

fn persisted_tool_result_kind(
    msg: &api::Message,
    compacted_tool_msg_ids: Option<&std::collections::HashSet<String>>,
) -> TerminalResultKind {
    if compacted_tool_msg_ids.is_some_and(|ids| ids.contains(&msg.id)) {
        return TerminalResultKind::Compacted;
    }

    let Some(api::message::Message::ToolCallResult(tool_call_result)) = msg.message.as_ref() else {
        return TerminalResultKind::Real;
    };
    if tool_call_result.result.is_some() {
        return TerminalResultKind::Real;
    }

    let content = msg.server_message_data.trim();
    if content.is_empty() {
        return TerminalResultKind::Real;
    }

    match serde_json::from_str::<Value>(content) {
        Ok(value) => {
            if value
                .get("_byop_intercepted")
                .and_then(Value::as_bool)
                .unwrap_or(false)
            {
                TerminalResultKind::LocalInterception
            } else if value.get("error").and_then(Value::as_str) == Some("invalid_arguments") {
                TerminalResultKind::StructuredError
            } else {
                TerminalResultKind::Real
            }
        }
        Err(_)
            if content.contains("_byop_intercepted") || content.contains("invalid_arguments") =>
        {
            TerminalResultKind::UnreadableLocalInterception
        }
        Err(_) => TerminalResultKind::Real,
    }
}

fn current_input_task_id(params: &RequestParams) -> String {
    params
        .byop_target_task_id
        .clone()
        .or_else(|| params.tasks.first().map(|task| task.id.clone()))
        .unwrap_or_else(|| "current_input".to_owned())
}

fn build_serializer_readiness_projection(
    params: &RequestParams,
    all_msgs: &[&api::Message],
    summarize_head_end: Option<usize>,
    summary_inserts: &HashMap<String, String>,
    hidden_msg_ids: &std::collections::HashSet<String>,
    compacted_tool_msg_ids: &std::collections::HashSet<String>,
) -> Vec<ProjectionItem> {
    let mut builder = SerializerProjectionBuilder::new();

    for (idx, msg) in all_msgs.iter().enumerate() {
        if let Some(head_end) = summarize_head_end {
            if idx >= head_end {
                continue;
            }
        }

        if hidden_msg_ids.contains(&msg.id) {
            if summary_inserts.contains_key(&msg.id) {
                builder.push_user_boundary(msg.task_id.clone(), format!("summary_user:{}", msg.id));
                builder.push_assistant_boundary(
                    msg.task_id.clone(),
                    format!("summary_assistant:{}", msg.id),
                );
            }
            continue;
        }

        let Some(inner) = &msg.message else {
            continue;
        };

        match inner {
            api::message::Message::UserQuery(_) => {
                builder.push_user_boundary(msg.task_id.clone(), msg.id.clone());
            }
            api::message::Message::AgentOutput(_) => {
                builder.push_assistant_boundary(msg.task_id.clone(), msg.id.clone());
            }
            api::message::Message::AgentReasoning(_) => {}
            api::message::Message::ToolCall(tool_call) => {
                builder.push_tool_call(&msg.task_id, &msg.id, tool_call);
            }
            api::message::Message::ToolCallResult(tool_call_result) => {
                if builder.should_skip_tool_result(&msg.task_id, &tool_call_result.tool_call_id) {
                    continue;
                }
                builder.push_tool_result(ProjectedToolResult::new(
                    msg.task_id.clone(),
                    msg.id.clone(),
                    None,
                    tool_call_result.tool_call_id.clone(),
                    RedactedToolKind::default(),
                    ToolResultSource::PersistedHistory,
                    persisted_tool_result_kind(msg, Some(compacted_tool_msg_ids)),
                ));
            }
            api::message::Message::ServerEvent(_)
            | api::message::Message::SystemQuery(_)
            | api::message::Message::UpdateTodos(_)
            | api::message::Message::Summarization(_)
            | api::message::Message::CodeReview(_)
            | api::message::Message::UpdateReviewComments(_)
            | api::message::Message::WebSearch(_)
            | api::message::Message::WebFetch(_)
            | api::message::Message::DebugOutput(_)
            | api::message::Message::ArtifactEvent(_)
            | api::message::Message::InvokeSkill(_)
            | api::message::Message::MessagesReceivedFromAgents(_)
            | api::message::Message::ModelUsed(_)
            | api::message::Message::EventsFromAgents(_)
            | api::message::Message::PassiveSuggestionResult(_)
            | api::message::Message::OrchestrationConfigSnapshot(_) => {}
        }
    }

    let current_task_id = current_input_task_id(params);
    for (idx, input) in params.input.iter().enumerate() {
        match input {
            AIAgentInput::UserQuery { .. }
            | AIAgentInput::InvokeSkill { .. }
            | AIAgentInput::ResumeConversation { .. }
            | AIAgentInput::SummarizeConversation { .. } => {
                builder.push_user_boundary(
                    current_task_id.clone(),
                    format!("current_input:{idx}:user"),
                );
            }
            AIAgentInput::ActionResult { result, .. } => {
                let tool_call_id = result.id.to_string();
                builder.push_tool_result(ProjectedToolResult::new(
                    result.task_id.to_string(),
                    format!("current_input:{idx}:{tool_call_id}"),
                    None,
                    tool_call_id,
                    RedactedToolKind::default(),
                    ToolResultSource::CurrentInput,
                    current_input_result_kind(result),
                ));
            }
            AIAgentInput::AutoCodeDiffQuery { .. }
            | AIAgentInput::InitProjectRules { .. }
            | AIAgentInput::TriggerPassiveSuggestion { .. }
            | AIAgentInput::CreateNewProject { .. }
            | AIAgentInput::CloneRepository { .. }
            | AIAgentInput::CodeReview { .. }
            | AIAgentInput::FetchReviewComments { .. }
            | AIAgentInput::StartFromAmbientRunPrompt { .. }
            | AIAgentInput::MessagesReceivedFromAgents { .. }
            | AIAgentInput::EventsFromAgents { .. }
            | AIAgentInput::PassiveSuggestionResult { .. }
            | AIAgentInput::CreateEnvironment { .. }
            | AIAgentInput::OrchestrationConfigUpdate { .. } => {}
        }
    }

    builder.finish()
}

pub(crate) fn classify_byop_controller_readiness(params: &RequestParams) -> ReadinessReport {
    classify_byop_controller_readiness_with_live_tool_calls(params, Vec::new())
}

pub(crate) fn classify_byop_controller_readiness_with_live_tool_calls(
    params: &RequestParams,
    live_tool_calls: Vec<LiveToolCall>,
) -> ReadinessReport {
    let skipped_cancellation_results = current_input_cancellation_result_keys(params);
    let projection = build_controller_readiness_projection(params, &skipped_cancellation_results);
    let mut live_tool_calls_for_context =
        cancellation_live_tool_calls(params, &skipped_cancellation_results);
    live_tool_calls_for_context.extend(live_tool_calls);
    let context = ReadinessContext {
        repair_records: params.byop_repair_state.repair_records().to_vec(),
        live_tool_calls: live_tool_calls_for_context,
    };
    classify_projection(&projection, &context)
}

fn current_input_cancellation_result_keys(params: &RequestParams) -> HashSet<(String, String)> {
    params
        .input
        .iter()
        .filter_map(|input| {
            let AIAgentInput::ActionResult { result, .. } = input else {
                return None;
            };
            result
                .result
                .is_cancelled()
                .then(|| (result.task_id.to_string(), result.id.to_string()))
        })
        .collect()
}

fn cancellation_live_tool_calls(
    params: &RequestParams,
    skipped_cancellation_results: &HashSet<(String, String)>,
) -> Vec<LiveToolCall> {
    use crate::ai::agent::task::helper::ToolCallExt;

    params
        .tasks
        .iter()
        .flat_map(|task| task.messages.iter())
        .filter_map(|msg| {
            let api::message::Message::ToolCall(tool_call) = msg.message.as_ref()? else {
                return None;
            };
            if tool_call.subagent().is_some() {
                return None;
            }
            if !skipped_cancellation_results
                .contains(&(msg.task_id.clone(), tool_call.tool_call_id.clone()))
            {
                return None;
            }
            Some(LiveToolCall::new(
                ToolCallRef::new(
                    ToolCallKey::new(&msg.task_id, &msg.id, &tool_call.tool_call_id),
                    redacted_tool_kind_for_tool_call(tool_call),
                ),
                LiveToolCallState::CancellationRequested,
            ))
        })
        .collect()
}

fn build_controller_readiness_projection(
    params: &RequestParams,
    skipped_current_action_results: &HashSet<(String, String)>,
) -> Vec<ProjectionItem> {
    let mut builder = SerializerProjectionBuilder::new();

    for msg in params.tasks.iter().flat_map(|task| task.messages.iter()) {
        let Some(inner) = &msg.message else {
            continue;
        };

        match inner {
            api::message::Message::UserQuery(_) => {
                builder.push_user_boundary(msg.task_id.clone(), msg.id.clone());
            }
            api::message::Message::AgentOutput(_) => {
                builder.push_assistant_boundary(msg.task_id.clone(), msg.id.clone());
            }
            api::message::Message::AgentReasoning(_) => {}
            api::message::Message::ToolCall(tool_call) => {
                builder.push_tool_call(&msg.task_id, &msg.id, tool_call);
            }
            api::message::Message::ToolCallResult(tool_call_result) => {
                if builder.should_skip_tool_result(&msg.task_id, &tool_call_result.tool_call_id) {
                    continue;
                }
                builder.push_tool_result(ProjectedToolResult::new(
                    msg.task_id.clone(),
                    msg.id.clone(),
                    None,
                    tool_call_result.tool_call_id.clone(),
                    RedactedToolKind::default(),
                    ToolResultSource::PersistedHistory,
                    persisted_tool_result_kind(msg, None),
                ));
            }
            api::message::Message::ServerEvent(_)
            | api::message::Message::SystemQuery(_)
            | api::message::Message::UpdateTodos(_)
            | api::message::Message::Summarization(_)
            | api::message::Message::CodeReview(_)
            | api::message::Message::UpdateReviewComments(_)
            | api::message::Message::WebSearch(_)
            | api::message::Message::WebFetch(_)
            | api::message::Message::DebugOutput(_)
            | api::message::Message::ArtifactEvent(_)
            | api::message::Message::InvokeSkill(_)
            | api::message::Message::MessagesReceivedFromAgents(_)
            | api::message::Message::ModelUsed(_)
            | api::message::Message::EventsFromAgents(_)
            | api::message::Message::PassiveSuggestionResult(_)
            | api::message::Message::OrchestrationConfigSnapshot(_) => {}
        }
    }

    let current_task_id = current_input_task_id(params);
    for (idx, input) in params.input.iter().enumerate() {
        match input {
            AIAgentInput::UserQuery { .. }
            | AIAgentInput::InvokeSkill { .. }
            | AIAgentInput::ResumeConversation { .. }
            | AIAgentInput::SummarizeConversation { .. } => {
                builder.push_user_boundary(
                    current_task_id.clone(),
                    format!("current_input:{idx}:user"),
                );
            }
            AIAgentInput::ActionResult { result, .. } => {
                let tool_call_id = result.id.to_string();
                if skipped_current_action_results
                    .contains(&(result.task_id.to_string(), tool_call_id.clone()))
                {
                    continue;
                }
                builder.push_tool_result(ProjectedToolResult::new(
                    result.task_id.to_string(),
                    format!("current_input:{idx}:{tool_call_id}"),
                    None,
                    tool_call_id,
                    RedactedToolKind::default(),
                    ToolResultSource::CurrentInput,
                    current_input_result_kind(result),
                ));
            }
            AIAgentInput::AutoCodeDiffQuery { .. }
            | AIAgentInput::InitProjectRules { .. }
            | AIAgentInput::TriggerPassiveSuggestion { .. }
            | AIAgentInput::CreateNewProject { .. }
            | AIAgentInput::CloneRepository { .. }
            | AIAgentInput::CodeReview { .. }
            | AIAgentInput::FetchReviewComments { .. }
            | AIAgentInput::StartFromAmbientRunPrompt { .. }
            | AIAgentInput::MessagesReceivedFromAgents { .. }
            | AIAgentInput::EventsFromAgents { .. }
            | AIAgentInput::PassiveSuggestionResult { .. }
            | AIAgentInput::CreateEnvironment { .. }
            | AIAgentInput::OrchestrationConfigUpdate { .. } => {}
        }
    }

    builder.finish()
}

fn validate_byop_serializer_readiness(
    params: &RequestParams,
    all_msgs: &[&api::Message],
    summarize_head_end: Option<usize>,
    summary_inserts: &HashMap<String, String>,
    hidden_msg_ids: &std::collections::HashSet<String>,
    compacted_tool_msg_ids: &std::collections::HashSet<String>,
) -> Result<ReadinessReport, ConvertToAPITypeError> {
    let projection = build_serializer_readiness_projection(
        params,
        all_msgs,
        summarize_head_end,
        summary_inserts,
        hidden_msg_ids,
        compacted_tool_msg_ids,
    );
    let conversation_id = params
        .byop_conversation_id
        .map(|id| id.to_string())
        .unwrap_or_else(|| "unknown".to_owned());
    let request_attempt_id = params
        .byop_readiness_attempt_id
        .clone()
        .unwrap_or_else(|| "serializer-unknown".to_owned());
    validate_serializer_readiness_projection_with_repair_state(
        projection,
        &params.byop_repair_state,
        &ReadinessDiagnosticContext::new(
            &conversation_id,
            &request_attempt_id,
            ReadinessTriggerLayer::SerializerValidation,
        ),
    )
}

fn validate_serializer_readiness_projection(
    projection: Vec<ProjectionItem>,
) -> Result<ReadinessReport, ConvertToAPITypeError> {
    validate_serializer_readiness_projection_with_repair_state(
        projection,
        &RepairStateStatus::default(),
        &ReadinessDiagnosticContext::new(
            "test-conversation",
            "test-attempt",
            ReadinessTriggerLayer::SerializerValidation,
        ),
    )
}

fn validate_serializer_readiness_projection_with_repair_state(
    projection: Vec<ProjectionItem>,
    repair_state: &RepairStateStatus,
    diagnostic_context: &ReadinessDiagnosticContext<'_>,
) -> Result<ReadinessReport, ConvertToAPITypeError> {
    let context = ReadinessContext {
        repair_records: repair_state.repair_records().to_vec(),
        live_tool_calls: Vec::new(),
    };
    let report = classify_projection(&projection, &context);

    match &report.state {
        ReadinessState::Ready => {
            if let Some(error_category) = repair_state.error_category() {
                log::error!(
                    "[byop-readiness] serializer continuing with invalid repair sidecar \
                     category={error_category:?} projection_items={}",
                    projection.len()
                );
            }
            Ok(report)
        }
        ReadinessState::AcceptedHistoryRepair { repairs } => {
            log_accepted_history_repair(repairs, diagnostic_context);
            Ok(report)
        }
        ReadinessState::PendingToolResults { .. }
        | ReadinessState::NeedsCancellationCommit { .. }
        | ReadinessState::DuplicateToolResults { .. }
        | ReadinessState::OrphanToolResult { .. }
        | ReadinessState::OutOfOrderToolResult { .. }
        | ReadinessState::MissingResultWithoutRepairSource { .. } => {
            let category = report.state.category();
            let mut diagnostics = ReadinessDiagnosticCoalescer::default();
            diagnostics.log_state(
                &report.state,
                diagnostic_context,
                ReadinessDiagnosticLevel::Error,
            );
            diagnostics.finish(diagnostic_context, ReadinessDiagnosticLevel::Error);
            log::error!(
                "[byop-readiness] serializer blocked request category={category:?} \
                 projection_items={} ignored_repair_records={} trigger_layer=serializer_validation \
                 request_attempt_id={}",
                projection.len(),
                report.ignored_repair_records.len(),
                diagnostic_context.request_attempt_id
            );

            Err(ConvertToAPITypeError::Other(
                BlockedByopReadinessError::new(category).into(),
            ))
        }
    }
}

fn log_accepted_history_repair(
    repairs: &[AcceptedRepair],
    diagnostic_context: &ReadinessDiagnosticContext<'_>,
) {
    log::info!(
        "{}",
        accepted_history_repair_log_message(repairs, diagnostic_context)
    );
}

fn accepted_history_repair_log_message(
    repairs: &[AcceptedRepair],
    diagnostic_context: &ReadinessDiagnosticContext<'_>,
) -> String {
    let forked_history_count = repairs
        .iter()
        .filter(|repair| matches!(repair.record.source, RepairSource::ForkedHistory))
        .count();
    let restored_legacy_history_count = repairs
        .iter()
        .filter(|repair| matches!(repair.record.source, RepairSource::RestoredLegacyHistory))
        .count();

    format!(
        "[byop-readiness] serializer accepted history repair records={} \
         category={:?} forked_history={} restored_legacy_history={} conversation_id={} \
         trigger_layer=serializer_validation request_attempt_id={} repair_keys={:?}",
        repairs.len(),
        ReadinessCategory::AcceptedHistoryRepair,
        forked_history_count,
        restored_legacy_history_count,
        diagnostic_context.conversation_id,
        diagnostic_context.request_attempt_id,
        repairs
            .iter()
            .map(|repair| format!(
                "task_id={} assistant_tool_call_message_id={} tool_call_id={} redacted_tool_kind={}",
                repair.tool_call.key.task_id,
                repair.tool_call.key.assistant_tool_call_message_id,
                repair.tool_call.key.tool_call_id,
                repair.tool_call.redacted_tool_kind.as_str()
            ))
            .collect::<Vec<_>>()
    )
}

/// 把 RequestParams 翻译为 genai `ChatRequest`(含 system + messages + tools)。
///
/// `force_echo_reasoning`:由 `super::reasoning::model_requires_reasoning_echo`
/// 决定。true 时所有 assistant message 强制挂 reasoning_content(空串占位),
/// 修复 DeepSeek-v4-flash / Kimi 等收紧校验的 thinking-mode endpoint。
fn build_chat_request(
    params: &RequestParams,
    force_echo_reasoning: bool,
    api_type: AgentProviderApiType,
    attachment_caps: attachment_caps::AttachmentCaps,
) -> Result<ChatRequest, ConvertToAPITypeError> {
    let agent_ctx = latest_input_context(&params.input);
    let plan_mode = is_plan_mode_turn(&params.input);
    let tool_names = available_tool_names(params);
    let mut system_text = prompt_renderer::render_system(
        &params.model,
        agent_ctx,
        &tool_names,
        plan_mode,
        &params.user_rules,
    );
    // Marb:legacy SSH 会话画像补丁。`render_system` 走 AIAgentContext,
    // 拿到的 OS/shell 是本地客户端;legacy SSH 下 PTY 实际在远端,
    // 追加一段 SSH 状态块矫正 LLM 推断。
    if let Some(ssh_block) = render_ssh_session_block(&params.session_context) {
        system_text.push_str(&ssh_block);
    }
    // 注:LRC / 长命令的工具用法引导(write_to_long_running_shell_command + command_id +
    // 各种 mode 与 raw 字节序列)已经在 `prompts/system/default.j2:69-79` 完整覆盖。
    // 用户当前所处的具体 PTY 上下文(命令名 / alt-screen 标志 / grid 内容)通过
    // user message 前缀的 `<attached_running_command>` XML 块单独注入(见
    // `render_running_command_context` 与 build_chat_request 中的 UserQuery 分支)。
    // 不在 system 这层重复硬编码 TUI 退出键之类,避免与 default.j2 的标准引导冲突或冗余。

    let mut messages: Vec<ChatMessage> = Vec::new();
    let mut outbound_tool_groups: Vec<OutboundAssistantToolGroup> = Vec::new();

    // 收集所有 task 的 messages,经 `collect_linearized_task_messages` 做确定性
    // DFS 线性化 + UserQuery 去重(修复 Issue #94 —— 历史轮 user 消息被乱序排到
    // 末尾、或 LRC subagent 副本导致重复)。详见该函数文档。
    let all_msgs: Vec<&api::Message> = collect_linearized_task_messages(&params.tasks);

    // Marb BYOP 本地会话压缩:把 conversation.compaction_state 应用到 message 序列。
    //   1. 过滤已被某次压缩覆盖的 (user, assistant) 对(`hidden_message_ids`)
    //   2. 在被隐去区间的位置插入一对合成的 (user "已压缩,以下为摘要" + assistant 摘要文本) message —
    //      这一步通过 `summary_inserts` 索引在主 loop 里就近 emit
    //   3. ToolCallResult 的 marker.tool_output_compacted_at 不为空时,后面分支替换 content 为占位符
    //
    // 当前 input 是 `AIAgentInput::SummarizeConversation` 时:进一步用 select 算法把 messages
    // 切到 head(去掉 tail),最后 input loop 末尾会追加 `build_prompt(...)` 作为 user message
    // (走完整的 SUMMARY_TEMPLATE),让上游 LLM 输出结构化摘要。
    let is_summarization_request = params
        .input
        .iter()
        .any(|i| matches!(i, AIAgentInput::SummarizeConversation { .. }));
    let summarization_overflow = params.input.iter().any(|i| {
        matches!(
            i,
            AIAgentInput::SummarizeConversation { .. }
        )
    });
    let _ = summarization_overflow; // 当前在 input loop 内的 follow-up 文案分支会用,目前先 silence dead

    let summary_inserts: std::collections::HashMap<String, String> =
        if let Some(state) = params.compaction_state.as_ref() {
            // user_msg_id → summary_text;遇到该 user_msg_id 时(它本来要被 hidden)替换为合成的摘要对
            state
                .completed()
                .iter()
                .filter_map(|c| {
                    c.summary_text
                        .as_ref()
                        .map(|s| (c.user_msg_id.clone(), s.clone()))
                })
                .collect()
        } else {
            std::collections::HashMap::new()
        };
    let hidden_msg_ids: std::collections::HashSet<String> = params
        .compaction_state
        .as_ref()
        .map(|s| s.hidden_message_ids())
        .unwrap_or_default();
    let compacted_tool_msg_ids: std::collections::HashSet<String> = params
        .compaction_state
        .as_ref()
        .map(|s| {
            // 收集所有标记了 tool_output_compacted_at 的 ToolCallResult message_ids
            // 通过遍历 all_msgs 并查 marker 实现
            let mut out = std::collections::HashSet::new();
            for msg in &all_msgs {
                if let Some(api::message::Message::ToolCallResult(_)) = &msg.message {
                    if s.marker(&msg.id)
                        .and_then(|m| m.tool_output_compacted_at)
                        .is_some()
                    {
                        out.insert(msg.id.clone());
                    }
                }
            }
            out
        })
        .unwrap_or_default();

    // 摘要请求路径:用 byop_compaction::algorithm::select 切 head;tail 不送上游
    let summarize_head_end: Option<usize> = if is_summarization_request {
        // 临时投影成 WarpMessageView 算 select
        let state_for_select = params.compaction_state.clone().unwrap_or_default();
        let tool_names =
            byop_compaction::message_view::build_tool_name_lookup(all_msgs.iter().copied());
        let views =
            byop_compaction::message_view::project(&all_msgs, &state_for_select, &tool_names);
        let cfg = byop_compaction::CompactionConfig::default();
        let model_limit = byop_compaction::overflow::ModelLimit::FALLBACK;
        let result = byop_compaction::algorithm::select(&views, &cfg, model_limit, |slice| {
            slice
                .iter()
                .map(byop_compaction::algorithm::MessageRef::estimate_size)
                .sum()
        });
        // head_end 是 views 里"head 区间"上界,与 all_msgs 同序
        Some(result.head_end)
    } else {
        None
    };

    let readiness_report = validate_byop_serializer_readiness(
        params,
        &all_msgs,
        summarize_head_end,
        &summary_inserts,
        &hidden_msg_ids,
        &compacted_tool_msg_ids,
    )?;

    let mut buf = AssistantBuffer::new(force_echo_reasoning);
    // Marb:历史里被 skip 掉的 subagent ToolCall 对应的 call_id —— 它们的
    // ToolCallResult 也必须 skip,否则会成为孤儿 tool_response,Anthropic 直接 400
    // `unexpected tool_use_id ... no corresponding tool_use block`。
    let mut skipped_subagent_call_ids: std::collections::HashSet<String> =
        std::collections::HashSet::new();

    for (idx, msg) in all_msgs.iter().enumerate() {
        // 摘要请求:tail 区间不送上游(只送 head + 末尾追加 SUMMARY_TEMPLATE)
        if let Some(head_end) = summarize_head_end {
            if idx >= head_end {
                continue;
            }
        }
        if hidden_msg_ids.contains(&msg.id) {
            if let Some(summary_text) = summary_inserts.get(&msg.id) {
                flush_assistant_buffer(&mut buf, &mut messages, &mut outbound_tool_groups);
                messages.push(ChatMessage::user(
                    "Conversation history was compacted. Below is the structured summary of all prior turns.".to_string(),
                ));
                messages.push(ChatMessage::assistant(summary_text.clone()));
            }
            continue;
        }
        let Some(inner) = &msg.message else {
            continue;
        };
        match inner {
            api::message::Message::UserQuery(u) => {
                flush_assistant_buffer(&mut buf, &mut messages, &mut outbound_tool_groups);
                // Marb:历史轮多模态保活。warp 自家路径靠云端 server 重注入 InputContext,
                // BYOP 直连没有那层,所以 `make_user_query_message` 持久化时把所有 binary
                // (image / pdf / audio)塞进了 `UserQuery.context.images`,这里反向恢复成
                // UserBinary 走 `build_user_message_with_binaries`,使后续轮模型仍能看到先前
                // 粘贴的多模态附件。模型 caps 不支持的 mime 由 build_user_message_with_binaries
                // 替换为 ERROR 文本(opencode unsupportedParts 风格),不会静默 drop。
                // 没有 binary → 退回老路 `ChatMessage::user(text)`,与修复前等价。
                let history_binaries: Vec<user_context::UserBinary> = u
                    .context
                    .as_ref()
                    .map(|ctx| {
                        ctx.images
                            .iter()
                            .filter(|b| !b.data.is_empty())
                            .enumerate()
                            .map(|(idx, b)| {
                                use base64::Engine;
                                user_context::UserBinary {
                                    name: format!("history-attachment-{}-{idx}", &msg.id),
                                    content_type: if b.mime_type.is_empty() {
                                        "application/octet-stream".to_string()
                                    } else {
                                        b.mime_type.clone()
                                    },
                                    data: base64::engine::general_purpose::STANDARD.encode(&b.data),
                                }
                            })
                            .collect()
                    })
                    .unwrap_or_default();
                let mut history_prefixes: Vec<String> = Vec::new();
                if let Some(prefix) =
                    user_context::render_api_referenced_attachments(&u.referenced_attachments)
                {
                    history_prefixes.push(prefix);
                }
                let history_text = if history_prefixes.is_empty() {
                    u.query.clone()
                } else {
                    format!("{}\n\n{}", history_prefixes.join("\n\n"), u.query)
                };
                if history_binaries.is_empty() {
                    messages.push(ChatMessage::user(history_text));
                } else {
                    messages.push(build_user_message_with_binaries(
                        history_text,
                        history_binaries,
                        attachment_caps,
                    ));
                }
            }
            api::message::Message::AgentReasoning(r) => {
                // 把上一轮的 reasoning 挂到下一个要 flush 的 assistant message 上。
                // genai 0.6 的 with_reasoning_content 会按当前 adapter 序列化:
                // DeepSeek/Kimi → reasoning_content 字段;Anthropic → thinking blocks。
                // 多段 AgentReasoning 累加(同一 turn 可能 stream 出多个 reasoning chunk
                // 落地为多条 AgentReasoning)。
                let next = r.reasoning.clone();
                if !next.is_empty() {
                    match buf.reasoning.as_mut() {
                        Some(existing) => existing.push_str(&next),
                        None => buf.reasoning = Some(next),
                    }
                }
            }
            api::message::Message::AgentOutput(a) => {
                if buf.text.is_some() || !buf.tool_calls.is_empty() {
                    flush_assistant_buffer(&mut buf, &mut messages, &mut outbound_tool_groups);
                }
                buf.text = Some(a.text.clone());
            }
            api::message::Message::ToolCall(tc) => {
                // Marb BYOP:**虚拟 subagent tool_call 不发给上游模型**。
                // LRC tag-in 场景下,我们在 chat_stream 流头合成 `Tool::Subagent { metadata: Cli }`
                // 写入 root.task.messages,只用于触发 conversation 创建 cli subtask + spawn 浮窗,
                // 它不是模型实际产出的工具调用,模型看到会 confused(多余 tool call + 没法回应)。
                // 同样它对应的 ToolCallResult 也要由下面分支过滤,避免出现
                // "tool_response 找不到匹配的 tool_call" 的不平衡。
                use crate::ai::agent::task::helper::ToolCallExt;
                if tc.subagent().is_some() {
                    skipped_subagent_call_ids.insert(tc.tool_call_id.clone());
                    continue;
                }
                if buf
                    .tool_call_keys
                    .first()
                    .is_some_and(|key| key.task_id != msg.task_id)
                {
                    flush_assistant_buffer(&mut buf, &mut messages, &mut outbound_tool_groups);
                }
                let (name, args_json) = serialize_outgoing_tool_call(
                    tc,
                    params.mcp_context.as_ref(),
                    &msg.server_message_data,
                );
                let assistant_message_id = buf
                    .tool_call_keys
                    .first()
                    .map(|key| key.assistant_tool_call_message_id.clone())
                    .unwrap_or_else(|| msg.id.clone());
                let key = ToolCallKey::new(&msg.task_id, assistant_message_id, &tc.tool_call_id);
                buf.push_tool_call(
                    ToolCall {
                        call_id: tc.tool_call_id.clone(),
                        fn_name: name,
                        fn_arguments: args_json,
                        thought_signatures: None,
                    },
                    key,
                );
            }
            api::message::Message::ToolCallResult(tcr) => {
                flush_assistant_buffer(&mut buf, &mut messages, &mut outbound_tool_groups);
                // Marb:对应 ToolCall 已被 skip(subagent 虚拟 call)→ result 也 skip,
                // 否则留下孤儿 tool_response 导致上游 400。
                if skipped_subagent_call_ids.contains(&tcr.tool_call_id) {
                    continue;
                }
                // BYOP 持久化的 ToolCallResult 走 server_message_data(content 已是 JSON 字符串);
                // server 端 emit 走 result oneof 结构化 variant — 兼容两路。
                let content = if compacted_tool_msg_ids.contains(&msg.id) {
                    // 压缩投影:被 prune 的 tool output 替换为占位符,不送实际内容上游
                    r#"{"status":"compacted","note":"tool output was pruned by local compaction"}"#
                        .to_string()
                } else if tcr.result.is_some() {
                    tools::serialize_result(tcr)
                } else if !msg.server_message_data.is_empty() {
                    msg.server_message_data.clone()
                } else {
                    r#"{"status":"empty"}"#.to_owned()
                };
                messages.push(ChatMessage::from(ToolResponse::new(
                    tcr.tool_call_id.clone(),
                    content,
                )));
            }
            _ => {
                // 其他 message 类型(SystemQuery/UpdateTodos/...)BYOP 暂不送上游。
            }
        }
    }
    flush_assistant_buffer(&mut buf, &mut messages, &mut outbound_tool_groups);

    // 当前轮新输入 → 追加。
    for input in &params.input {
        match input {
            AIAgentInput::UserQuery {
                query,
                context,
                referenced_attachments,
                running_command,
                ..
            } => {
                // 当前轮 UserQuery 自带的附件类 context(Block / SelectedText / File / Image)
                // 严格对齐 warp 自家路径走 `api::InputContext.executed_shell_commands` 等字段
                // 上行后由后端注入 prompt 的语义。BYOP 没有后端这层,直接 prepend 到 user message。
                // 环境型 context(env / git / skills / ...)由 prompt_renderer 渲染进 system,
                // 与本路径不重叠。
                //
                // Marb：LRC tag-in 场景下，`running_command: Some(...)` 含完整 PTY 上下文
                // （alt-screen grid_contents + command + is_alt_screen_active 标志），用
                // `render_running_command_context` 渲成 `<attached_running_command>` XML 块。
                // 模型据此决定调 write_to_long_running_shell_command。
                // 没填（普通对话或 controller 没注入）时回退到 `lrc_command_id` 简短上下文。
                //
                // **P1-10 prompt cache 优化**：LRC 上下文块**追加到 query 之后**而不
                // 是前缀。原因：
                //   - grid_contents 随 PTY 状态每秒变化，是 “高频变动” 内容。
                //   - 放到 query 前面会让 user message 头部不稳定→ messages 段末尾
                //     2 个 Anthropic breakpoint 写入的哈希总是不同，复用价值低。
                //   - 放到 query 之后，同一个 query (如 “退出 nvim”) 在不同 PTY 快照上仍
                //     共享前缀“user 问题”，提高跨调用复用可能。
                // 模型行为差别微小：指令在前还是 context 在前，模型都能正确理解。
                // user_attachments 的 prefix（如 SelectedText / Block）仍放前缀位，因为
                // 它们对应用户“明确选中”的内容，应作为问题背景而非实例补充。
                let mut suffixes: Vec<String> = Vec::new();
                let request_running_command = running_command
                    .as_ref()
                    .or(params.lrc_running_command.as_ref());
                if let Some(rc) = request_running_command {
                    suffixes.push(render_running_command_context(rc));
                } else if let Some(command_id) = params.lrc_command_id.as_deref() {
                    suffixes.push(render_running_command_id_context(command_id));
                }
                let mut prefixes: Vec<String> = Vec::new();
                let user_attachments = user_context::collect_user_attachments(context);
                if let Some(p) = &user_attachments.prefix {
                    prefixes.push(p.clone());
                }
                if let Some(p) = user_context::render_referenced_attachments(referenced_attachments)
                {
                    prefixes.push(p);
                }
                let full_text = match (prefixes.is_empty(), suffixes.is_empty()) {
                    (true, true) => query.clone(),
                    (false, true) => format!("{}\n\n{query}", prefixes.join("\n\n")),
                    (true, false) => format!("{query}\n\n{}", suffixes.join("\n\n")),
                    (false, false) => format!(
                        "{}\n\n{query}\n\n{}",
                        prefixes.join("\n\n"),
                        suffixes.join("\n\n"),
                    ),
                };
                log::info!(
                    "[byop-diag] build_chat_request UserQuery: query_len={} \
                     running_command={} prefixes={} suffixes={} full_text_len={} binaries={}",
                    query.len(),
                    match request_running_command {
                        Some(rc) => format!(
                            "Some(grid_len={} alt={})",
                            rc.grid_contents.len(),
                            rc.is_alt_screen_active
                        ),
                        None => "None".to_owned(),
                    },
                    prefixes.len(),
                    suffixes.len(),
                    full_text.len(),
                    user_attachments.binaries.len(),
                );
                messages.push(build_user_message_with_binaries(
                    full_text,
                    user_attachments.binaries,
                    attachment_caps,
                ));
            }
            AIAgentInput::ActionResult { result, .. } => {
                // 上一轮模型回了 tool_calls,client 端执行完后 result 走 `params.input`
                // 而不是 `params.tasks` 历史。必须在这里序列化为 ToolResponse,否则
                // genai/上游会因 tool_call_id 配对失败 400。
                let tool_call_id = result.id.to_string();
                let content = tools::serialize_action_result(result).unwrap_or_else(|| {
                    serde_json::json!({ "result": result.result.to_string() }).to_string()
                });
                messages.push(ChatMessage::from(ToolResponse::new(tool_call_id, content)));
            }
            AIAgentInput::InvokeSkill {
                skill, user_query, ..
            } => {
                let mut composed = format!(
                    "请按下面的技能 \"{}\" 指引执行任务:\n\n{}\n\n---\n",
                    skill.name, skill.content,
                );
                if let Some(uq) = user_query {
                    composed.push_str(&format!("用户进一步指令: {}", uq.query));
                }
                messages.push(ChatMessage::user(composed));
            }
            AIAgentInput::ResumeConversation { context } => {
                // BYOP 没有 server 端 resume prompt 注入层。LRC auto-resume 时必须显式
                // 重带当前 PTY 上下文,否则错误恢复轮会退化成普通对话并重新选择 shell 工具。
                let mut prefixes: Vec<String> = Vec::new();
                if let Some(lrc_prefix) = render_lrc_request_context(params) {
                    prefixes.push(lrc_prefix);
                }
                let user_attachments = user_context::collect_user_attachments(context);
                if let Some(p) = &user_attachments.prefix {
                    prefixes.push(p.clone());
                }
                if !prefixes.is_empty() {
                    let full_text = format!("{}\n\nContinue.", prefixes.join("\n\n"));
                    messages.push(build_user_message_with_binaries(
                        full_text,
                        user_attachments.binaries,
                        attachment_caps,
                    ));
                }
            }
            AIAgentInput::SummarizeConversation {
                prompt, ..
            } => {
                // Marb BYOP 本地会话压缩入口 — 1:1 对齐 opencode `compaction.ts processCompaction`。
                //
                // 此前 messages loop 已根据 `summarize_head_end` 把序列切到 head(去掉 tail);
                // 这里追加最后一条 user message:`build_prompt(previous_summary, plugin_context)`,
                // 它包含 SUMMARY_TEMPLATE(9 段 Markdown 模板)+ 增量摘要锚点。
                //
                // 模型会 emit 一段结构化 Markdown 摘要文本,controller 接到 stream 完成
                // 后把它写回 conversation.compaction_state(参见 Phase 6 controller 改动)。
                let prev_summary = params
                    .compaction_state
                    .as_ref()
                    .and_then(|s| s.previous_summary())
                    .map(str::to_string);
                let mut anchor_context: Vec<String> = Vec::new();
                if let Some(custom) = prompt.as_ref().filter(|p| !p.is_empty()) {
                    // /compact <自定义指令> 走这里 — 把用户指令拼到 plugin_context 段
                    anchor_context
                        .push(format!("Additional instructions from the user:\n{custom}"));
                }
                let nextp =
                    byop_compaction::prompt::build_prompt(prev_summary.as_deref(), &anchor_context);
                messages.push(ChatMessage::user(nextp));
            }
            AIAgentInput::AutoCodeDiffQuery { .. }
            | AIAgentInput::CreateNewProject { .. }
            | AIAgentInput::CodeReview { .. } => {
                // 暂时忽略
            }
            _ => {}
        }
    }

    if let ReadinessState::AcceptedHistoryRepair { repairs } = &readiness_report.state {
        repair_tool_call_pairs_for_accepted_history_gaps(
            &mut messages,
            repairs,
            &outbound_tool_groups,
        )?;
    }

    // 防御性 sanitize: 确保 messages 末尾不是 assistant。
    // Anthropic / 部分网关不接受末尾为 assistant 的请求(prefill 仅特定模型支持),
    // 而 warp 的 `AIAgentInput::ResumeConversation`(handoff/auto-resume after error 等)
    // 不附加新 user 消息,会让序列末尾停在历史 assistant 上。
    // 这里统一兜底:末尾若是 assistant,追加一条隐式 user 消息让上游继续。
    ensure_ends_with_user(&mut messages);

    let mut tools_array = build_tools_array(params);

    // Anthropic 路径:给 tools 数组**最后一个 tool**打 1h cache_control breakpoint,
    // 使整个 tools 段成为长 TTL 的静态前缀(对齐 Zed
    // `crates/anthropic/src/completion.rs::254-258`)。
    //
    // 处理顺序 `tools → system → messages`,长 TTL 必须在短 TTL 之前。本路径下:
    // - tools 末尾 1h(此处)
    // - system 1h(`apply_caching_anthropic` 给 ChatRole::System message)
    // - messages 尾部 5m(`apply_caching_anthropic` 给 last 2 non-system)
    //
    // tools 段在 session 内变化最少(切 web_search / plan_mode / LRC 才变),命中率
    // 极高,1h 写入 2× base 摊到多次复用,等效近 0 — 同时挡住外部反代在 system
    // 上注入 5m 引发的 1h-after-5m 排序错误(让 tools 1h 这一个 breakpoint 接管
    // 整个 tools+system 静态前缀)。
    if matches!(api_type, AgentProviderApiType::Anthropic) {
        if let Some(last_tool) = tools_array.last_mut() {
            last_tool.cache_control = Some(CacheControl::Ephemeral1h);
        }
    }

    // 出站消息文本透传给 `serde_json` 处理 JSON escape,不再做激进的字符级
    // sanitize(参考 zed `into_anthropic` / opencode `provider/transform.ts`,
    // 两者都不在出站层打平控制字符或替换 `\` / `"`)。Anthropic / OpenAI / Gemini
    // 官方 API 与主流 BYOP 反代均能正确处理 `serde_json` 产出的合法 escape。

    // Prompt caching(1:1 移植自 opencode `provider/transform.ts::applyCaching`):
    // - opencode 选 first 2 system message + last 2 non-system message,统一打上
    //   anthropic.cacheControl / openaiCompatible.cache_control / bedrock.cachePoint
    //   等多 SDK 兼容标记。AI SDK 各 provider 实现读对应 key,无关 key 自动忽略。
    // - 我们走 rust-genai,Anthropic adapter 支持 per-message `cache_control`,
    //   OpenAI / OpenAiResp adapter 仅认 `ChatOptions` 级别的 prompt_cache_key /
    //   cache_control,DeepSeek / Gemini / Ollama 服务端隐式缓存,无需 client opt-in。
    // - 故在此只对 Anthropic 路径"per-message"打标:把 system 文本作为
    //   ChatRole::System message 推到 messages 头部并打 Ephemeral,再把末尾两条
    //   非 system message 也打 Ephemeral(对应 opencode 的 system+last 2 模式)。
    //   OpenAI 系的 `prompt_cache_key` / `cache_control` 在 `build_chat_options`
    //   里设置(请求级别),也来自 opencode 同一组规则的下游 fallback。
    let messages = if matches!(api_type, AgentProviderApiType::Anthropic) {
        let mut msgs: Vec<ChatMessage> = std::iter::once(ChatMessage::system(system_text.clone()))
            .chain(messages)
            .collect();
        apply_caching_anthropic(&mut msgs);
        msgs
    } else {
        messages
    };

    let mut req = ChatRequest::from_messages(messages);
    // Anthropic 路径 system 已经作为 ChatRole::System message 进 messages,
    // 不再设 `with_system`,避免 genai Anthropic adapter 的"first system 不能挂
    // cache_control"限制(`adapter_impl.rs::into_anthropic_request_parts` 注释)。
    if !matches!(api_type, AgentProviderApiType::Anthropic) {
        req = req.with_system(system_text);
    }
    if !tools_array.is_empty() {
        req = req.with_tools(tools_array);
    }
    Ok(req)
}

const BYOP_DIAG_SNIPPET_CHARS: usize = 240;
const REPAIR_PLACEHOLDER_NOTE: &str =
    "tool result was unavailable in repaired conversation history";

fn is_placeholder_tool_response_content(content: &str) -> bool {
    if content == "(tool 执行结果未保留)" {
        return true;
    }

    let Ok(Value::Object(object)) = serde_json::from_str::<Value>(content) else {
        return false;
    };

    object.len() == 3
        && object.get("status").and_then(Value::as_str) == Some("unavailable")
        && matches!(
            object.get("reason").and_then(Value::as_str),
            Some("forked_history_repair" | "restored_legacy_history_repair")
        )
        && object.get("note").and_then(Value::as_str) == Some(REPAIR_PLACEHOLDER_NOTE)
}

fn insert_preferred_tool_response(
    responses_by_call_id: &mut HashMap<String, ToolResponse>,
    response: &ToolResponse,
) {
    let should_replace = match responses_by_call_id.get(&response.call_id) {
        None => true,
        Some(existing) => should_replace_tool_response(existing, response),
    };
    if should_replace {
        responses_by_call_id.insert(response.call_id.clone(), response.clone());
    }
}

fn should_replace_tool_response(existing: &ToolResponse, candidate: &ToolResponse) -> bool {
    is_placeholder_tool_response_content(&existing.content)
        || !is_placeholder_tool_response_content(&candidate.content)
}

fn snippet_for_log(s: &str, max_chars: usize) -> String {
    use std::fmt::Write as _;

    let mut out = String::new();
    for (idx, ch) in s.chars().enumerate() {
        if idx >= max_chars {
            out.push_str("...");
            break;
        }
        match ch {
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            c if c.is_control() => {
                let _ = write!(out, "\\u{{{:04x}}}", c as u32);
            }
            c => out.push(c),
        }
    }
    out
}

fn json_value_for_log(value: &Value) -> (usize, String) {
    let json = serde_json::to_string(value)
        .unwrap_or_else(|_| "<failed-to-serialize-json-value>".to_owned());
    (json.len(), snippet_for_log(&json, BYOP_DIAG_SNIPPET_CHARS))
}

fn binary_for_log(binary: &Binary) -> String {
    let name = binary
        .name
        .as_deref()
        .map(|n| snippet_for_log(n, 80))
        .unwrap_or_default();
    match &binary.source {
        BinarySource::Base64(data) => format!(
            "mime={} name={} source=base64 chars={}",
            binary.content_type,
            name,
            data.len()
        ),
        BinarySource::Url(url) => format!(
            "mime={} name={} source=url chars={} url={}",
            binary.content_type,
            name,
            url.len(),
            snippet_for_log(url, 120)
        ),
    }
}

fn log_chat_request_details(
    chat_req: &ChatRequest,
    model_id: &str,
    api_type: AgentProviderApiType,
) {
    let system_in_head = matches!(api_type, AgentProviderApiType::Anthropic)
        && chat_req
            .messages
            .first()
            .map(|m| matches!(m.role, ChatRole::System))
            .unwrap_or(false);
    let tool_count = chat_req.tools.as_ref().map(|t| t.len()).unwrap_or(0);
    let tool_names: Vec<String> = chat_req
        .tools
        .as_ref()
        .map(|tools| tools.iter().map(|t| t.name.as_str().to_owned()).collect())
        .unwrap_or_default();
    log::info!(
        "[byop-diag] request summary: adapter={:?} model={} system_len={} \
         system_in_messages_head={} messages={} tools={} tool_names={:?} \
         previous_response_id_present={} store={:?} system_snippet={:?}",
        adapter_kind_for(api_type),
        model_id,
        chat_req.system.as_deref().map(str::len).unwrap_or(0),
        system_in_head,
        chat_req.messages.len(),
        tool_count,
        tool_names,
        chat_req.previous_response_id.is_some(),
        chat_req.store,
        chat_req
            .system
            .as_deref()
            .map(|s| snippet_for_log(s, BYOP_DIAG_SNIPPET_CHARS))
            .unwrap_or_default(),
    );

    if let Some(tools) = &chat_req.tools {
        for (idx, tool) in tools.iter().enumerate() {
            let schema_len = tool
                .schema
                .as_ref()
                .and_then(|schema| serde_json::to_string(schema).ok())
                .map(|schema| schema.len())
                .unwrap_or(0);
            log::info!(
                "[byop-diag] request tool[{idx}]: name={} desc_len={} schema_len={} \
                 strict={:?} cache_control={:?}",
                tool.name.as_str(),
                tool.description.as_deref().map(str::len).unwrap_or(0),
                schema_len,
                tool.strict,
                tool.cache_control,
            );
        }
    }

    let flow: Vec<String> = chat_req
        .messages
        .iter()
        .enumerate()
        .map(|(idx, msg)| {
            let text_len: usize = msg.content.texts().iter().map(|t| t.len()).sum();
            let tool_call_ids: Vec<String> = msg
                .content
                .tool_calls()
                .iter()
                .map(|tc| tc.call_id.clone())
                .collect();
            let tool_response_ids: Vec<String> = msg
                .content
                .tool_responses()
                .iter()
                .map(|tr| tr.call_id.clone())
                .collect();
            format!(
                "{idx}:{:?}(text_len={text_len},tool_calls={tool_call_ids:?},tool_responses={tool_response_ids:?})",
                msg.role
            )
        })
        .collect();
    log::info!("[byop-diag] request message_flow={flow:?}");

    for (idx, msg) in chat_req.messages.iter().enumerate() {
        let mut text_count = 0;
        let mut text_total_len = 0;
        let mut first_text_snippet: Option<String> = None;
        let mut binary_summaries: Vec<String> = Vec::new();
        let mut tool_call_summaries: Vec<String> = Vec::new();
        let mut tool_response_summaries: Vec<String> = Vec::new();
        let mut thought_count = 0;
        let mut thought_total_len = 0;
        let mut reasoning_count = 0;
        let mut reasoning_total_len = 0;
        let mut custom_count = 0;

        for part in &msg.content {
            match part {
                ContentPart::Text(text) => {
                    text_count += 1;
                    text_total_len += text.len();
                    if first_text_snippet.is_none() {
                        first_text_snippet = Some(snippet_for_log(text, BYOP_DIAG_SNIPPET_CHARS));
                    }
                }
                ContentPart::Binary(binary) => {
                    binary_summaries.push(binary_for_log(binary));
                }
                ContentPart::ToolCall(tool_call) => {
                    let (args_len, args_snippet) = json_value_for_log(&tool_call.fn_arguments);
                    tool_call_summaries.push(format!(
                        "call_id={} name={} args_len={} args={} thought_signatures={}",
                        tool_call.call_id,
                        tool_call.fn_name,
                        args_len,
                        args_snippet,
                        tool_call
                            .thought_signatures
                            .as_ref()
                            .map(|s| s.len())
                            .unwrap_or(0)
                    ));
                }
                ContentPart::ToolResponse(tool_response) => {
                    tool_response_summaries.push(format!(
                        "call_id={} content_len={} placeholder={} content={}",
                        tool_response.call_id,
                        tool_response.content.len(),
                        is_placeholder_tool_response_content(&tool_response.content),
                        snippet_for_log(&tool_response.content, BYOP_DIAG_SNIPPET_CHARS)
                    ));
                }
                ContentPart::ThoughtSignature(thought) => {
                    thought_count += 1;
                    thought_total_len += thought.len();
                }
                ContentPart::ReasoningContent(reasoning) => {
                    reasoning_count += 1;
                    reasoning_total_len += reasoning.len();
                }
                ContentPart::Custom(_) => {
                    custom_count += 1;
                }
            }
        }

        let cache_control = msg
            .options
            .as_ref()
            .and_then(|options| options.cache_control.as_ref())
            .map(|cache| format!("{cache:?}"))
            .unwrap_or_else(|| "None".to_owned());
        log::info!(
            "[byop-diag] request message[{idx}]: role={:?} parts={} size={} \
             cache_control={} text_parts={} text_total_len={} first_text={:?} \
             binaries={:?} tool_calls={:?} tool_responses={:?} \
             thought_signatures={} thought_total_len={} reasoning_parts={} \
             reasoning_total_len={} custom_parts={}",
            msg.role,
            msg.content.len(),
            msg.content.size(),
            cache_control,
            text_count,
            text_total_len,
            first_text_snippet.unwrap_or_default(),
            binary_summaries,
            tool_call_summaries,
            tool_response_summaries,
            thought_count,
            thought_total_len,
            reasoning_count,
            reasoning_total_len,
            custom_count,
        );
    }

    for (idx, msg) in chat_req.messages.iter().enumerate() {
        let expected_call_ids: Vec<String> = msg
            .content
            .tool_calls()
            .iter()
            .map(|tc| tc.call_id.clone())
            .collect();
        if expected_call_ids.is_empty() {
            continue;
        }
        let next = chat_req.messages.get(idx + 1);
        let next_role = next.map(|m| format!("{:?}", m.role)).unwrap_or_default();
        let response_call_ids: Vec<String> = next
            .filter(|m| matches!(m.role, ChatRole::Tool))
            .map(|m| {
                m.content
                    .tool_responses()
                    .iter()
                    .map(|tr| tr.call_id.clone())
                    .collect()
            })
            .unwrap_or_default();
        let matched = response_call_ids == expected_call_ids;
        if matched {
            log::info!(
                "[byop-diag] request tool_pair idx={idx}: expected_call_ids={expected_call_ids:?} \
                 next_role={next_role} response_call_ids={response_call_ids:?}"
            );
        } else {
            log::warn!(
                "[byop-diag] request tool_pair mismatch idx={idx}: \
                 expected_call_ids={expected_call_ids:?} next_role={next_role} \
                 response_call_ids={response_call_ids:?}"
            );
        }
    }

    for (idx, msg) in chat_req.messages.iter().enumerate() {
        if !matches!(msg.role, ChatRole::Tool) {
            continue;
        }
        let response_call_ids: Vec<String> = msg
            .content
            .tool_responses()
            .iter()
            .map(|tr| tr.call_id.clone())
            .collect();
        let previous_expected: Vec<String> = idx
            .checked_sub(1)
            .and_then(|prev_idx| chat_req.messages.get(prev_idx))
            .filter(|prev| matches!(prev.role, ChatRole::Assistant))
            .map(|prev| {
                prev.content
                    .tool_calls()
                    .iter()
                    .map(|tc| tc.call_id.clone())
                    .collect()
            })
            .unwrap_or_default();
        if response_call_ids != previous_expected {
            log::warn!(
                "[byop-diag] request orphan_or_misordered_tool_response idx={idx}: \
                 response_call_ids={response_call_ids:?} previous_assistant_call_ids={previous_expected:?}"
            );
        }
    }
}

/// 仅在 serializer 已判定为 `AcceptedHistoryRepair` 后运行:把被 RepairRecord
/// 明确授权的历史缺口转换为 outbound-only 结构化 ToolResponse。
///
/// 普通缺失、重复、孤儿或跨边界乱序已经在 readiness validation 阶段阻断;
/// 这里不再为 normal flow 生成占位结果,也不写回 conversation history。
fn repair_tool_call_pairs_for_accepted_history_gaps(
    messages: &mut Vec<ChatMessage>,
    repairs: &[AcceptedRepair],
    outbound_tool_groups: &[OutboundAssistantToolGroup],
) -> Result<(), ConvertToAPITypeError> {
    use std::collections::{HashMap, HashSet};

    if repairs.is_empty() {
        return Ok(());
    }

    let repair_by_key: HashMap<ToolCallKey, &AcceptedRepair> = repairs
        .iter()
        .map(|repair| (repair.tool_call.key.clone(), repair))
        .collect();
    let group_by_message_index: HashMap<usize, &OutboundAssistantToolGroup> = outbound_tool_groups
        .iter()
        .map(|group| (group.message_index, group))
        .collect();
    let mut call_id_counts: HashMap<String, usize> = HashMap::new();
    for group in outbound_tool_groups {
        for key in &group.tool_call_keys {
            *call_id_counts.entry(key.tool_call_id.clone()).or_default() += 1;
        }
    }
    let mut placeholders_inserted: Vec<String> = Vec::new();
    let mut orphan_call_ids: Vec<String> = Vec::new();
    let mut missing_without_repair: Vec<String> = Vec::new();

    let original = std::mem::take(messages);
    let mut late_responses_by_unique_call_id: HashMap<String, ToolResponse> = HashMap::new();
    let mut late_response_call_ids: HashSet<String> = HashSet::new();
    for (idx, msg) in original.iter().enumerate() {
        if msg.role != genai::chat::ChatRole::Tool {
            continue;
        }

        let is_adjacent_to_group =
            idx > 0 && group_by_message_index.contains_key(&(idx.saturating_sub(1)));
        if is_adjacent_to_group {
            continue;
        }

        for resp in msg.content.tool_responses() {
            if call_id_counts.get(&resp.call_id) == Some(&1) {
                insert_preferred_tool_response(&mut late_responses_by_unique_call_id, resp);
                late_response_call_ids.insert(resp.call_id.clone());
            }
        }
    }

    let mut rewritten: Vec<ChatMessage> = Vec::with_capacity(original.len());
    let mut idx = 0;
    while idx < original.len() {
        let msg = original[idx].clone();
        if msg.role == genai::chat::ChatRole::Tool {
            orphan_call_ids.extend(
                msg.content
                    .tool_responses()
                    .iter()
                    .filter(|response| !late_response_call_ids.contains(&response.call_id))
                    .map(|response| response.call_id.clone()),
            );
            idx += 1;
            continue;
        }

        let Some(group) = group_by_message_index.get(&idx).copied() else {
            rewritten.push(msg);
            idx += 1;
            continue;
        };

        rewritten.push(msg);
        idx += 1;

        let mut responses_by_call_id: HashMap<String, ToolResponse> = HashMap::new();
        while idx < original.len() && original[idx].role == genai::chat::ChatRole::Tool {
            for resp in original[idx].content.tool_responses() {
                insert_preferred_tool_response(&mut responses_by_call_id, resp);
            }
            idx += 1;
        }

        let mut bundled: Vec<ToolResponse> = Vec::new();
        for key in &group.tool_call_keys {
            let cid = &key.tool_call_id;
            let mut response = responses_by_call_id.remove(cid);
            if call_id_counts.get(cid) == Some(&1) {
                if let Some(late_response) = late_responses_by_unique_call_id.remove(cid) {
                    response = match response {
                        Some(existing)
                            if !should_replace_tool_response(&existing, &late_response) =>
                        {
                            Some(existing)
                        }
                        _ => Some(late_response),
                    };
                }
            }

            match response {
                Some(resp) => bundled.push(resp),
                None => {
                    if let Some(repair) = repair_by_key.get(key) {
                        placeholders_inserted.push(cid.clone());
                        bundled.push(ToolResponse::new(
                            cid.clone(),
                            repair_placeholder_content(repair.record.source),
                        ));
                    } else {
                        missing_without_repair.push(cid.clone());
                    }
                }
            }
        }

        if !bundled.is_empty() {
            rewritten.push(ChatMessage::from(bundled));
        }

        if !responses_by_call_id.is_empty() {
            orphan_call_ids.extend(responses_by_call_id.into_keys());
        }
    }

    *messages = rewritten;

    if !orphan_call_ids.is_empty() {
        log::warn!(
            "[byop-diag] accepted_history_repair: 丢弃 {} 个孤儿 ToolResponse: \
             orphan_call_ids={:?}",
            orphan_call_ids.len(),
            orphan_call_ids
        );
    }
    if !placeholders_inserted.is_empty() {
        log::info!(
            "[byop-diag] accepted_history_repair: 给 {} 个 ToolCall 补 repair placeholder \
             ToolResponse: missing_call_ids={:?}",
            placeholders_inserted.len(),
            placeholders_inserted
        );
    }
    if !missing_without_repair.is_empty() {
        // readiness classifier 已判定 AcceptedHistoryRepair 时,每个 missing tool call 都应
        // 在 repairs 中有对应授权;若到这里仍 missing,说明 classifier 与 serializer 的
        // tool call key 来源出现了不一致(例如未来重构 projection 或 outbound_tool_groups 构建逻辑
        // 引入差异)。此时不能继续发出缺失 ToolResponse 的非法请求,必须阻断。
        log::error!(
            "[byop-diag] accepted_history_repair: readiness 未授权的缺失 ToolResponse: \
             missing_call_ids={:?}",
            missing_without_repair
        );
        return Err(ConvertToAPITypeError::Other(
            BlockedByopReadinessError::new(ReadinessCategory::MissingResultWithoutRepairSource)
                .into(),
        ));
    }
    Ok(())
}

fn repair_placeholder_content(source: RepairSource) -> String {
    json!({
        "status": "unavailable",
        "reason": source.placeholder_reason(),
        "note": REPAIR_PLACEHOLDER_NOTE,
    })
    .to_string()
}

/// 兜底:确保 messages 末尾是 user(或 tool 响应)。
///
/// 触发场景:`AIAgentInput::ResumeConversation` 不附加新 user 消息,直接重发历史。
/// Anthropic 原生 API 拒绝末尾为 assistant 的请求(`This model does not support
/// assistant message prefill. The conversation must end with a user message.`),
/// 重试 3 次都同 payload → UI 渲染 error block 触发 flex panic。
///
/// 末尾是 assistant 时追加 `ChatMessage::user("Continue.")`,提示模型继续即可。
/// Tool 角色作为 user 输入的一种(模型会把 tool 响应当作下一轮起点)不动。
/// 空 messages 不触发,避免给空对话凭空塞内容。
fn ensure_ends_with_user(messages: &mut Vec<ChatMessage>) {
    use genai::chat::ChatRole;
    if let Some(last) = messages.last() {
        if last.role == ChatRole::Assistant {
            messages.push(ChatMessage::user("Continue."));
        }
    }
}


// ---------------------------------------------------------------------------
// 主流程
// ---------------------------------------------------------------------------

/// 标题生成所需的 BYOP 配置。可能与主请求同 provider 也可能不同(用户在 Profile
/// Editor 里独立选了 title_model)。
pub struct TitleGenInput {
    pub base_url: String,
    pub api_key: String,
    pub model_id: String,
    pub api_type: AgentProviderApiType,
    pub reasoning_effort: crate::settings::ReasoningEffortSetting,
}

pub struct ByopOutputInput {
    pub params: RequestParams,
    pub base_url: String,
    pub api_key: String,
    pub model_id: String,
    pub api_type: AgentProviderApiType,
    pub reasoning_effort: crate::settings::ReasoningEffortSetting,
    pub extra_headers: Vec<(String, String)>,
    pub task_id: String,
    pub target_task_id: String,
    pub needs_create_task: bool,
    pub lrc_command_id: Option<String>,
    pub lrc_should_spawn_subagent: bool,
    pub context_window: Option<u32>,
    pub cancellation_rx: futures::channel::oneshot::Receiver<()>,
    /// ユーザー設定 (image/pdf/audio の三態 Override) を反映済みの attachment caps。
    /// `resolve_for_model` で計算され、UI 表示と runtime 動作を一致させる。
    pub attachment_caps: attachment_caps::AttachmentCaps,
}

/// `task_id`: conversation 的 root task id(controller 端从 history model 取)。
/// `target_task_id`: 本轮模型输出应该写入的 task id;普通对话等于 root,
/// CLI subagent 后续轮为已有 subtask。
/// `needs_create_task`: 仅首轮(root 还是 Optimistic)需要 emit `CreateTask`。
pub async fn generate_byop_output(
    input: ByopOutputInput,
) -> Result<ResponseStream, ConvertToAPITypeError> {
    let ByopOutputInput {
        params,
        base_url,
        api_key,
        model_id,
        api_type,
        reasoning_effort,
        extra_headers,
        task_id,
        target_task_id,
        needs_create_task,
        lrc_command_id,
        lrc_should_spawn_subagent,
        context_window,
        cancellation_rx: _cancellation_rx,
        attachment_caps,
    } = input;

    let force_echo_reasoning = super::reasoning::model_requires_reasoning_echo(api_type, &model_id);
    // 仅对已知把 reasoning 夹在 <think> 标签里的模型(如 MiniMax M3)激活流式提取。
    // 其他模型保持原始 Chunk 输出行为,避免误吞含字面量 <think> 的正常文本。
    let use_think_extraction = super::reasoning::model_uses_think_tags_in_content(&model_id);
    let chat_req = build_chat_request(&params, force_echo_reasoning, api_type, attachment_caps)?;
    let conversation_id = params
        .conversation_token
        .as_ref()
        .map(|t| t.as_str().to_string())
        .unwrap_or_default();
    let chat_opts = build_chat_options(
        api_type,
        &base_url,
        &model_id,
        reasoning_effort,
        extra_headers,
        if conversation_id.is_empty() {
            None
        } else {
            Some(conversation_id.as_str())
        },
    );
    let client = build_client(api_type, base_url, api_key);
    let request_id = Uuid::new_v4().to_string();
    let mcp_context = params.mcp_context.clone();

    // ⚠️ BYOP 持久化关键:warp 自家路径下,以下 ClientAction 都是 server 端 emit
    // 让 client 端把 UserQuery / ToolCallResult 等"非模型产出"的 message
    // 写回 task.messages,从而让下一轮请求的 `params.tasks` snapshot 完整。
    //
    // BYOP 去云化客户端自管,server 端不存在,必须我们自己 emit 这些写回事件,
    // 否则下一轮 `compute_active_tasks` 只看到模型产出(reasoning/output/tool_call),
    // 缺失对应的 user_query 和 tool_call_result,模型 context 严重断裂。
    //
    // 这里在流开始后把当前轮 UserQuery / ToolCallResult 按 `params.input` 原顺序写入。
    // 用户打断 pending tool 后继续输入时,controller 会传入 ActionResult → UserQuery;
    // 持久化不能拆成"所有 UserQuery 先写,所有 ActionResult 后写",否则历史会变成
    // Assistant(tool_call) → UserQuery → ToolCallResult。
    //
    // emit 时机必须在 CreateTask 之后(任务已升级为 Server 状态),
    // 在模型响应开始之前(UI 顺序:user 显示 → thinking/answer)。
    // Marb:历史轮多模态保活。除 query 文本外,把当前轮 UserQuery.context 里的所有
    // multimodal binary(image / pdf / audio / ...)一并打包进 `UserQuery.context.images`
    // 持久化(proto 字段叫 images,语义上是通用 BinaryFile —— `bytes data + mime_type`,
    // 跟 opencode FilePart 等价),使 build_chat_request 下一轮重建 messages 时能从历史
    // message 上恢复 binary,继续以 ContentPart::Binary 注入上游(模型不支持的 mime 由
    // build_user_message_with_binaries 替换为 ERROR 文本,与 opencode unsupportedParts 一致)。
    // 上游 warp 自家路径不需要这步因为云端 server 持有 InputContext;BYOP 直连必须客户端自管。
    let pending_user_queries: Vec<(String, Vec<user_context::UserBinary>)> = params
        .input
        .iter()
        .filter_map(|i| match i {
            AIAgentInput::UserQuery { query, context, .. } => {
                let attachments = user_context::collect_user_attachments(context);
                Some((query.clone(), attachments.binaries))
            }
            _ => None,
        })
        .collect();
    // INFO 级别一行总览 + 每条 message 一行简报(role + 文本长度 + tool 计数 + reasoning 标记),
    // 默认日志配置即可看到,便于诊断"历史是否完整传上去"等问题。
    //
    // 注:Anthropic 路径下，`build_chat_request` 会把 system 文本作为 `ChatMessage::system`
    // 推到 messages[0] 以便打 `cache_control`，所以 `chat_req.system` 会是 None、`system_len`
    // 显示为 0；实际 system 内容仍然在 messages[0] 里(看下面逐条报告)。为避免误
    // 导诊断者，这里加上 `system_in_messages_head` 提示。
    log_chat_request_details(&chat_req, &model_id, api_type);

    // 诊断:构造包含 system / messages / tools 的完整 ChatRequest JSON dump,保存到
    // stream 闭包。真实 Anthropic wire body 会由 genai adapter 再转换一层,但这里已经
    // 覆盖所有传入 BYOP 的原始字符串,足够定位非法 escape 来自 prompt、工具描述、
    // schema 还是 tool result。
    let diag_body_json = serde_json::to_string(&json!({
        "model": &model_id,
        "chat_request": &chat_req,
    }))
    .unwrap_or_default();
    log::info!("[byop] diag_body_approx_len={}", diag_body_json.len());
    log::info!("[byop-diag] full_request_json={diag_body_json}");

    // 主动扫描原始文本里的"可疑反斜杠序列":serde_json 把源字符串里的字面
    // `\` 序列化为 `\\`,所以 wire body 里出现"两个连续反斜杠 + u/x" 才意味着
    // 原文有字面 `\u` / `\x`,这是 proxy 误"还原 `\\u` → `\u`"触发 invalid escape
    // 的真实风险点。源字符串里的 `\n` / `\r` / `\t` 经 serde_json 输出为单个反斜杠 +
    // 字母,本身就是合法 JSON escape,proxy 不会再二次还原,不算可疑。
    fn scan_suspicious_backslash(label: &str, s: &str) {
        let bytes = s.as_bytes();
        let mut bs_hits: Vec<(usize, String)> = Vec::new();
        let mut ctrl_hits: Vec<(usize, u8)> = Vec::new();
        let mut i = 0;
        while i < bytes.len() {
            let b = bytes[i];
            // 字面 `\\u` / `\\x` 序列(源字符串中含 `\u` / `\x`)。
            if b == b'\\'
                && i + 2 < bytes.len()
                && bytes[i + 1] == b'\\'
                && matches!(bytes[i + 2], b'u' | b'x')
            {
                let end = (i + 10).min(bytes.len());
                let snippet = String::from_utf8_lossy(&bytes[i..end]).to_string();
                if bs_hits.len() < 5 {
                    bs_hits.push((i, snippet));
                }
                // 跳过这一对,避免对同一位置触发多次。
                i += 3;
                continue;
            }
            // raw 控制字符(byte 0x00-0x08, 0x0B-0x0C, 0x0E-0x1F)。
            // serde_json 会 escape 为 `\u00XX`,合法 JSON;但部分 strict proxy
            // 或经过 base64 / 中间编码层时这些字节最容易出错。
            if (b < 0x20 && !matches!(b, b'\t' | b'\n' | b'\r')) && ctrl_hits.len() < 10 {
                ctrl_hits.push((i, b));
            }
            i += 1;
        }
        if !bs_hits.is_empty() {
            log::warn!("[byop] {label} suspicious literal '\\\\u'/'\\\\x' patterns: {bs_hits:?}");
        }
        if !ctrl_hits.is_empty() {
            log::warn!("[byop] {label} contains raw control chars (offset, byte): {ctrl_hits:?}");
        }
    }
    scan_suspicious_backslash("full_request_json", &diag_body_json);
    if let Some(sys) = chat_req.system.as_deref() {
        scan_suspicious_backslash("system", sys);
    }
    for (idx, m) in chat_req.messages.iter().enumerate() {
        if let Some(t) = m.content.first_text() {
            scan_suspicious_backslash(&format!("msg[{idx}]"), t);
        }
    }

    let stream = async_stream::stream! {
        let mut chat_req = chat_req;
        // 1) StreamInit — 始终先发,UI 能立刻显示 "thinking..."
        yield Ok(api::ResponseEvent {
            r#type: Some(api::response_event::Type::Init(
                api::response_event::StreamInit {
                    request_id: request_id.clone(),
                    conversation_id,
                    run_id: String::new(),
                },
            )),
        });

        // 2) 首轮:CreateTask 升级 Optimistic root → Server。
        if needs_create_task {
            yield Ok(create_task_event(&task_id));
        }

        // 3) 持久化 input 里的 UserQuery / ToolCallResult 到 task.messages。
        //    (warp server 路径由后端 emit;BYOP 客户端必须自己 emit,见上方注释。)
        //    tag-in 首轮先写 root,再由下面的 spawn 分支复制到新 subtask;已有 CLI
        //    subagent 的后续轮直接写 target_task_id。
        let persistence_task_id = if lrc_should_spawn_subagent {
            task_id.as_str()
        } else {
            target_task_id.as_str()
        };
        let mut persistence_messages: Vec<api::Message> = Vec::new();
        let mut persistence_order: Vec<String> = Vec::new();
        for (input_idx, input) in params.input.iter().enumerate() {
            match input {
                AIAgentInput::UserQuery {
                    query,
                    context,
                    running_command,
                    ..
                } => {
                    let attachments = user_context::collect_user_attachments(context);
                    log::info!(
                        "[byop-diag] persistence input[{input_idx}]: task_id={} \
                         kind=UserQuery query_len={} binaries={} running_command={} \
                         lrc_command_id={} query={:?}",
                        persistence_task_id,
                        query.len(),
                        attachments.binaries.len(),
                        running_command.is_some(),
                        lrc_command_id.as_deref().unwrap_or(""),
                        snippet_for_log(query, BYOP_DIAG_SNIPPET_CHARS),
                    );
                    persistence_order.push(format!(
                        "{input_idx}:UserQuery(query_len={},binaries={})",
                        query.len(),
                        attachments.binaries.len()
                    ));
                    persistence_messages.push(make_user_query_message(
                        persistence_task_id,
                        &request_id,
                        query.clone(),
                        &attachments.binaries,
                    ));
                }
                AIAgentInput::ActionResult { result, .. } => {
                    let content = tools::serialize_action_result(result).unwrap_or_else(|| {
                        serde_json::json!({ "result": result.result.to_string() }).to_string()
                    });
                    log::info!(
                        "[byop-diag] persistence input[{input_idx}]: task_id={} \
                         kind=ActionResult call_id={} content_len={} content={:?}",
                        persistence_task_id,
                        result.id,
                        content.len(),
                        snippet_for_log(&content, BYOP_DIAG_SNIPPET_CHARS),
                    );
                    persistence_order.push(format!(
                        "{input_idx}:ActionResult(call_id={},content_len={})",
                        result.id,
                        content.len()
                    ));
                    persistence_messages.push(make_tool_call_result_message(
                        persistence_task_id,
                        &request_id,
                        result.id.to_string(),
                        content,
                    ));
                }
                _ => {}
            }
        }
        log::info!(
            "[byop-diag] persistence summary: request_id={} task_id={} emitted_messages={} \
             input_order={:?}",
            request_id,
            persistence_task_id,
            persistence_messages.len(),
            persistence_order,
        );
        if !persistence_messages.is_empty() {
            yield Ok(make_add_messages_event(persistence_task_id, persistence_messages));
        }

        // 3.5) LRC subagent spawn(对齐上游云端的 cli subagent 注入路径)。
        //
        // 当请求来自 alt-screen + agent tagged-in 状态时,`lrc_command_id` 携带当前 LRC
        // block 的 id 字符串。此处客户端合成两条事件:
        //   a) AddMessagesToTask(root, [<虚拟 subagent tool_call>])
        //      在 root.messages 里挂一条 ToolCall::Subagent { task_id=<新 subtask>,
        //      metadata: Cli { command_id }, payload: "" }。
        //      conversation `Task::new_subtask` 会从 parent.messages 里按 task_id 匹配
        //      这条 subagent_call,提取出 SubagentParams 挂到 subtask。
        //   b) CreateTask(api::Task { id=<新 subtask>, dependencies.parent_task_id=root })
        //      触发 `apply_client_action::CreateTask`,因 parent_id 非空走 `new_subtask`,
        //      接着 emit `BlocklistAIHistoryEvent::CreatedSubtask` →
        //      `cli_controller::handle_history_model_event` 看到 cli_subagent_block_id
        //      非空,emit `CLISubagentEvent::SpawnedSubagent` → terminal_view 创建
        //      `CLISubagentView` 浮窗,挂进 `cli_subagent_views` map。
        //
        // 切换后续 chunk emit 的 task_id 到 subtask_id,让模型 reasoning/output/tool_call
        // 全部进 subtask,subagent_view 据此渲染浮窗内容。
        //
        // 时序约束:必须在 root CreateTask + UserQuery 持久化之后,模型流之前。
        // 否则 conversation 找不到 root task / 找不到 user query 引用对。
        let mut current_task_id = if lrc_should_spawn_subagent {
            task_id.clone()
        } else {
            target_task_id.clone()
        };
        if lrc_should_spawn_subagent {
            let Some(command_id) = lrc_command_id.clone() else {
                log::warn!("[byop] LRC spawn requested without command_id");
                yield Err(Arc::new(AIApiError::Other(anyhow::anyhow!(
                    "BYOP LRC spawn requested without command_id"
                ))));
                return;
            };
            let subtask_id = Uuid::new_v4().to_string();
            let tool_call_id = Uuid::new_v4().to_string();
            log::info!(
                "[byop] LRC tag-in: spawning cli subagent subtask={subtask_id} \
                 command_id={command_id} parent={task_id}"
            );

            let subagent_tool = api::message::tool_call::Tool::Subagent(
                api::message::tool_call::Subagent {
                    task_id: subtask_id.clone(),
                    payload: String::new(),
                    metadata: Some(
                        api::message::tool_call::subagent::Metadata::Cli(
                            api::message::tool_call::subagent::CliSubagent {
                                command_id,
                            },
                        ),
                    ),
                },
            );
            let subagent_msg = make_tool_call_message(
                &task_id,
                &request_id,
                &tool_call_id,
                subagent_tool,
            );
            // a) 把 subagent tool_call 挂到 root.messages,供 new_subtask 反查 SubagentParams。
            yield Ok(make_add_messages_event(&task_id, vec![subagent_msg]));
            // b) 创建带 parent_task_id 的 subtask;conversation 检测 parent_id 非空 →
            //    走 `Task::new_subtask` 路径,自动绑定 SubagentParams。
            yield Ok(create_subtask_event(&subtask_id, &task_id));

            // c) Marb A1:把当前轮的 UserQuery 也复制一份到 subtask,初始化 subtask 的
            //    exchange.output.messages。否则 CLISubagentView 渲染时 subtask 的 exchanges
            //    output 为空,浮窗永远只显示 49.6 高度的空对话框,看不到任何内容。
            //    上游云端在 cli subagent 任务上有完整 ClientAction 序列填 exchange.output,
            //    BYOP 客户端自管必须显式注入。
            //
            //    只复制本轮 UserQuery(`pending_user_queries`),不动 root 的副本(root
            //    保留 user query 引用以避免 exchange.input 为空导致状态机错乱)。
            //    后续模型 chunks 走 `current_task_id = subtask_id`,append 到这个起点之后。
            if !pending_user_queries.is_empty() {
                let mut subtask_messages: Vec<api::Message> = Vec::new();
                for (q, imgs) in &pending_user_queries {
                    subtask_messages.push(make_user_query_message(
                        &subtask_id,
                        &request_id,
                        q.clone(),
                        imgs,
                    ));
                }
                yield Ok(make_add_messages_event(&subtask_id, subtask_messages));
            }

            // 后续 chunk emit 切到 subtask。
            current_task_id = subtask_id;
        }

        let mut saved_stream_end: Option<genai::chat::StreamEnd> = None;
        let mut agentic_loop_iter: u32 = 0;
        loop {
        let mut intercepted_tool_responses: Vec<genai::chat::ToolResponse> = Vec::new();
        let mut has_non_intercepted_tool = false;
        log::info!("[byop] opening stream: model={model_id}");
        let mut sdk_stream = match client
            .exec_chat_stream(&model_id, chat_req.clone(), Some(&chat_opts))
            .await
        {
            Ok(resp) => {
                log::info!("[byop] stream opened OK (HTTP request accepted)");
                resp.stream
            }
            Err(e) => {
                let mapped = map_genai_error(e);
                log::error!("[byop] open stream failed: {mapped:#}");
                yield Err(Arc::new(AIApiError::Other(anyhow::anyhow!(
                    "BYOP open stream failed: {mapped}"
                ))));
                return;
            }
        };

        // 流式状态:文本 / 推理各自的 message id 在第一次 chunk 到达时生成,
        // 之后的 chunk 走 AppendToMessageContent 增量追加。
        let mut text_msg_id: Option<String> = None;
        let mut reasoning_msg_id: Option<String> = None;
        // <think>...</think> 流式提取状态:仅当 `use_think_extraction` 为 true 时有意义。
        // 已知把 reasoning 夹在 <think> 标签里的模型(如 MiniMax M3)用此提取。
        let mut think_active = false;
        let mut think_buf = String::new();
        // tool_call 按 call_id 累积 — genai 流式发的 ToolCallChunk 已带完整 ToolCall
        // (since 0.4.0 行为),但跨 chunk 同一 call_id 可能多次出现 args 增量,
        // 用 HashMap 按 id 累积后在流末统一 emit。
        let mut tool_bufs: HashMap<String, ToolCall> = HashMap::new();
        let mut tool_order: Vec<String> = Vec::new();
        // call_id → 首帧占位 ToolCall message 的 id。
        // 首次 ToolCallChunk 到达且可解析时立即 emit 一条占位卡(让 UI 在 stream End
        // 之前就能看到"调用 X 工具"反馈),流末用 update_message 原地刷新为最终 args。
        // 不在表里的 call_id(首帧 parse 失败 / web 工具)走老路径在 End 后一次性 emit。
        let mut tool_msg_ids: HashMap<String, String> = HashMap::new();
        // call_id → 上次 update_message 增量刷新的时刻。
        // 长 args 工具(create_or_edit_document、长 grep query)args 跨多 chunk 累积时,
        // 节流 ≥ 200ms reparse + update,体感跟文本流一样连续而不是首帧定格到 End。
        let mut tool_last_update: HashMap<String, Instant> = HashMap::new();
        // 增量刷新节流阈值:小于此值的连续 chunk 不再 update_message,避免频繁 UI 重排。
        // 注:SDK stream 每个 ChatStreamEvent 独立 await,多 tool 并发时本就是顺序到达,
        // 同 tick batch emit 在此层意义不大;真正降抖在节流上,这条注释提醒后续不要瞎引入 batch。
        const TOOL_ARGS_UPDATE_THROTTLE_MS: u64 = 200;
        // 诊断:统计 stream 各类事件计数,流末打 INFO log。
        // 用于排查"消息静默消失"——如果 chunk_count=0 且 tool_count=0,说明上游返回空内容。
        let mut start_count: u32 = 0;
        let mut chunk_count: u32 = 0;
        let mut chunk_bytes: usize = 0;
        let mut reasoning_count: u32 = 0;
        let mut reasoning_bytes: usize = 0;
        let mut tool_chunk_count: u32 = 0;
        let mut end_count: u32 = 0;
        let mut other_count: u32 = 0;
        // 累积本轮 token 使用量。genai 在 ChatStreamEvent::End 事件里携带
        // captured_usage(Option<Usage>),其 prompt_tokens 是本轮整段 history
        // (Anthropic / OpenAI 都按"完整请求 prompt"计),completion_tokens 是模型输出。
        // 二者相加除以 context_window 即为"context 占用率",和 warp 自家 server 路径语义一致。
        let mut captured_prompt_tokens: i32 = 0;
        let mut captured_completion_tokens: i32 = 0;
        // P0-6 prompt cache 命中率监控:从 genai `Usage.prompt_tokens_details` 里拼
        // 出 Anthropic / OpenAI / Gemini 返回的 cache_read / cache_create 字段。
        // 详见 stream End 处理逻辑。DeepSeek / Ollama 本身不走 cache 字段,后续
        // 依然保持 0。
        let mut captured_cache_read_tokens: i32 = 0;
        let mut captured_cache_create_tokens: i32 = 0;

        while let Some(item) = sdk_stream.next().await {
            let event = match item {
                Ok(ev) => ev,
                Err(e) => {
                    let mapped = map_genai_error(e);
                    let err_text = format!("{mapped:#}");
                    log::error!("[byop] stream chunk error: {err_text}");
                    log::error!("[byop-diag] full_request_json_on_error={diag_body_json}");
                    // 从错误消息里 parse "column N",dump diag_body_json 该位置 ±200 char 上下文 + 字节 hex。
                    if let Some(col) = err_text
                        .split("column ")
                        .nth(1)
                        .and_then(|s| s.chars().take_while(|c| c.is_ascii_digit()).collect::<String>().parse::<usize>().ok())
                    {
                        let body = &diag_body_json;
                        let byte_len = body.len();
                        let start = col.saturating_sub(200).min(byte_len);
                        let end = (col + 200).min(byte_len);
                        let context = body.get(start..end).unwrap_or("(slice failed: 非 char 边界)");
                        log::error!(
                            "[byop] error column={col} diag_body_len={byte_len} context[{start}..{end}]={context:?}"
                        );
                        let hex_start = col.saturating_sub(20).min(byte_len);
                        let hex_end = (col + 20).min(byte_len);
                        if let Some(slice) = body.as_bytes().get(hex_start..hex_end) {
                            log::error!("[byop] error bytes[{hex_start}..{hex_end}] hex={slice:02x?}");
                        }
                    }
                    yield Err(Arc::new(AIApiError::Other(anyhow::anyhow!(
                        "BYOP stream error: {mapped}"
                    ))));
                    return;
                }
            };

            match event {
                ChatStreamEvent::Start => {
                    // unit event;UI 已通过 StreamInit 显示 thinking,这里 no-op
                    start_count += 1;
                }
                ChatStreamEvent::Chunk(c) if !c.content.is_empty() => {
                    chunk_count += 1;
                    chunk_bytes += c.content.len();
                    if use_think_extraction {
                        // <think> 标签流式提取:仅对 THINK_TAG_IN_CONTENT_MODELS 白名单内的模型激活。
                        // 把 /delta/content 中的 <think>...</think> 段路由到 reasoning 通道,
                        // 其余内容照常走文本通道。支持标签内容跨 chunk 边界。
                        //
                        // known limitation: `<think>` 标签字符串本身跨 chunk 截断时(如
                        // chunk1 末尾为 `<thi`、chunk2 开头为 `nk>`)无法识别,残余字符串
                        // 作为普通文本输出。大多数推理模型会把 `<think>` 作为完整 token 输出,
                        // 实际触发概率极低。
                        let mut rest: &str = &c.content;
                        loop {
                            if think_active {
                                match rest.find("</think>") {
                                    Some(end) => {
                                        think_buf.push_str(&rest[..end]);
                                        let reasoning = std::mem::take(&mut think_buf);
                                        think_active = false;
                                        rest = &rest[end + "</think>".len()..];
                                        if !reasoning.is_empty() {
                                            reasoning_count += 1;
                                            reasoning_bytes += reasoning.len();
                                            if let Some(id) = reasoning_msg_id.clone() {
                                                yield Ok(make_append_event(&current_task_id, &id, AppendKind::Reasoning(reasoning)));
                                            } else {
                                                let new_id = Uuid::new_v4().to_string();
                                                let mut msg = make_reasoning_message(&current_task_id, &request_id, reasoning);
                                                msg.id = new_id.clone();
                                                reasoning_msg_id = Some(new_id);
                                                yield Ok(make_add_messages_event(&current_task_id, vec![msg]));
                                            }
                                        }
                                    }
                                    None => {
                                        think_buf.push_str(rest);
                                        break;
                                    }
                                }
                            } else {
                                match rest.find("<think>") {
                                    Some(start) => {
                                        let before = rest[..start].to_owned();
                                        think_active = true;
                                        rest = &rest[start + "<think>".len()..];
                                        if !before.is_empty() {
                                            if let Some(id) = text_msg_id.clone() {
                                                yield Ok(make_append_event(&current_task_id, &id, AppendKind::Text(before)));
                                            } else {
                                                let new_id = Uuid::new_v4().to_string();
                                                let mut msg = make_agent_output_message(&current_task_id, &request_id, before);
                                                msg.id = new_id.clone();
                                                text_msg_id = Some(new_id);
                                                yield Ok(make_add_messages_event(&current_task_id, vec![msg]));
                                            }
                                        }
                                    }
                                    None => {
                                        let text = rest.to_owned();
                                        if !text.is_empty() {
                                            if let Some(id) = text_msg_id.clone() {
                                                yield Ok(make_append_event(&current_task_id, &id, AppendKind::Text(text)));
                                            } else {
                                                let new_id = Uuid::new_v4().to_string();
                                                let mut msg = make_agent_output_message(&current_task_id, &request_id, text);
                                                msg.id = new_id.clone();
                                                text_msg_id = Some(new_id);
                                                yield Ok(make_add_messages_event(&current_task_id, vec![msg]));
                                            }
                                        }
                                        break;
                                    }
                                }
                            }
                        }
                    } else {
                        if let Some(id) = text_msg_id.clone() {
                            yield Ok(make_append_event(&current_task_id, &id, AppendKind::Text(c.content)));
                        } else {
                            let new_id = Uuid::new_v4().to_string();
                            let mut msg = make_agent_output_message(&current_task_id, &request_id, c.content);
                            msg.id = new_id.clone();
                            text_msg_id = Some(new_id);
                            yield Ok(make_add_messages_event(&current_task_id, vec![msg]));
                        }
                    }
                }
                ChatStreamEvent::Chunk(_) => {}
                ChatStreamEvent::ReasoningChunk(c) if !c.content.is_empty() => {
                    reasoning_count += 1;
                    reasoning_bytes += c.content.len();
                    // 运行时 latch:该 (api_type, model_id) 发过 reasoning chunk →
                    // 标记下一轮起强制 echo reasoning_content,覆盖 INTERLEAVED_RULES
                    // 静态表外的任意国产/第三方 thinking 模型(对齐 opencode 数据驱动思路,
                    // 用 stream 探测代替外置 catalog)。
                    super::reasoning::note_reasoning_seen(api_type, &model_id);
                    if let Some(id) = reasoning_msg_id.clone() {
                        yield Ok(make_append_event(&current_task_id, &id, AppendKind::Reasoning(c.content)));
                    } else {
                        let new_id = Uuid::new_v4().to_string();
                        let mut msg = make_reasoning_message(&current_task_id, &request_id, c.content);
                        msg.id = new_id.clone();
                        reasoning_msg_id = Some(new_id);
                        yield Ok(make_add_messages_event(&current_task_id, vec![msg]));
                    }
                }
                ChatStreamEvent::ReasoningChunk(_) => {}
                ChatStreamEvent::ToolCallChunk(tc) => {
                    tool_chunk_count += 1;
                    let mut call = tc.tool_call;
                    // 极个别 provider(自建 ollama 代理等)不发 call_id,本地 uuid 兜底。
                    if call.call_id.is_empty() {
                        call.call_id = Uuid::new_v4().to_string();
                    }
                    // 首次见到该 call_id → 立即 push 占位 ToolCall 消息到 pending_placeholders,
                    // 让 UI 在 stream End 之前就出现"调用 X 工具"卡片。
                    // 多 tool 同 tick 内来时:本循环结束前统一 batch emit 一次 add_messages,
                    // 减少 view tree 重排次数。
                    // 已在表里(占位已发)且 args 又来新 chunk → 节流 ≥ 200ms reparse + update_message
                    // 增量刷新 args,长 args 工具(create_or_edit_document、长 grep 等)体感连续。
                    // web 工具(webfetch/websearch)走自己的 loading 帧链路(L2102 区域),
                    // 这里跳过避免双卡。
                    // todowrite 走 BYOP todo 拦截器,合成 Message::UpdateTodos 触发 chip,
                    // 这里也跳过占位避免出现一张无意义的"调用 todowrite"卡。
                    if call.fn_name != tools::webfetch::TOOL_NAME
                        && call.fn_name != tools::websearch::TOOL_NAME
                        && call.fn_name != tools::todowrite::TOOL_NAME
                    {
                        if let Some(msg_id) = tool_msg_ids.get(&call.call_id).cloned() {
                            // 已 emit 占位 → 节流增量刷新。
                            let now = Instant::now();
                            let last = tool_last_update.get(&call.call_id).copied();
                            let elapsed_ok = last
                                .map(|t| now.duration_since(t).as_millis() as u64 >= TOOL_ARGS_UPDATE_THROTTLE_MS)
                                .unwrap_or(true);
                            if elapsed_ok {
                                if let Ok(parsed) =
                                    parse_incoming_tool_call(&call, mcp_context.as_ref())
                                {
                                    let mut updated = make_tool_call_message(
                                        &current_task_id,
                                        &request_id,
                                        &call.call_id,
                                        parsed,
                                    );
                                    updated.id = msg_id;
                                    tool_last_update.insert(call.call_id.clone(), now);
                                    yield Ok(make_update_message_event(
                                        &current_task_id,
                                        updated,
                                        vec!["tool_call".to_owned()],
                                    ));
                                }
                                // reparse 失败(intermediate 状态):静默,等下次 chunk。
                            }
                        } else if let Ok(parsed) =
                            parse_incoming_tool_call(&call, mcp_context.as_ref())
                        {
                            // 首次 parse 成功 → 立即 emit 占位卡。
                            // 每个 chunk 在未 emit 占位前都会重 parse(即"retry on every
                            // chunk"),所以即便首帧 args 不全,后续任意 chunk 完整时
                            // 都会立刻触发占位 emit—— 这就是 P1-4 的覆盖路径,
                            // 不再需要 generic placeholder variant。
                            let msg_id = Uuid::new_v4().to_string();
                            let mut placeholder = make_tool_call_message(
                                &current_task_id,
                                &request_id,
                                &call.call_id,
                                parsed,
                            );
                            placeholder.id = msg_id.clone();
                            tool_msg_ids.insert(call.call_id.clone(), msg_id);
                            tool_last_update.insert(
                                call.call_id.clone(),
                                Instant::now(),
                            );
                            yield Ok(make_add_messages_event(
                                &current_task_id,
                                vec![placeholder],
                            ));
                        }
                        // 首帧 parse 失败(args 还不完整 / 未知工具):暂不 emit,
                        // 等下次 chunk 再尝试或 End 时走老路径,避免视觉抖动。
                    }
                    // 同一 call_id 多次 chunk:后到的覆盖(genai 已合并 args)。
                    if !tool_bufs.contains_key(&call.call_id) {
                        tool_order.push(call.call_id.clone());
                    }
                    tool_bufs.insert(call.call_id.clone(), call);
                }
                ChatStreamEvent::End(end) => {
                    end_count += 1;
                    // genai >= 0.4.0 的 captured_content 含 tool_calls。
                    // 优先用 captured_content 里的 tool_calls(更完整),
                    // 否则用 streaming 中累积的 tool_bufs。
                    if let Some(content) = end.captured_content.as_ref() {
                        let mut captured_order: Vec<String> = Vec::new();
                        for call in content.tool_calls() {
                            if !captured_order.contains(&call.call_id) {
                                captured_order.push(call.call_id.clone());
                            }
                            tool_bufs.insert(call.call_id.clone(), call.clone());
                        }
                        if !captured_order.is_empty() {
                            for call_id in &tool_order {
                                if !captured_order.contains(call_id) {
                                    captured_order.push(call_id.clone());
                                }
                            }
                            tool_order = captured_order;
                        }
                    }
                    if let Some(usage) = end.captured_usage.as_ref() {
                        // 多次 End 取最大值兜底(理论上单次 stream 只有一次 End)。
                        if let Some(p) = usage.prompt_tokens {
                            captured_prompt_tokens = captured_prompt_tokens.max(p);
                        }
                        if let Some(c) = usage.completion_tokens {
                            captured_completion_tokens = captured_completion_tokens.max(c);
                        }
                        // P0-6 prompt cache 命中率监控:Anthropic / OpenAI / Gemini 在
                        // `prompt_tokens_details` 中分别返回 `cache_read_input_tokens`
                        // (Anthropic) / `cached_tokens`(OpenAI) / `cachedContentTokenCount`
                        // (Gemini)。genai 已统一映射到 `cached_tokens`。
                        // 同样 `cache_creation_tokens` 仅 Anthropic 提供(写入计费提示)。
                        // 多次 End 取最大值兜底,语义同 prompt/completion。
                        if let Some(details) = usage.prompt_tokens_details.as_ref() {
                            if let Some(r) = details.cached_tokens {
                                captured_cache_read_tokens =
                                    captured_cache_read_tokens.max(r);
                            }
                            if let Some(w) = details.cache_creation_tokens {
                                captured_cache_create_tokens =
                                    captured_cache_create_tokens.max(w);
                            }
                        }
                    }
                    saved_stream_end = Some(end);
                }
                _ => {
                    other_count += 1;
                    // ThoughtSignatureChunk 等暂不处理(Gemini 3 thoughts 需要回传给后续轮次,
                    // 当前 BYOP 不持久化 thought_signatures,接受降级)
                }
            }
        }

        // 流统计 INFO log。chunk_count=0 && tool_count=0 时上游返回为空,
        // 大概率是 model_id 不被识别 / max_tokens 缺失 / Anthropic API 兼容代理返回 200 但 body 空。
        let total_tools = tool_bufs.len();
        log::info!(
            "[byop] stream stats: start={start_count} chunks={chunk_count} ({chunk_bytes}B) \
             reasoning={reasoning_count} ({reasoning_bytes}B) tool_chunks={tool_chunk_count} \
             ends={end_count} other={other_count} captured_tools={total_tools}"
        );
        // P0-6 prompt cache 命中率日志(只在 provider 返回 cache 字段时打)。
        // ratio = cache_read / (prompt_tokens.max(1)) 表示本轮 input 中有多少比例直接
        // 命中了缓存。create > 0 表示本轮有 cache write,write 价 ≈ 1.25x base(5m)或
        // 2x base(1h)。read 价 ≈ 0.1x base,长期看只要 ≥ 1 次复用就回本。
        // 用 ratio 判定 P0 优化是否生效:同一对话第 2+ 轮应当看到 ratio 显著上升。
        //
        // **P2-16**:额外拼一个 `compaction=` 标识。压缩本身会重写历史使 messages
        // prefix 跨压缩之前后不一致 → 压缩后首轮必然 cache miss。在日志里输出该
        // 信号让后期分析(`script/analyze-prompt-cache.ps1`)能区分“正常 miss”与
        // “压缩导致 miss”,避免误伤。
        if captured_cache_read_tokens > 0 || captured_cache_create_tokens > 0 {
            let denom = captured_prompt_tokens.max(1);
            let read_ratio = captured_cache_read_tokens as f32 / denom as f32;
            let create_ratio = captured_cache_create_tokens as f32 / denom as f32;
            // 压缩状态:none → 未启用 / inactive → 启用但本轮未变化 /
            // active(已 hide 的 message id 个数) → 本轮走了压缩路径。
            let compaction_label = match params.compaction_state.as_ref() {
                None => "none".to_owned(),
                Some(s) => {
                    let hidden = s.hidden_message_ids().len();
                    if hidden == 0 {
                        "inactive".to_owned()
                    } else {
                        format!("active(hidden={hidden})")
                    }
                }
            };
            log::info!(
                "[byop-cache] prompt_tokens={captured_prompt_tokens} \
                 cache_read={captured_cache_read_tokens} ({:.1}%) \
                 cache_create={captured_cache_create_tokens} ({:.1}%) \
                 model={model_id} compaction={compaction_label}",
                read_ratio * 100.0,
                create_ratio * 100.0,
            );
        }
        if chunk_count == 0 && reasoning_count == 0 && total_tools == 0 {
            log::warn!(
                "[byop] stream returned 0 content / 0 reasoning / 0 tool_calls — \
                 上游可能返回空响应(model_id 错? max_tokens 缺? proxy 异常?)"
            );
        }

        // 流结束:把累积的 tool_calls 一次性发出。
        let mut final_messages: Vec<api::Message> = Vec::new();
        let mut ordered_tool_calls: Vec<ToolCall> = Vec::with_capacity(tool_bufs.len());
        for call_id in tool_order {
            if let Some(call) = tool_bufs.remove(&call_id) {
                ordered_tool_calls.push(call);
            }
        }
        let mut unordered_tool_calls: Vec<ToolCall> = tool_bufs.into_values().collect();
        if !unordered_tool_calls.is_empty() {
            // 正常路径下 ChunkArgs / End 两处都会同步维护 `tool_order`,所以走到这里
            // `tool_bufs` 应为空。仅在 provider 异常(例如 captured_content 与 ChunkArgs
            // 互相缺失某 call_id) 才会命中此 fallback。dict-sort 能保证 OpenAI 兼容
            // 路径 `tool_calls[]` 顺序跨调用稳定(不漂移 cache prefix),但应告警。
            log::warn!(
                "[byop] {} tool_calls fell through to dict-sort fallback — \
                 provider inconsistency between ChunkArgs and captured_content; \
                 call_ids={:?}",
                unordered_tool_calls.len(),
                unordered_tool_calls.iter().map(|t| t.call_id.as_str()).collect::<Vec<_>>(),
            );
        }
        unordered_tool_calls.sort_by(|a, b| a.call_id.cmp(&b.call_id));
        ordered_tool_calls.extend(unordered_tool_calls);
        for call in ordered_tool_calls {
            // 诊断:dump 模型实际发的 tool_call raw payload
            // (call_id / fn_name / fn_arguments JSON 原文 + 类型标注),
            // 便于核对模型是否按 schema 出入参(常见问题:bool 字段被字符串化、
            // 数字被加引号、嵌套对象塌成字符串等)。
            // debug 级:只在排查 schema 问题时开 RUST_LOG=debug,平时不污染 INFO。
            // info 级保留一行不带 args 的简短摘要,便于看流式时序。
            log::info!(
                "[byop] tool_call_in: name={} call_id={}",
                call.fn_name,
                call.call_id,
            );
            if log::log_enabled!(log::Level::Debug) {
                let args_repr = if call.fn_arguments.is_string() {
                    format!("string({:?})", call.fn_arguments.as_str().unwrap_or(""))
                } else {
                    format!(
                        "{}({})",
                        match &call.fn_arguments {
                            Value::Object(_) => "object",
                            Value::Array(_) => "array",
                            Value::Bool(_) => "bool",
                            Value::Number(_) => "number",
                            Value::Null => "null",
                            Value::String(_) => "string",
                        },
                        call.fn_arguments
                    )
                };
                log::debug!(
                    "[byop] tool_call_in_args: name={} call_id={} args={}",
                    call.fn_name,
                    call.call_id,
                    args_repr,
                );
            }

            // Marb BYOP todowrite 拦截:不映射到 protobuf executor,合成
            // `Message::UpdateTodos` 直接写 conversation.todo_lists 触发 chip + popup
            // UI(对齐 server-side ClientAction::AddMessagesToTask::UpdateTodos 路径)。
            // 然后追加 carrier ToolCall + ToolCallResult 给模型 unblock。
            if call.fn_name == tools::todowrite::TOOL_NAME {
                let args_str = if call.fn_arguments.is_string() {
                    call.fn_arguments.as_str().unwrap_or("").to_owned()
                } else {
                    call.fn_arguments.to_string()
                };

                match tools::todowrite::build_update_todos_messages(
                    &args_str,
                    &current_task_id,
                    &request_id,
                ) {
                    Ok(todo_msgs) if !todo_msgs.is_empty() => {
                        // 直接 yield UpdateTodos 让 UI 实时更新 chip。
                        // 走 AddMessagesToTask:apply_client_action 路径会
                        // 命中 Message::UpdateTodos 分支 → update_todo_list_from_todo_op
                        // → emit BlocklistAIHistoryEvent::UpdatedTodoList,UI 自动刷新。
                        yield Ok(make_add_messages_event(&current_task_id, todo_msgs));
                        let result_payload =
                            tools::todowrite::success_result_to_json("todo list updated");
                        let result_content = serde_json::to_string(&result_payload)
                            .unwrap_or_else(|_| r#"{"status":"ok"}"#.to_owned());
                        final_messages.push(make_tool_call_carrier_message(
                            &current_task_id,
                            &request_id,
                            &call.call_id,
                            &call.fn_name,
                            &args_str,
                        ));
                        final_messages.push(make_tool_call_result_message(
                            &current_task_id,
                            &request_id,
                            call.call_id.clone(),
                            result_content,
                        ));
                    }
                    Ok(_) => {
                        // 空 todos 数组:不 emit UpdateTodos,但仍要给模型 result
                        // 否则下一轮 chat 会卡(模型等 tool_call_id 的 result)。
                        let result_payload = tools::todowrite::success_result_to_json("no todos");
                        let result_content = serde_json::to_string(&result_payload)
                            .unwrap_or_else(|_| r#"{"status":"ok","message":"no todos"}"#.to_owned());
                        final_messages.push(make_tool_call_carrier_message(
                            &current_task_id,
                            &request_id,
                            &call.call_id,
                            &call.fn_name,
                            &args_str,
                        ));
                        final_messages.push(make_tool_call_result_message(
                            &current_task_id,
                            &request_id,
                            call.call_id.clone(),
                            result_content,
                        ));
                    }
                    Err(e) => {
                        // args 解析失败:跟 from_args 失败一样,emit error tool_result。
                        log::warn!(
                            "[byop] todowrite args parse failed: call_id={} err={e:#}",
                            call.call_id
                        );
                        let error_payload = tools::todowrite::invalid_arguments_result_to_json(
                            e.to_string(),
                            &args_str,
                        );
                        let error_content = serde_json::to_string(&error_payload)
                            .unwrap_or_else(|_| r#"{"error":"invalid_arguments"}"#.to_owned());
                        final_messages.push(make_tool_call_carrier_message(
                            &current_task_id,
                            &request_id,
                            &call.call_id,
                            &call.fn_name,
                            &args_str,
                        ));
                        final_messages.push(make_tool_call_result_message(
                            &current_task_id,
                            &request_id,
                            call.call_id.clone(),
                            error_content,
                        ));
                    }
                }
                intercepted_tool_responses.push(genai::chat::ToolResponse::new(call.call_id.clone(), final_messages.last().map(|m| m.server_message_data.clone()).unwrap_or_default()));
                continue;
            }

            // Marb BYOP web 工具拦截:webfetch / websearch 不映射到 protobuf
            // executor variant,在这里直接跑本地 HTTP,合成 (carrier ToolCall,
            // ToolCallResult) 一对消息,绕开 parse_incoming_tool_call。
            //
            // UI:对齐 cloud 模式,前后各 emit 一条 `Message::WebSearch` /
            // `Message::WebFetch` 状态消息,触发 inline_action `WebSearchView` /
            // `WebFetchView` 渲染:Searching/Fetching loading 卡片 → Success(URL 列表)
            // / Error 折叠卡。这两条不进 final_messages,直接 yield 让 UI 实时更新;
            // carrier + result 仍走 final_messages 给下一轮模型推理用。
            if call.fn_name == tools::webfetch::TOOL_NAME
                || call.fn_name == tools::websearch::TOOL_NAME
            {
                let args_str = if call.fn_arguments.is_string() {
                    call.fn_arguments.as_str().unwrap_or("").to_owned()
                } else {
                    call.fn_arguments.to_string()
                };
                let is_search = call.fn_name == tools::websearch::TOOL_NAME;

                // 预解析 args 抽 query / url 给 UI loading 卡。args 解析失败也要 emit
                // (用空字段兜底),保证 UI 至少看到一帧 loading,后续 dispatch
                // 仍会返回 invalid_arguments → 切到 Error 卡。
                let preview_query = if is_search {
                    serde_json::from_str::<tools::web_runtime::SearchToolArgs>(&args_str)
                        .map(|a| a.query)
                        .unwrap_or_default()
                } else {
                    String::new()
                };
                let preview_urls: Vec<String> = if !is_search {
                    serde_json::from_str::<tools::web_runtime::FetchArgs>(&args_str)
                        .map(|a| vec![a.url])
                        .unwrap_or_default()
                } else {
                    Vec::new()
                };

                // Searching/Fetching loading 帧与最终 Success/Error 帧必须共用同一个
                // message.id —— `block.rs::handle_web_search_messages` 按 id 复用
                // WebSearchView,id 不同会创建两张独立卡。
                let web_msg_id = Uuid::new_v4().to_string();
                let mut loading_msg = if is_search {
                    make_web_search_searching_message(
                        &current_task_id,
                        &request_id,
                        preview_query.clone(),
                    )
                } else {
                    make_web_fetch_fetching_message(
                        &current_task_id,
                        &request_id,
                        preview_urls.clone(),
                    )
                };
                loading_msg.id = web_msg_id.clone();
                yield Ok(make_add_messages_event(&current_task_id, vec![loading_msg]));

                let result_json = dispatch_byop_web_tool(&call.fn_name, &args_str).await;

                let mut done_msg = if is_search {
                    make_web_search_status_from_result(
                        &current_task_id,
                        &request_id,
                        &preview_query,
                        &result_json,
                    )
                } else {
                    make_web_fetch_status_from_result(
                        &current_task_id,
                        &request_id,
                        &preview_urls,
                        &result_json,
                    )
                };
                done_msg.id = web_msg_id;
                // 第二帧不能再用 AddMessagesToTask —— 那会往 task.messages 追加第二条
                // 同 id 记录,`output.rs::WebSearch` 渲染分支按 message 数量 add_child,
                // 显示成两张并排卡。改用 UpdateTaskMessage + FieldMask:`task::upsert_message`
                // 找到同 id 现有 message 后走 FieldMaskOperation::update 原地合并,
                // task.messages 仍只有一条 → UI 一张卡 set_status 切换。
                let mask_path = if is_search { "web_search" } else { "web_fetch" };
                yield Ok(make_update_message_event(
                    &current_task_id,
                    done_msg,
                    vec![mask_path.to_owned()],
                ));

                let result_content = serde_json::to_string(&result_json)
                    .unwrap_or_else(|_| r#"{"status":"serialize_error"}"#.to_owned());
                final_messages.push(make_tool_call_carrier_message(
                    &current_task_id,
                    &request_id,
                    &call.call_id,
                    &call.fn_name,
                    &args_str,
                ));
                final_messages.push(make_tool_call_result_message(
                    &current_task_id,
                    &request_id,
                    call.call_id.clone(),
                    result_content.clone(),
                ));
                intercepted_tool_responses.push(genai::chat::ToolResponse::new(call.call_id.clone(), result_content));
                continue;
            }

            match parse_incoming_tool_call(&call, mcp_context.as_ref()) {
                Ok(warp_tool) => {
                    has_non_intercepted_tool = true;
                    // 如果 ToolCallChunk 阶段已经 emit 过占位卡(同 call_id),
                    // 改用 update_message 原地刷新为最终 args(覆盖 chunk 中可能后到
                    // 的 args delta)。占位与终帧共用同一 message.id,
                    // task::upsert_message 走 FieldMaskOperation::update,
                    // task.messages 仍只有一条 → UI 一张卡 in-place 刷新,不双卡。
                    if let Some(msg_id) = tool_msg_ids.get(&call.call_id).cloned() {
                        let mut updated = make_tool_call_message(
                            &current_task_id,
                            &request_id,
                            &call.call_id,
                            warp_tool,
                        );
                        updated.id = msg_id;
                        yield Ok(make_update_message_event(
                            &current_task_id,
                            updated,
                            vec!["tool_call".to_owned()],
                        ));
                    } else {
                        final_messages.push(make_tool_call_message(
                            &current_task_id,
                            &request_id,
                            &call.call_id,
                            warp_tool,
                        ));
                    }
                }
                Err(e) => {
                    // 关键:不再把 from_args 失败吞成纯文本(原实现:emit AgentOutput),
                    // 因为模型那一轮以为自己调了 tool 在等 result,看到一段中文 assistant 文字
                    // 完全不知道是参数类型错,无法定向修正重试。
                    // 改成 emit 一对 ToolCall(carrier) + ToolCallResult(error JSON),
                    // 让模型在下一轮看到标准 tool_result error,可以按惯例改 args 重试或换工具。
                    //
                    // ToolCall 的 `tool` oneof 留 None(没有合适的结构化 variant),原始
                    // fn_name + args_str 通过 server_message_data 携带,
                    // serialize_outgoing_tool_call 的 carrier 分支会优先还原。
                    let args_str = if call.fn_arguments.is_string() {
                        call.fn_arguments.as_str().unwrap_or("").to_owned()
                    } else {
                        call.fn_arguments.to_string()
                    };
                    log::warn!(
                        "[byop] tool_call parse failed → emit synthetic error tool_result: \
                         tool={} call_id={} err={e:#}",
                        call.fn_name,
                        call.call_id
                    );
                    let error_payload = serde_json::json!({
                        "error": "invalid_arguments",
                        "detail": e.to_string(),
                        "tool": call.fn_name,
                        "received_args": &args_str,
                        "hint": "Arguments did not match the tool's JSON Schema. \
                                 Re-emit the tool call with corrected types / required fields, \
                                 or pick a different tool.",
                    });
                    let error_content = serde_json::to_string(&error_payload)
                        .unwrap_or_else(|_| r#"{"error":"invalid_arguments"}"#.to_owned());
                    final_messages.push(make_tool_call_carrier_message(
                        &current_task_id,
                        &request_id,
                        &call.call_id,
                        &call.fn_name,
                        &args_str,
                    ));
                    final_messages.push(make_tool_call_result_message(
                        &current_task_id,
                        &request_id,
                        call.call_id.clone(),
                        error_content,
                    ));
                }
            }
        }
        if !final_messages.is_empty() {
            yield Ok(make_add_messages_event(&current_task_id, final_messages));
        }

        // BYOP agentic loop: if ALL tool_calls were locally-intercepted,
        // append results to chat_req and re-invoke model.
        if !intercepted_tool_responses.is_empty() && !has_non_intercepted_tool && agentic_loop_iter < 8 {
            agentic_loop_iter += 1;
            if let Some(end) = saved_stream_end.take() {
                if let Some(content) = end.captured_content {
                    chat_req.messages.push(genai::chat::ChatMessage::assistant(content));
                }
                for tr in intercepted_tool_responses.drain(..) {
                    chat_req.messages.push(genai::chat::ChatMessage::from(tr));
                }
            }
            log::info!("[byop] agentic loop: re-invoking model after locally-intercepted tools");
            continue;
        }


        // 把 captured token usage 折算成 ConversationUsageMetadata.context_window_usage
        // 注入 StreamFinished — controller 的 handle_response_stream_finished 会把它写到
        // conversation.conversation_usage_metadata,footer 监听 UpdatedStreamingExchange/
        // AppendedExchange 事件即在每轮末实时刷新 "X% context remaining" 工具提示。
        let usage_metadata = context_window.and_then(|cw| {
            if cw == 0 || (captured_prompt_tokens == 0 && captured_completion_tokens == 0) {
                return None;
            }
            let used = (captured_prompt_tokens + captured_completion_tokens).max(0) as f32;
            let pct = (used / cw as f32).clamp(0.0, 1.0);
            log::info!(
                "[byop] context usage: prompt={} completion={} window={} → {:.1}%",
                captured_prompt_tokens,
                captured_completion_tokens,
                cw,
                pct * 100.0
            );
            Some(api::response_event::stream_finished::ConversationUsageMetadata {
                context_window_usage: pct,
                summarized: false,
                credits_spent: 0.0,
                #[allow(deprecated)]
                token_usage: Vec::new(),
                tool_usage_metadata: None,
                warp_token_usage: std::collections::HashMap::new(),
                byok_token_usage: std::collections::HashMap::new(),
                custom_endpoint_token_usage: std::collections::HashMap::new(),
                platform_credits_spent: 0.0,
            })
        });
        yield Ok(make_finished_done(usage_metadata));
        break;
        } // end loop
    };

    Ok(Box::pin(stream))
}

/// 用独立 BYOP 配置发一个短的非工具请求,让模型对首条 user query 生成会话标题。
/// 所有错误吞掉(返回 Err 让上游打 warn log,不影响主流程)。
///
/// 实现委托给 `oneshot::byop_oneshot_streaming_completion`,这里只负责拼 prompt 和清洗输出。
///
/// ## prompt 设计
///
/// - **system**: 见 `prompts/tasks/title_system.md`,结构化 task/rules/examples,
///   覆盖中英双语示例,显式禁止 "回答用户问题 / 拒绝 / 加引号"。
/// - **user**: 把原始 `user_query` 包在 `<user>...</user>` 里,前置一句明确的
///   "Generate a title for this conversation:",避免弱模型把 user 当主指令直接答复
///   (典型坏 case:user="你是谁" → 模型答"我是 Claude"被当作标题)。
/// - **temperature**: 0.3 — opencode title agent 用 0.5,这里更保守,降低跑题。
pub(crate) async fn generate_title_via_byop(
    tg: &TitleGenInput,
    user_query: &str,
) -> Result<Option<String>, anyhow::Error> {
    let cfg = super::oneshot::OneshotConfig {
        base_url: tg.base_url.clone(),
        api_key: tg.api_key.clone(),
        model_id: tg.model_id.clone(),
        api_type: tg.api_type,
        reasoning_effort: tg.reasoning_effort,
    };
    let system = include_str!("../prompts/tasks/title_system.md");
    let user_prompt = format!(
        "Generate a title for this conversation:\n<user>{}</user>",
        user_query
    );
    let opts = super::oneshot::OneshotOptions {
        max_chars: Some(1000),
        temperature: Some(0.5),
        ..Default::default()
    };
    let raw = super::oneshot::byop_oneshot_completion(&cfg, system, &user_prompt, &opts).await?;
    Ok(sanitize_title(&raw))
}

/// 清洗 title 文本。空字符串 → None(让上游跳过 emit)。
///
/// 处理顺序:
/// 1. 剥 `<think>...</think>` / `<reasoning>...</reasoning>` 思考块(reasoning 模型常见前缀)。
/// 2. 取首行非空内容(模型常前置"好的,标题是:"再换行给标题)。
/// 3. 剥 `Title:` / `标题:` / `Thread:` / `Subject:` 等前缀(大小写不敏感)。
/// 4. 剥首尾引号 / 反引号(中英文)。
/// 5. 去尾标点。
/// 6. 50 字符截断(按 char,保护 CJK),超过则尾部加 `…`。
fn sanitize_title(raw: &str) -> Option<String> {
    // 1. 剥 reasoning 标签(可能有多个,DOTALL 模式)。
    let mut s = raw.to_owned();
    for tag in &["think", "reasoning", "thought", "scratchpad"] {
        let open = format!("<{}>", tag);
        let close = format!("</{}>", tag);
        while let (Some(start), Some(end_rel)) =
            (s.find(&open), s.find(&close).map(|e| e + close.len()))
        {
            if end_rel <= start {
                break;
            }
            s.replace_range(start..end_rel, "");
        }
    }

    // 2. 取首行非空。
    let first_line = s
        .lines()
        .map(|l| l.trim())
        .find(|l| !l.is_empty())
        .unwrap_or("")
        .to_owned();
    let mut s = first_line;

    // 3. 剥前缀(循环剥,处理 "Title: 标题: foo" 这类双前缀)。
    let prefixes = [
        "title:",
        "subject:",
        "thread:",
        "标题:",
        "标题：",
        "主题:",
        "主题：",
    ];
    loop {
        let lower = s.to_lowercase();
        let mut stripped = false;
        for p in &prefixes {
            if lower.starts_with(p) {
                s = s[p.len()..].trim_start().to_owned();
                stripped = true;
                break;
            }
        }
        if !stripped {
            break;
        }
    }

    // 4. 剥首尾引号(中英文)。
    let quotes = ['"', '\'', '`', '“', '”', '‘', '’', '《', '》', '「', '」'];
    while let Some(c) = s.chars().next() {
        if quotes.contains(&c) {
            s.remove(0);
        } else {
            break;
        }
    }
    while let Some(c) = s.chars().last() {
        if quotes.contains(&c) {
            let new_len = s.len() - c.len_utf8();
            s.truncate(new_len);
        } else {
            break;
        }
    }

    // 5. 去尾标点。
    while let Some(c) = s.chars().last() {
        if matches!(
            c,
            '.' | '。' | '!' | '！' | '?' | '？' | ',' | '，' | ';' | '；' | ':' | '：'
        ) {
            let new_len = s.len() - c.len_utf8();
            s.truncate(new_len);
        } else {
            break;
        }
    }

    let s = s.trim().to_owned();
    if s.is_empty() {
        return None;
    }

    // 6. 50 字符截断(按 char,保护 CJK)。超长加省略号。
    const MAX_CHARS: usize = 50;
    let chars: Vec<char> = s.chars().collect();
    if chars.len() > MAX_CHARS {
        let mut truncated: String = chars.iter().take(MAX_CHARS - 1).collect();
        truncated.push('…');
        Some(truncated)
    } else {
        Some(s)
    }
}

/// BYOP web tool local dispatcher (webfetch / websearch).
async fn dispatch_byop_web_tool(tool_name: &str, args_str: &str) -> Value {
    use tools::web_runtime;
    let client = match web_runtime::build_ssrf_safe_client() {
        Ok(c) => c,
        Err(e) => {
            log::warn!("[byop] reqwest client build failed: {e:#}");
            return web_runtime::error_to_json(tool_name, &anyhow::anyhow!(e.to_string()));
        }
    };
    if tool_name == tools::webfetch::TOOL_NAME {
        match serde_json::from_str::<web_runtime::FetchArgs>(args_str) {
            Ok(args) => match web_runtime::run_webfetch(&client, args).await {
                Ok(out) => web_runtime::fetch_output_to_json(&out),
                Err(e) => {
                    log::warn!("[byop][webfetch] error: {e:#}");
                    web_runtime::error_to_json(tool_name, &e)
                }
            },
            Err(e) => web_runtime::error_to_json(
                tool_name,
                &anyhow::anyhow!(format!("invalid arguments: {e}")),
            ),
        }
    } else {
        match serde_json::from_str::<web_runtime::SearchToolArgs>(args_str) {
            Ok(args) => {
                let api_key = std::env::var("EXA_API_KEY").ok();
                match web_runtime::run_websearch(&client, args, api_key.as_deref(), None).await {
                    Ok(out) => web_runtime::search_output_to_json(&out),
                    Err(e) => {
                        log::warn!("[byop][websearch] error: {e:#}");
                        web_runtime::error_to_json(tool_name, &e)
                    }
                }
            }
            Err(e) => web_runtime::error_to_json(
                tool_name,
                &anyhow::anyhow!(format!("invalid arguments: {e}")),
            ),
        }
    }
}

fn parse_incoming_tool_call(
    call: &ToolCall,
    mcp_ctx: Option<&crate::ai::agent::MCPContext>,
) -> anyhow::Result<api::message::tool_call::Tool> {
    let args_str = if call.fn_arguments.is_string() {
        call.fn_arguments.as_str().unwrap_or("").to_owned()
    } else {
        call.fn_arguments.to_string()
    };
    if tools::mcp::is_mcp_function(&call.fn_name) {
        return tools::mcp::parse_mcp_tool_call(&call.fn_name, &args_str, mcp_ctx);
    }
    let Some(tool) = tools::lookup(&call.fn_name) else {
        anyhow::bail!("unknown tool name: {}", call.fn_name);
    };
    match (tool.from_args)(&args_str) {
        Ok(t) => Ok(t),
        Err(e) => {
            let schema = (tool.parameters)();
            if let Some(coerced) = tools::coerce::coerce_args_against_schema(&args_str, &schema) {
                match (tool.from_args)(&coerced) {
                    Ok(t) => {
                        log::info!(
                            "[byop] from_args coerced ok: tool={} original_err={e:#}",
                            call.fn_name
                        );
                        return Ok(t);
                    }
                    Err(e2) => {
                        log::warn!(
                            "[byop] from_args failed (after coerce): tool={} err={e2:#} original_err={e:#} coerced_args={coerced} args_str={args_str}",
                            call.fn_name
                        );
                        return Err(e2);
                    }
                }
            }
            log::warn!(
                "[byop] from_args failed: tool={} err={e:#} args_str={args_str}",
                call.fn_name
            );
            Err(e)
        }
    }
}

#[cfg(test)]
mod assistant_buffer_tests {
    use super::*;
    use genai::chat::{ChatRole, ToolCall};

    fn reasoning_part(msg: &ChatMessage) -> Option<&str> {
        for p in msg.content.parts() {
            if let ContentPart::ReasoningContent(r) = p {
                return Some(r.as_str());
            }
        }
        None
    }

    /// gate=false + 真实 reasoning → **丢弃**(zerx-lab/warp #25 修复点)。
    /// Cerebras / Groq / OpenRouter 等 OpenAI-strict provider 见到字段就 400。
    #[test]
    fn no_echo_drops_real_reasoning_text() {
        let mut buf = AssistantBuffer::new(false);
        buf.text = Some("Hi".to_string());
        buf.reasoning = Some("internal thought".to_string());
        let mut msgs = Vec::new();
        buf.flush_into(&mut msgs);
        assert_eq!(msgs.len(), 1);
        assert_eq!(msgs[0].role, ChatRole::Assistant);
        assert!(
            reasoning_part(&msgs[0]).is_none(),
            "must not echo reasoning"
        );
    }

    /// gate=false + tool_calls + 真实 reasoning → tool_calls 这条也不挂 reasoning。
    #[test]
    fn no_echo_drops_reasoning_on_tool_calls_message() {
        let mut buf = AssistantBuffer::new(false);
        buf.text = Some("calling".to_string());
        buf.tool_calls = vec![ToolCall {
            call_id: "c1".to_string(),
            fn_name: "echo".to_string(),
            fn_arguments: serde_json::json!({}),
            thought_signatures: None,
        }];
        buf.reasoning = Some("planning".to_string());
        let mut msgs = Vec::new();
        buf.flush_into(&mut msgs);
        assert_eq!(msgs.len(), 2, "text + tool_calls flush 成两条");
        for m in &msgs {
            assert!(
                reasoning_part(m).is_none(),
                "any-msg reasoning must be absent"
            );
        }
    }

    /// gate=true + 真实 reasoning → 挂真实值(DeepSeek / Kimi 路径)。
    #[test]
    fn echo_keeps_real_reasoning() {
        let mut buf = AssistantBuffer::new(true);
        buf.text = Some("ok".to_string());
        buf.reasoning = Some("thinking...".to_string());
        let mut msgs = Vec::new();
        buf.flush_into(&mut msgs);
        assert_eq!(msgs.len(), 1);
        assert_eq!(reasoning_part(&msgs[0]), Some("thinking..."));
    }

    /// gate=true + 无 reasoning → 挂占位符(满足"字段必须存在"校验)。
    #[test]
    fn echo_inserts_placeholder_when_empty() {
        let mut buf = AssistantBuffer::new(true);
        buf.text = Some("ok".to_string());
        buf.reasoning = None;
        let mut msgs = Vec::new();
        buf.flush_into(&mut msgs);
        assert_eq!(msgs.len(), 1);
        assert_eq!(reasoning_part(&msgs[0]), Some(REASONING_ECHO_PLACEHOLDER));
    }

    /// gate=true + tool_calls + 真实 reasoning → text 这条占位,tool_calls 那条挂真实值。
    #[test]
    fn echo_with_tool_calls_splits_correctly() {
        let mut buf = AssistantBuffer::new(true);
        buf.text = Some("calling".to_string());
        buf.tool_calls = vec![ToolCall {
            call_id: "c1".to_string(),
            fn_name: "echo".to_string(),
            fn_arguments: serde_json::json!({}),
            thought_signatures: None,
        }];
        buf.reasoning = Some("plan".to_string());
        let mut msgs = Vec::new();
        buf.flush_into(&mut msgs);
        assert_eq!(msgs.len(), 2);
        // text 这条:占位
        assert_eq!(reasoning_part(&msgs[0]), Some(REASONING_ECHO_PLACEHOLDER));
        // tool_calls 这条:真实 reasoning + 含 ToolCall part
        assert_eq!(reasoning_part(&msgs[1]), Some("plan"));
        assert!(
            !msgs[1].content.tool_calls().is_empty(),
            "second message must carry tool_calls"
        );
    }
}

#[cfg(test)]
mod dashscope_thinking_tests {
    use super::*;
    use crate::settings::ReasoningEffortSetting as R;

    const DASHSCOPE_CN: &str = "https://dashscope.aliyuncs.com/compatible-mode/v1/";
    const DASHSCOPE_INTL: &str = "https://dashscope-intl.aliyuncs.com/compatible-mode/v1/";

    #[test]
    fn dashscope_qwen3_triggers() {
        assert!(dashscope_needs_enable_thinking(
            AgentProviderApiType::OpenAi,
            DASHSCOPE_CN,
            "qwen3-235b-a22b",
            R::High
        ));
    }

    #[test]
    fn dashscope_qwq_triggers() {
        assert!(dashscope_needs_enable_thinking(
            AgentProviderApiType::OpenAi,
            DASHSCOPE_INTL,
            "qwq-32b",
            R::Medium
        ));
    }

    #[test]
    fn dashscope_deepseek_r1_triggers() {
        assert!(dashscope_needs_enable_thinking(
            AgentProviderApiType::OpenAi,
            DASHSCOPE_CN,
            "deepseek-r1",
            R::High
        ));
    }

    #[test]
    fn dashscope_kimi_k2_thinking_excluded() {
        // opencode 注释:kimi-k2-thinking 默认就开,不重复注入
        assert!(!dashscope_needs_enable_thinking(
            AgentProviderApiType::OpenAi,
            DASHSCOPE_CN,
            "kimi-k2-thinking",
            R::High
        ));
    }

    #[test]
    fn dashscope_off_setting_skips() {
        // 用户主动关思考时尊重之
        assert!(!dashscope_needs_enable_thinking(
            AgentProviderApiType::OpenAi,
            DASHSCOPE_CN,
            "qwen3-30b",
            R::Off
        ));
    }

    #[test]
    fn dashscope_non_reasoning_model_skips() {
        // qwen-turbo / qwen2.5 等纯 chat 模型不该被注入
        assert!(!dashscope_needs_enable_thinking(
            AgentProviderApiType::OpenAi,
            DASHSCOPE_CN,
            "qwen-turbo",
            R::High
        ));
        assert!(!dashscope_needs_enable_thinking(
            AgentProviderApiType::OpenAi,
            DASHSCOPE_CN,
            "qwen2.5-72b",
            R::High
        ));
    }

    #[test]
    fn non_dashscope_url_skips() {
        // OpenAI / Cerebras / Groq 等不是 DashScope 的 base_url
        assert!(!dashscope_needs_enable_thinking(
            AgentProviderApiType::OpenAi,
            "https://api.openai.com/v1/",
            "qwen3-30b",
            R::High
        ));
        assert!(!dashscope_needs_enable_thinking(
            AgentProviderApiType::OpenAi,
            "https://api.cerebras.ai/v1/",
            "qwen3-30b",
            R::High
        ));
    }

    #[test]
    fn non_openai_api_type_skips() {
        // Anthropic / Gemini / DeepSeek api_type 不走这条路径
        assert!(!dashscope_needs_enable_thinking(
            AgentProviderApiType::Anthropic,
            DASHSCOPE_CN,
            "qwen3-30b",
            R::High
        ));
        assert!(!dashscope_needs_enable_thinking(
            AgentProviderApiType::DeepSeek,
            DASHSCOPE_CN,
            "deepseek-r1",
            R::High
        ));
    }
}

/// `build_chat_options` 中"思考深度档位下发"的回归测试。
///
/// 对齐 Zed `LanguageModelRequest::thinking_allowed=false` 在各 provider 的处理:
/// **Off 时所有 provider 都不能让服务端思考**。具体策略按 provider 不同:
/// - Anthropic / Gemini:不发 thinking 字段(跳过 `with_reasoning_effort`)
/// - DeepSeek:`extra_body.thinking.type=disabled`(服务端默认开启,需显式关)
/// - OpenAI / OpenAiResp:`reasoning_effort: "none"`(GPT-5 接受)
#[cfg(test)]
mod build_chat_options_off_tests {
    use super::*;
    use crate::settings::ReasoningEffortSetting as R;
    use genai::chat::ReasoningEffort as GE;

    fn opts(api_type: AgentProviderApiType, model: &str, effort: R) -> genai::chat::ChatOptions {
        build_chat_options(
            api_type,
            "https://example.com/v1/",
            model,
            effort,
            vec![],
            None,
        )
    }

    /// claude-sonnet-4-6(`SUPPORT_ADAPTTIVE_THINK_MODELS` 命中)+ Off 必须**完全
    /// 不传** `reasoning_effort`,否则 vendor genai adapter 会无条件插入
    /// `thinking:{type:adaptive}`(`adapter_impl.rs:121-135`)。
    #[test]
    fn anthropic_sonnet_4_6_off_skips_reasoning_effort() {
        let o = opts(AgentProviderApiType::Anthropic, "claude-sonnet-4-6", R::Off);
        assert!(
            o.reasoning_effort.is_none(),
            "Anthropic+Off 必须不传 reasoning_effort,避免 4.6 系强插 adaptive thinking"
        );
        assert!(
            o.extra_body.is_none(),
            "Anthropic+Off 也不应注入 extra_body"
        );
    }

    /// claude-opus-4-6 同上(双重命中 SUPPORT_EFFORT + SUPPORT_ADAPTIVE)。
    #[test]
    fn anthropic_opus_4_6_off_skips_reasoning_effort() {
        let o = opts(AgentProviderApiType::Anthropic, "claude-opus-4-6", R::Off);
        assert!(o.reasoning_effort.is_none());
        assert!(o.extra_body.is_none());
    }

    /// claude-opus-4-7+ + Off:虽然不在 adaptive 名单(本来就 OK),仍应一致跳过。
    #[test]
    fn anthropic_opus_4_7_off_skips_reasoning_effort() {
        let o = opts(AgentProviderApiType::Anthropic, "claude-opus-4-7", R::Off);
        assert!(o.reasoning_effort.is_none());
        assert!(o.extra_body.is_none());
    }

    /// Anthropic + High 仍走原 reasoning_effort 路径。
    #[test]
    fn anthropic_high_injects_reasoning_effort() {
        let o = opts(AgentProviderApiType::Anthropic, "claude-opus-4-7", R::High);
        assert!(matches!(o.reasoning_effort, Some(GE::High)));
    }

    /// Anthropic + Auto 不传任何参数。
    #[test]
    fn anthropic_auto_skips() {
        let o = opts(AgentProviderApiType::Anthropic, "claude-opus-4-7", R::Auto);
        assert!(o.reasoning_effort.is_none());
    }

    /// Gemini + Off:不发 thinkingConfig。
    #[test]
    fn gemini_off_skips_reasoning_effort() {
        let o = opts(AgentProviderApiType::Gemini, "gemini-2.5-pro", R::Off);
        assert!(o.reasoning_effort.is_none());
        assert!(o.extra_body.is_none());
    }

    /// Gemini + Medium 走 thinkingBudget 路径。
    #[test]
    fn gemini_medium_injects_reasoning_effort() {
        let o = opts(AgentProviderApiType::Gemini, "gemini-2.5-pro", R::Medium);
        assert!(matches!(o.reasoning_effort, Some(GE::Medium)));
    }

    /// DeepSeek + Off:必须发 `extra_body.thinking.type=disabled`,
    /// 而**不能**走 reasoning_effort=none(服务端 400 unknown variant)。
    #[test]
    fn deepseek_off_uses_extra_body_disabled() {
        let o = opts(AgentProviderApiType::DeepSeek, "deepseek-v4-flash", R::Off);
        assert!(
            o.reasoning_effort.is_none(),
            "DeepSeek+Off 不能走 reasoning_effort=none"
        );
        let body = o.extra_body.as_ref().expect("extra_body must be set");
        assert_eq!(
            body.pointer("/thinking/type"),
            Some(&serde_json::Value::String("disabled".to_string())),
            "DeepSeek+Off 必须发 thinking.type=disabled"
        );
    }

    /// DeepSeek + High 走 reasoning_effort 顶层字段。
    #[test]
    fn deepseek_high_injects_reasoning_effort() {
        let o = opts(AgentProviderApiType::DeepSeek, "deepseek-reasoner", R::High);
        assert!(matches!(o.reasoning_effort, Some(GE::High)));
        assert!(o.extra_body.is_none());
    }

    /// OpenAI(GPT-5)+ Off:走 reasoning_effort=none(GPT-5 接受 `none` 档)。
    #[test]
    fn openai_gpt5_off_uses_reasoning_effort_none() {
        let o = opts(AgentProviderApiType::OpenAi, "gpt-5", R::Off);
        assert!(
            matches!(o.reasoning_effort, Some(GE::None)),
            "OpenAI+GPT-5+Off 应发 reasoning_effort=none"
        );
    }

    /// 不支持 reasoning 的模型 + 任意非 Auto 档位:跳过(避免上游 400)。
    #[test]
    fn anthropic_haiku_3_5_off_skips() {
        let o = opts(
            AgentProviderApiType::Anthropic,
            "claude-3-5-haiku-20241022",
            R::Off,
        );
        assert!(o.reasoning_effort.is_none());
        assert!(o.extra_body.is_none());
    }

    #[test]
    fn openai_gpt4o_off_skips() {
        // gpt-4o 不在 reasoning 名单,Off 也跳过
        let o = opts(AgentProviderApiType::OpenAi, "gpt-4o", R::Off);
        assert!(o.reasoning_effort.is_none());
    }
}

/// **端到端 cache 边界稳定性测试**:验证多轮对话模拟下,prompt cache
/// 需要的“前缀字节级一致”保证。这些测试并不调用上游 API,仅检查
/// `apply_caching_anthropic` 与 `build_chat_options` 输出的确定性。
///
/// 这是 cache 命中的**最低门槛**:如果同样输入跨调用输出不一致,
/// 上游哈希必不一致 → 100% miss。反之输出一致也不能保证命中。
#[cfg(test)]
mod cache_boundary_stability_tests {
    use super::*;
    use genai::chat::{ChatMessage, ChatRole};

    /// 构造一个典型的多轮对话 messages 序列:
    /// system + user_1 + assistant_1 + user_2 + assistant_2 + user_3
    /// (末尾是 user,与 `ensure_ends_with_user` 输出一致)。
    fn build_three_turn_conversation() -> Vec<ChatMessage> {
        vec![
            ChatMessage::system(
                "You are a helpful coding assistant for Marb BYOP.\n\
                 Guidelines: be concise, prefer code over prose.",
            ),
            ChatMessage::user("What is rust borrow checker?"),
            ChatMessage::assistant("It enforces ownership rules at compile time."),
            ChatMessage::user("Show me a code example"),
            ChatMessage::assistant("```rust\nfn main() { let s = String::new(); }\n```"),
            ChatMessage::user("Explain the lifetime in that code"),
        ]
    }

    fn extract_cache_control(msg: &ChatMessage) -> Option<CacheControl> {
        // ChatMessage 的 cache_control 在 `options.cache_control` 上。
        msg.options.as_ref().and_then(|o| o.cache_control.clone())
    }

    fn cache_signature(msgs: &[ChatMessage]) -> Vec<(usize, ChatRole, Option<CacheControl>)> {
        msgs.iter()
            .enumerate()
            .map(|(i, m)| (i, m.role.clone(), extract_cache_control(m)))
            .collect()
    }

    /// **P0-4 主要验收**:apply_caching_anthropic 在同一输入上重复调用
    /// 产出的 cache 标记位置与 TTL 必须 byte-equal。
    #[test]
    fn apply_caching_anthropic_is_deterministic() {
        let mut a = build_three_turn_conversation();
        let mut b = build_three_turn_conversation();
        apply_caching_anthropic(&mut a);
        apply_caching_anthropic(&mut b);
        assert_eq!(
            cache_signature(&a),
            cache_signature(&b),
            "同输入 × 多次调用 cache 标记必须一致"
        );
    }

    /// **TTL 混合策略验收**:system 走 1h(静态前缀),非 system 走 5m(会话尾)。
    /// 顺序 system(1h) → messages(5m) 满足 Anthropic 排序约束,且对外部注入 5m 免疫。
    #[test]
    fn anthropic_cache_uses_mixed_ttl() {
        let mut msgs = build_three_turn_conversation();
        apply_caching_anthropic(&mut msgs);
        let tagged: Vec<_> = msgs
            .iter()
            .filter(|m| extract_cache_control(m).is_some())
            .collect();
        assert!(!tagged.is_empty(), "必须至少打一个 breakpoint");
        for m in &tagged {
            let cc = extract_cache_control(m).unwrap();
            let expected = if matches!(m.role, ChatRole::System) {
                CacheControl::Ephemeral1h
            } else {
                CacheControl::Ephemeral5m
            };
            assert_eq!(cc, expected, "role={:?} 的 TTL 不匹配预期", m.role);
        }
    }

    /// **P0-4 覆盖面验收**:opencode 路子 first 2 system + last 2 non-system。
    /// 多轮对话(1 个 system + 5 个 non-system)应该打上 1+2=3 个标记。
    #[test]
    fn anthropic_marks_first_2_system_and_last_2_non_system() {
        let mut msgs = build_three_turn_conversation();
        apply_caching_anthropic(&mut msgs);
        let tagged_indices: Vec<usize> = msgs
            .iter()
            .enumerate()
            .filter(|(_, m)| extract_cache_control(m).is_some())
            .map(|(i, _)| i)
            .collect();
        // 验证 system(idx=0) 与末尾 2 个 non-system(idx=4, idx=5)都被打上。
        assert!(tagged_indices.contains(&0), "首 system 未被标记");
        assert!(tagged_indices.contains(&4), "倒数第 2 条未被标记");
        assert!(tagged_indices.contains(&5), "末条未被标记");
        assert_eq!(
            tagged_indices.len(),
            3,
            "总计 3 个 breakpoint(1 system + 2 tail)"
        );
    }

    /// **模拟多轮对话中的缓存 prefix 稳定性**:
    /// turn N 的 messages 是 turn N-1 的 messages + (N-1 轮 assistant) + (新 user)。
    /// 起始部分的 cache 标记(system + 中间轮)不应随轮数增长而漂移。
    #[test]
    fn cache_marks_stable_as_conversation_grows() {
        // turn 1
        let mut t1 = vec![ChatMessage::system("sys"), ChatMessage::user("q1")];
        apply_caching_anthropic(&mut t1);
        let sys_t1_cc = extract_cache_control(&t1[0]);

        // turn 2:增加 assistant_1 + user_2
        let mut t2 = vec![
            ChatMessage::system("sys"),
            ChatMessage::user("q1"),
            ChatMessage::assistant("a1"),
            ChatMessage::user("q2"),
        ];
        apply_caching_anthropic(&mut t2);
        let sys_t2_cc = extract_cache_control(&t2[0]);

        // 首 system 的 cache_control 跨轮一致 → 表示上游哈希不变 → 后续会命中。
        assert_eq!(
            sys_t1_cc, sys_t2_cc,
            "首 system breakpoint 的 TTL/位置跨轮应一致"
        );
        // turn 1 的 user 位置被打(末尾),turn 2 不再被打。
        assert!(extract_cache_control(&t1[1]).is_some());
        assert!(
            extract_cache_control(&t2[1]).is_none(),
            "turn 2 的旧 user 不再是 tail"
        );
    }

    /// **build_chat_options 输出确定性**(同输入跨调用结果一致)。
    /// prompt cache 命中的最低门槛 \u{2014} 哈希位级一致。
    #[test]
    fn openai_chat_options_is_deterministic() {
        use crate::settings::ReasoningEffortSetting as R;
        let make = || {
            build_chat_options(
                AgentProviderApiType::OpenAi,
                "https://api.openai.com/v1/",
                "gpt-5-mini",
                R::Auto,
                vec![],
                Some("conv-abc-123"),
            )
        };
        let a = make();
        let b = make();
        assert_eq!(a.prompt_cache_key, b.prompt_cache_key);
        assert_eq!(a.cache_control, b.cache_control);
    }

    /// **opencode 兼容白名单 provider**(api.openai.com / *.openai.azure.com /
    /// openrouter.ai / api.venice.ai / opencode.ai/zen)→ 下发 prompt_cache_key,
    /// 且 **永远不下发 cache_control**(对应 prompt_cache_retention 字段;
    /// opencode 仓库不使用该字段)。
    #[test]
    fn whitelisted_provider_emits_prompt_cache_key_only() {
        use crate::settings::ReasoningEffortSetting as R;
        // 选 5 个白名单代表 URL,每个都覆盖到 api_type=OpenAi 的判定分支。
        let whitelisted = [
            "https://api.openai.com/v1/",
            "https://my-resource.openai.azure.com/openai/v1/",
            "https://openrouter.ai/api/v1/",
            "https://api.venice.ai/api/v1/",
            "https://opencode.ai/zen/v1/",
        ];
        for url in whitelisted {
            let opts = build_chat_options(
                AgentProviderApiType::OpenAi,
                url,
                "gpt-5-mini",
                R::Auto,
                vec![],
                Some("conv-1"),
            );
            assert_eq!(
                opts.prompt_cache_key.as_deref(),
                Some("conv-1"),
                "{url}: 白名单 provider 应下发 prompt_cache_key=conversation_id"
            );
            assert!(
                opts.cache_control.is_none(),
                "{url}: cache_control 永远不发(opencode 不使用 prompt_cache_retention)"
            );
        }
    }

    /// **#126 回归**:OpenAi api_type 但 base_url 不在白名单(OpenCode Go
    /// 中转 Kimi / GLM、vLLM、lm-studio、DashScope、Moonshot、智谱原生 等)
    /// → 既不下发 cache_control,也不下发 prompt_cache_key。
    ///
    /// 对齐 opencode `options()` 函数:除 openai/azure/openrouter/venice/opencode
    /// 五个 providerID 之外,任何 provider 都不设 promptCacheKey。
    #[test]
    fn non_whitelisted_provider_emits_nothing() {
        use crate::settings::ReasoningEffortSetting as R;
        // issue #126 正文 + 用户后续 comment 的两个具体例子,加上其他主流
        // OpenAI 兼容中转。每个 URL 都不应下发任何 cache 字段。
        let byop_urls = [
            ("https://opencode.go/v1/", "kimi-k2.6"),
            ("https://opencode.go/v1/", "glm-5.1"),
            ("https://api.moonshot.cn/v1/", "kimi-k2"),
            ("https://open.bigmodel.cn/api/paas/v4/", "glm-4.6"),
            (
                "https://dashscope.aliyuncs.com/compatible-mode/v1/",
                "qwen-max",
            ),
            ("http://localhost:1234/v1/", "local-model"),
        ];
        for (url, model) in byop_urls {
            let opts = build_chat_options(
                AgentProviderApiType::OpenAi,
                url,
                model,
                R::Auto,
                vec![],
                Some("conv-byop"),
            );
            assert!(
                opts.cache_control.is_none(),
                "{url}: 非白名单不应下发 cache_control"
            );
            assert!(
                opts.prompt_cache_key.is_none(),
                "{url}: 非白名单不应下发 prompt_cache_key"
            );
        }
    }

    /// OpenAiResp api_type 走同一份判定逻辑(genai openai_resp adapter 序列化
    /// 同样的字段),白名单内下发 / 非白名单屏蔽。
    #[test]
    fn openai_resp_follows_same_whitelist() {
        use crate::settings::ReasoningEffortSetting as R;
        let on_whitelist = build_chat_options(
            AgentProviderApiType::OpenAiResp,
            "https://api.openai.com/v1/",
            "gpt-5",
            R::Auto,
            vec![],
            Some("conv-resp"),
        );
        assert_eq!(on_whitelist.prompt_cache_key.as_deref(), Some("conv-resp"));
        assert!(on_whitelist.cache_control.is_none());

        let off_whitelist = build_chat_options(
            AgentProviderApiType::OpenAiResp,
            "https://custom.relay/v1/",
            "gpt-5",
            R::Auto,
            vec![],
            Some("conv-resp"),
        );
        assert!(off_whitelist.prompt_cache_key.is_none());
        assert!(off_whitelist.cache_control.is_none());
    }

    /// **conversation_id 为空跳过 prompt_cache_key**(避免跨会话误挂路由)。
    #[test]
    fn openai_empty_conversation_id_skips_cache_key() {
        use crate::settings::ReasoningEffortSetting as R;
        let opts = build_chat_options(
            AgentProviderApiType::OpenAi,
            "https://api.openai.com/v1/",
            "gpt-5",
            R::Auto,
            vec![],
            Some(""),
        );
        assert!(
            opts.prompt_cache_key.is_none(),
            "空 conversation_id 应跳过 prompt_cache_key"
        );
        assert!(opts.cache_control.is_none(), "cache_control 永远不发");
    }

    /// **Anthropic 路径 build_chat_options 不下发 cache_control**
    /// (Anthropic 走 per-message,不走 ChatOptions 级)。
    #[test]
    fn anthropic_chat_options_no_cache_control() {
        use crate::settings::ReasoningEffortSetting as R;
        let opts = build_chat_options(
            AgentProviderApiType::Anthropic,
            "https://api.anthropic.com/v1/",
            "claude-opus-4-7",
            R::Auto,
            vec![],
            Some("conv-3"),
        );
        assert!(
            opts.cache_control.is_none(),
            "Anthropic 的 ChatOptions 不能带 cache_control(走 per-message)"
        );
        assert!(
            opts.prompt_cache_key.is_none(),
            "Anthropic 不走 prompt_cache_key"
        );
    }

    /// **DeepSeek / Gemini / Ollama 服务端隐式缓存,不下发 cache_control**。
    #[test]
    fn implicit_cache_providers_no_cache_control() {
        use crate::settings::ReasoningEffortSetting as R;
        for api in [
            AgentProviderApiType::DeepSeek,
            AgentProviderApiType::Gemini,
            AgentProviderApiType::Ollama,
        ] {
            let opts = build_chat_options(
                api,
                "https://example.com/v1/",
                "some-model",
                R::Auto,
                vec![],
                Some("conv"),
            );
            assert!(
                opts.cache_control.is_none(),
                "{api:?} 不应下发 cache_control"
            );
        }
    }
}

#[cfg(test)]
mod serializer_readiness_tests {
    use super::*;
    use crate::ai::agent::task::TaskId;
    use crate::ai::agent::{AIAgentActionId, AIAgentActionResultType, RequestCommandOutputResult};
    use crate::ai::byop_compaction::state::{CompactionState, CompletedCompaction};
    use crate::ai::byop_readiness::{
        PendingByopToolResultsError, RepairRecord, RepairState, ToolCallKey, ToolCallRef,
        BLOCKED_BYOP_REQUEST_MESSAGE,
    };
    use std::collections::{HashMap, HashSet};
    use std::sync::Arc;

    fn kind() -> RedactedToolKind {
        RedactedToolKind::new("shell")
    }

    fn assistant_calls(task_id: &str, assistant_id: &str, call_ids: &[&str]) -> ProjectionItem {
        ProjectionItem::assistant_tool_calls(
            task_id,
            assistant_id,
            call_ids
                .iter()
                .map(|call_id| ProjectedToolCall::new(task_id, assistant_id, *call_id, kind()))
                .collect(),
        )
    }

    fn result(
        task_id: &str,
        message_id: &str,
        assistant_id: Option<&str>,
        call_id: &str,
        source: ToolResultSource,
    ) -> ProjectionItem {
        ProjectionItem::tool_result(ProjectedToolResult::new(
            task_id,
            message_id,
            assistant_id.map(str::to_owned),
            call_id,
            kind(),
            source,
            TerminalResultKind::Real,
        ))
    }

    fn assert_blocked_category(projection: Vec<ProjectionItem>, category: &str) {
        let error = validate_serializer_readiness_projection(projection).unwrap_err();
        assert!(
            error.to_string().contains(BLOCKED_BYOP_REQUEST_MESSAGE),
            "error should use blocked-request copy for {category}: {error}"
        );
    }

    fn shell_tool() -> api::message::tool_call::Tool {
        use api::message::tool_call::run_shell_command::WaitUntilCompleteValue;

        api::message::tool_call::Tool::RunShellCommand(api::message::tool_call::RunShellCommand {
            command: "echo hi".to_owned(),
            is_read_only: true,
            uses_pager: false,
            is_risky: false,
            citations: vec![],
            wait_until_complete_value: Some(WaitUntilCompleteValue::WaitUntilComplete(true)),
            risk_category: 0,
        })
    }

    fn subagent_tool() -> api::message::tool_call::Tool {
        api::message::tool_call::Tool::Subagent(api::message::tool_call::Subagent {
            task_id: "subtask-1".to_owned(),
            payload: String::new(),
            metadata: Some(api::message::tool_call::subagent::Metadata::Cli(
                api::message::tool_call::subagent::CliSubagent {
                    command_id: "command-1".to_owned(),
                },
            )),
        })
    }

    fn task_with_messages(messages: Vec<api::Message>) -> api::Task {
        api::Task {
            id: "task-1".to_owned(),
            messages,
            dependencies: None,
            description: String::new(),
            summary: String::new(),
            server_data: String::new(),
        }
    }

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

    fn cancelled_action_result_input(call_id: &str) -> AIAgentInput {
        AIAgentInput::ActionResult {
            result: AIAgentActionResult {
                id: AIAgentActionId::from(call_id.to_owned()),
                task_id: TaskId::new("task-1".to_owned()),
                result: AIAgentActionResultType::RequestCommandOutput(
                    RequestCommandOutputResult::CancelledBeforeExecution,
                ),
            },
            context: Arc::<[AIAgentContext]>::from([]),
        }
    }

    fn request_params(messages: Vec<api::Message>, input: Vec<AIAgentInput>) -> RequestParams {
        RequestParams::new_for_test(input, vec![task_with_messages(messages)])
    }

    fn request_params_with_repair(
        messages: Vec<api::Message>,
        input: Vec<AIAgentInput>,
        repair_state: RepairState,
    ) -> RequestParams {
        let mut params = request_params(messages, input);
        params.byop_repair_state = RepairStateStatus::Valid(repair_state);
        params
    }

    fn build_openai_request(params: &RequestParams) -> Result<ChatRequest, ConvertToAPITypeError> {
        build_chat_request(params, false, AgentProviderApiType::OpenAi, attachment_caps::AttachmentCaps::default())
    }

    fn assert_request_has_no_repair_placeholder(request: &ChatRequest) {
        assert!(
            !request.messages.iter().any(|message| {
                message
                    .content
                    .tool_responses()
                    .iter()
                    .any(|response| is_placeholder_tool_response_content(&response.content))
            }),
            "normal request body must not emit placeholder tool results"
        );
    }

    fn tool_response_contents(request: &ChatRequest, call_id: &str) -> Vec<String> {
        request
            .messages
            .iter()
            .flat_map(|message| message.content.tool_responses())
            .filter(|response| response.call_id == call_id)
            .map(|response| response.content.clone())
            .collect()
    }

    fn assert_build_request_blocked(params: RequestParams, category: &str) {
        let error = build_openai_request(&params).unwrap_err();
        assert!(
            error.to_string().contains(BLOCKED_BYOP_REQUEST_MESSAGE),
            "error should use blocked-request copy for {category}: {error}"
        );
    }

    #[test]
    fn placeholder_then_cancellation_regression_blocks_waits_or_sends_real_cancellation() {
        let user_message = make_user_query_message("task-1", "req-1", "hi".to_owned(), &[]);
        let tool_call_message = make_tool_call_message("task-1", "req-1", "call-1", shell_tool());
        let assistant_message_id = tool_call_message.id.clone();

        assert_build_request_blocked(
            request_params(
                vec![user_message.clone(), tool_call_message.clone()],
                vec![user_query_input("continue")],
            ),
            "MissingResultWithoutRepairSource",
        );

        let pending_report = classify_byop_controller_readiness_with_live_tool_calls(
            &request_params(
                vec![user_message.clone(), tool_call_message.clone()],
                vec![user_query_input("continue")],
            ),
            vec![LiveToolCall::new(
                ToolCallRef::new(
                    ToolCallKey::new("task-1", assistant_message_id, "call-1"),
                    kind(),
                ),
                LiveToolCallState::Running,
            )],
        );
        assert!(matches!(
            pending_report.state,
            ReadinessState::PendingToolResults { ref tool_calls }
                if tool_calls.len() == 1 && tool_calls[0].key.tool_call_id == "call-1"
        ));
        let ReadinessState::PendingToolResults { tool_calls } = pending_report.state else {
            panic!("expected pending tool results");
        };
        let pending_error = PendingByopToolResultsError::new(tool_calls.len());
        assert_eq!(
            pending_error.category(),
            ReadinessCategory::PendingToolResults
        );
        assert_eq!(pending_error.tool_call_count(), 1);
        assert!(
            !pending_error
                .to_string()
                .contains(BLOCKED_BYOP_REQUEST_MESSAGE),
            "pending wait must remain distinct from blocked user-facing errors"
        );

        let request = build_openai_request(&request_params(
            vec![user_message, tool_call_message],
            vec![
                cancelled_action_result_input("call-1"),
                user_query_input("continue"),
            ],
        ))
        .expect("current cancellation result should serialize as a real tool result");
        let errors = strict_chat_completions_ordering_errors(&request.messages);
        assert!(
            errors.is_empty(),
            "request body ordering errors: {errors:?}"
        );
        assert_request_has_no_repair_placeholder(&request);

        let contents = tool_response_contents(&request, "call-1");
        assert_eq!(contents.len(), 1);
        assert!(
            contents[0].to_ascii_lowercase().contains("cancel"),
            "expected real cancellation content, got {}",
            contents[0]
        );
    }

    #[test]
    fn controller_readiness_requires_cancellation_commit_before_user_boundary() {
        let tool_call_message = make_tool_call_message("task-1", "req-1", "call-1", shell_tool());
        let params = request_params(
            vec![tool_call_message],
            vec![
                cancelled_action_result_input("call-1"),
                user_query_input("continue"),
            ],
        );

        let report = classify_byop_controller_readiness(&params);

        assert!(matches!(
            report.state,
            ReadinessState::NeedsCancellationCommit { ref tool_calls }
                if tool_calls.len() == 1
                    && tool_calls[0].key.tool_call_id == "call-1"
        ));
    }

    #[test]
    fn controller_readiness_ignores_duplicate_current_cancellation_after_persistence() {
        let tool_call_message = make_tool_call_message("task-1", "req-1", "call-1", shell_tool());
        let tool_result_message = make_tool_call_result_message(
            "task-1",
            "req-1",
            "call-1".to_owned(),
            r#"{"status":"cancelled"}"#.to_owned(),
        );
        let params = request_params(
            vec![tool_call_message, tool_result_message],
            vec![
                cancelled_action_result_input("call-1"),
                user_query_input("continue"),
            ],
        );

        let report = classify_byop_controller_readiness(&params);

        assert!(matches!(report.state, ReadinessState::Ready));
    }

    #[test]
    fn controller_readiness_accepts_committed_local_interception_results() {
        let carrier =
            make_tool_call_carrier_message("task-1", "req-1", "call-1", "todowrite", "{}");
        let local_result = make_tool_call_result_message(
            "task-1",
            "req-1",
            "call-1".to_owned(),
            r#"{"_byop_intercepted":true,"status":"ok"}"#.to_owned(),
        );
        let invalid_arguments_result = make_tool_call_result_message(
            "task-1",
            "req-1",
            "call-2".to_owned(),
            r#"{"error":"invalid_arguments","tool":"dummy"}"#.to_owned(),
        );
        let invalid_arguments_carrier =
            make_tool_call_carrier_message("task-1", "req-1", "call-2", "dummy", "{}");
        let params = request_params(
            vec![
                carrier,
                local_result,
                invalid_arguments_carrier,
                invalid_arguments_result,
            ],
            vec![],
        );

        let report = classify_byop_controller_readiness(&params);

        assert!(matches!(report.state, ReadinessState::Ready));
    }

    #[test]
    fn controller_readiness_blocks_unreadable_local_interception_payload() {
        let carrier =
            make_tool_call_carrier_message("task-1", "req-1", "call-1", "todowrite", "{}");
        let unreadable_result = make_tool_call_result_message(
            "task-1",
            "req-1",
            "call-1".to_owned(),
            r#"{"_byop_intercepted":true,"status":"ok""#.to_owned(),
        );
        let params = request_params(vec![carrier, unreadable_result], vec![]);

        let report = classify_byop_controller_readiness(&params);

        assert!(matches!(
            report.state,
            ReadinessState::MissingResultWithoutRepairSource {
                reason: crate::ai::byop_readiness::MissingResultReason::UnreadableLocalInterception,
                ..
            }
        ));
    }

    #[test]
    fn controller_readiness_reports_pending_live_action() {
        let tool_call_message = make_tool_call_message("task-1", "req-1", "call-1", shell_tool());
        let assistant_message_id = tool_call_message.id.clone();
        let params = request_params(vec![tool_call_message], vec![]);

        let report = classify_byop_controller_readiness_with_live_tool_calls(
            &params,
            vec![LiveToolCall::new(
                ToolCallRef::new(
                    ToolCallKey::new("task-1", assistant_message_id, "call-1"),
                    kind(),
                ),
                LiveToolCallState::Running,
            )],
        );

        assert!(matches!(
            report.state,
            ReadinessState::PendingToolResults { ref tool_calls }
                if tool_calls.len() == 1 && tool_calls[0].key.tool_call_id == "call-1"
        ));
    }

    #[test]
    fn normal_flow_missing_result_blocks_before_placeholder_repair() {
        assert_blocked_category(
            vec![assistant_calls("task-1", "assistant-1", &["call-1"])],
            "MissingResultWithoutRepairSource",
        );
    }

    #[test]
    fn duplicate_orphan_and_out_of_order_results_block_serialization() {
        assert_blocked_category(
            vec![
                assistant_calls("task-1", "assistant-1", &["call-1"]),
                result(
                    "task-1",
                    "result-1",
                    Some("assistant-1"),
                    "call-1",
                    ToolResultSource::PersistedHistory,
                ),
                result(
                    "task-1",
                    "result-2",
                    Some("assistant-1"),
                    "call-1",
                    ToolResultSource::PersistedHistory,
                ),
            ],
            "DuplicateToolResults",
        );
        assert_blocked_category(
            vec![result(
                "task-1",
                "result-1",
                Some("assistant-missing"),
                "call-1",
                ToolResultSource::PersistedHistory,
            )],
            "OrphanToolResult",
        );
        assert_blocked_category(
            vec![
                assistant_calls("task-1", "assistant-1", &["call-1"]),
                ProjectionItem::user_boundary("task-1", "user-2"),
                result(
                    "task-1",
                    "result-1",
                    Some("assistant-1"),
                    "call-1",
                    ToolResultSource::PersistedHistory,
                ),
            ],
            "OutOfOrderToolResult",
        );
    }

    #[test]
    fn visible_boundaries_block_pending_tool_groups_but_filtered_messages_do_not() {
        assert_blocked_category(
            vec![
                assistant_calls("task-1", "assistant-1", &["call-1"]),
                ProjectionItem::other_boundary("task-1", "visible-other"),
            ],
            "MissingResultWithoutRepairSource",
        );

        let report = validate_serializer_readiness_projection(vec![
            assistant_calls("task-1", "assistant-1", &["call-1"]),
            result(
                "task-1",
                "result-1",
                Some("assistant-1"),
                "call-1",
                ToolResultSource::PersistedHistory,
            ),
        ])
        .expect("filtered-out messages should not affect readiness");
        assert_eq!(report.state, ReadinessState::Ready);
    }

    #[test]
    fn current_input_action_result_satisfies_serializer_readiness() {
        let report = validate_serializer_readiness_projection(vec![
            assistant_calls("task-1", "assistant-1", &["call-1"]),
            result(
                "task-1",
                "current_input:0:call-1",
                None,
                "call-1",
                ToolResultSource::CurrentInput,
            ),
        ])
        .expect("current input result should satisfy a visible tool call");
        assert_eq!(report.state, ReadinessState::Ready);
    }

    #[test]
    fn accepted_history_repair_is_sendable_but_distinct_from_ready() {
        let repair = RepairRecord::new(
            RepairSource::ForkedHistory,
            ToolCallKey::new("task-1", "assistant-1", "call-1"),
        );
        let report = validate_serializer_readiness_projection_with_repair_state(
            vec![assistant_calls("task-1", "assistant-1", &["call-1"])],
            &RepairStateStatus::Valid(RepairState::new(vec![repair.clone()])),
            &ReadinessDiagnosticContext::new(
                "test-conversation",
                "test-attempt",
                ReadinessTriggerLayer::SerializerValidation,
            ),
        )
        .expect("accepted repair should be sendable");

        assert_eq!(
            report.state,
            ReadinessState::AcceptedHistoryRepair {
                repairs: vec![AcceptedRepair {
                    record: repair,
                    tool_call: ToolCallRef::new(
                        ToolCallKey::new("task-1", "assistant-1", "call-1"),
                        kind(),
                    ),
                }],
            }
        );
    }

    #[test]
    fn invalid_repair_sidecar_does_not_block_ready_projection() {
        let report = validate_serializer_readiness_projection_with_repair_state(
            vec![
                assistant_calls("task-1", "assistant-1", &["call-1"]),
                result(
                    "task-1",
                    "result-1",
                    Some("assistant-1"),
                    "call-1",
                    ToolResultSource::PersistedHistory,
                ),
            ],
            &RepairStateStatus::from_sidecar_json(Some("{not valid json".to_owned())),
            &ReadinessDiagnosticContext::new(
                "test-conversation",
                "test-attempt",
                ReadinessTriggerLayer::SerializerValidation,
            ),
        )
        .expect("valid real results should not need repair authorization");

        assert_eq!(report.state, ReadinessState::Ready);
    }

    fn make_tool_call(call_id: &str) -> ToolCall {
        ToolCall {
            call_id: call_id.to_owned(),
            fn_name: "dummy".to_owned(),
            fn_arguments: serde_json::json!({}),
            thought_signatures: None,
        }
    }

    fn assistant_with_calls(call_ids: &[&str]) -> ChatMessage {
        ChatMessage::from(
            call_ids
                .iter()
                .map(|call_id| make_tool_call(call_id))
                .collect::<Vec<_>>(),
        )
    }

    fn tool_response(call_id: &str) -> ChatMessage {
        ChatMessage::from(ToolResponse::new(call_id.to_owned(), "{}".to_owned()))
    }

    fn strict_chat_completions_ordering_errors(messages: &[ChatMessage]) -> Vec<String> {
        let mut errors = Vec::new();
        let mut pending_call_ids: Vec<String> = Vec::new();
        let mut seen_tool_results: HashSet<String> = HashSet::new();

        for (idx, msg) in messages.iter().enumerate() {
            if !pending_call_ids.is_empty() && msg.role != ChatRole::Tool {
                errors.push(format!(
                    "message {idx} role {:?} appeared before pending tool responses {:?}",
                    msg.role, pending_call_ids
                ));
            }

            if msg.role == ChatRole::Assistant {
                let tool_call_ids: Vec<String> = msg
                    .content
                    .tool_calls()
                    .iter()
                    .map(|tool_call| tool_call.call_id.clone())
                    .collect();
                if !tool_call_ids.is_empty() {
                    pending_call_ids = tool_call_ids;
                }
            } else if msg.role == ChatRole::Tool {
                let responses = msg.content.tool_responses();
                if pending_call_ids.is_empty() {
                    errors.push(format!("message {idx} is an orphan tool response"));
                }
                for response in responses {
                    if !seen_tool_results.insert(response.call_id.clone()) {
                        errors.push(format!("duplicate tool result id {}", response.call_id));
                    }
                    match pending_call_ids.first() {
                        Some(expected_call_id) if expected_call_id == &response.call_id => {
                            pending_call_ids.remove(0);
                        }
                        Some(expected_call_id) => {
                            if let Some(pos) = pending_call_ids
                                .iter()
                                .position(|call_id| call_id == &response.call_id)
                            {
                                errors.push(format!(
                                    "out-of-order tool result id {} expected {}",
                                    response.call_id, expected_call_id
                                ));
                                pending_call_ids.remove(pos);
                            } else {
                                errors.push(format!("orphan tool result id {}", response.call_id));
                            }
                        }
                        None => {
                            errors.push(format!("orphan tool result id {}", response.call_id));
                        }
                    }
                }
            }
        }

        if !pending_call_ids.is_empty() {
            errors.push(format!(
                "request ended with pending tool responses {:?}",
                pending_call_ids
            ));
        }

        errors
    }

    #[test]
    fn strict_request_body_checker_accepts_ordered_tool_responses() {
        let messages = vec![
            ChatMessage::user("hi"),
            assistant_with_calls(&["a", "b"]),
            tool_response("a"),
            tool_response("b"),
            ChatMessage::user("continue"),
        ];

        assert!(strict_chat_completions_ordering_errors(&messages).is_empty());
    }

    #[test]
    fn strict_request_body_checker_rejects_orphans_duplicates_and_early_boundaries() {
        let orphan = vec![ChatMessage::user("hi"), tool_response("a")];
        assert!(strict_chat_completions_ordering_errors(&orphan)
            .iter()
            .any(|error| error.contains("orphan")));

        let duplicate = vec![
            ChatMessage::user("hi"),
            assistant_with_calls(&["a"]),
            tool_response("a"),
            tool_response("a"),
        ];
        assert!(strict_chat_completions_ordering_errors(&duplicate)
            .iter()
            .any(|error| error.contains("duplicate")));

        let early_boundary = vec![
            ChatMessage::user("hi"),
            assistant_with_calls(&["a"]),
            ChatMessage::user("too soon"),
            tool_response("a"),
        ];
        assert!(strict_chat_completions_ordering_errors(&early_boundary)
            .iter()
            .any(|error| error.contains("before pending")));

        let out_of_order = vec![
            ChatMessage::user("hi"),
            assistant_with_calls(&["a", "b"]),
            tool_response("b"),
            tool_response("a"),
        ];
        assert!(strict_chat_completions_ordering_errors(&out_of_order)
            .iter()
            .any(|error| error.contains("out-of-order")));
    }

    #[test]
    fn build_chat_request_body_rejects_missing_duplicate_orphan_and_out_of_order_history() {
        assert_build_request_blocked(
            request_params(
                vec![
                    make_user_query_message("task-1", "req-1", "hi".to_owned(), &[]),
                    make_tool_call_message("task-1", "req-1", "call-1", shell_tool()),
                ],
                vec![],
            ),
            "MissingResultWithoutRepairSource",
        );

        assert_build_request_blocked(
            request_params(
                vec![
                    make_user_query_message("task-1", "req-1", "hi".to_owned(), &[]),
                    make_tool_call_message("task-1", "req-1", "call-1", shell_tool()),
                    make_tool_call_result_message(
                        "task-1",
                        "req-1",
                        "call-1".to_owned(),
                        r#"{"status":"ok"}"#.to_owned(),
                    ),
                    make_tool_call_result_message(
                        "task-1",
                        "req-1",
                        "call-1".to_owned(),
                        r#"{"status":"ok-again"}"#.to_owned(),
                    ),
                ],
                vec![],
            ),
            "DuplicateToolResults",
        );

        assert_build_request_blocked(
            request_params(
                vec![
                    make_user_query_message("task-1", "req-1", "hi".to_owned(), &[]),
                    make_tool_call_result_message(
                        "task-1",
                        "req-1",
                        "call-1".to_owned(),
                        r#"{"status":"orphan"}"#.to_owned(),
                    ),
                ],
                vec![],
            ),
            "OrphanToolResult",
        );

        assert_build_request_blocked(
            request_params(
                vec![
                    make_user_query_message("task-1", "req-1", "hi".to_owned(), &[]),
                    make_tool_call_message("task-1", "req-1", "call-1", shell_tool()),
                    make_user_query_message("task-1", "req-2", "too soon".to_owned(), &[]),
                    make_tool_call_result_message(
                        "task-1",
                        "req-2",
                        "call-1".to_owned(),
                        r#"{"status":"late"}"#.to_owned(),
                    ),
                ],
                vec![],
            ),
            "OutOfOrderToolResult",
        );
    }

    #[test]
    fn build_chat_request_body_accepts_current_input_tool_result() {
        let params = request_params(
            vec![
                make_user_query_message("task-1", "req-1", "hi".to_owned(), &[]),
                make_tool_call_message("task-1", "req-1", "call-1", shell_tool()),
            ],
            vec![
                cancelled_action_result_input("call-1"),
                user_query_input("continue"),
            ],
        );

        let request = build_openai_request(&params).expect("current input result should serialize");
        let errors = strict_chat_completions_ordering_errors(&request.messages);
        assert!(
            errors.is_empty(),
            "request body ordering errors: {errors:?}"
        );
        assert_request_has_no_repair_placeholder(&request);
    }

    #[test]
    fn build_chat_request_body_ignores_filtered_subagent_tool_call_result() {
        let params = request_params(
            vec![
                make_user_query_message("task-1", "req-1", "hi".to_owned(), &[]),
                make_tool_call_message("task-1", "req-1", "subagent-call-1", subagent_tool()),
                make_tool_call_result_message(
                    "task-1",
                    "req-1",
                    "subagent-call-1".to_owned(),
                    r#"{"status":"spawned"}"#.to_owned(),
                ),
            ],
            vec![],
        );

        let request =
            build_openai_request(&params).expect("filtered subagent result should not block");
        let errors = strict_chat_completions_ordering_errors(&request.messages);
        assert!(
            errors.is_empty(),
            "request body ordering errors: {errors:?}"
        );
        assert!(
            request
                .messages
                .iter()
                .flat_map(|message| message.content.tool_responses())
                .all(|response| response.call_id != "subagent-call-1"),
            "filtered subagent ToolCallResult must not be sent outbound"
        );
    }

    #[test]
    fn build_chat_request_body_emits_structured_repair_placeholder_only_for_accepted_repair() {
        let tool_call_message = make_tool_call_message("task-1", "req-1", "call-1", shell_tool());
        let assistant_message_id = tool_call_message.id.clone();
        let repair = RepairRecord::new(
            RepairSource::ForkedHistory,
            ToolCallKey::new("task-1", assistant_message_id, "call-1"),
        );
        let params = request_params_with_repair(
            vec![
                make_user_query_message("task-1", "req-1", "hi".to_owned(), &[]),
                tool_call_message,
            ],
            vec![],
            RepairState::new(vec![repair]),
        );

        let request = build_openai_request(&params).expect("accepted repair should serialize");
        let response = request
            .messages
            .iter()
            .flat_map(|message| message.content.tool_responses())
            .find(|response| response.call_id == "call-1")
            .expect("repair placeholder response should be present");
        let payload: serde_json::Value =
            serde_json::from_str(&response.content).expect("placeholder should be JSON");
        let object = payload
            .as_object()
            .expect("placeholder should be an object");

        assert_eq!(
            object.keys().cloned().collect::<HashSet<_>>(),
            HashSet::from([
                "status".to_string(),
                "reason".to_string(),
                "note".to_string()
            ])
        );
        assert_eq!(payload["status"], "unavailable");
        assert_eq!(payload["reason"], "forked_history_repair");
        assert_eq!(
            payload["note"],
            "tool result was unavailable in repaired conversation history"
        );
        assert!(!response.content.contains("(tool 执行结果未保留)"));
        assert!(
            params.tasks[0].messages.iter().all(|message| !matches!(
                message.message,
                Some(api::message::Message::ToolCallResult(_))
            )),
            "accepted repair placeholders must remain outbound-only"
        );
    }

    #[test]
    fn accepted_repair_log_summary_includes_source_counts_and_redacted_keys() {
        let repairs = vec![
            AcceptedRepair {
                record: RepairRecord::new(
                    RepairSource::ForkedHistory,
                    ToolCallKey::new("task-1", "assistant-1", "call-1"),
                ),
                tool_call: ToolCallRef::new(
                    ToolCallKey::new("task-1", "assistant-1", "call-1"),
                    kind(),
                ),
            },
            AcceptedRepair {
                record: RepairRecord::new(
                    RepairSource::RestoredLegacyHistory,
                    ToolCallKey::new("task-1", "assistant-2", "call-2"),
                ),
                tool_call: ToolCallRef::new(
                    ToolCallKey::new("task-1", "assistant-2", "call-2"),
                    RedactedToolKind::new("local_interception:webfetch"),
                ),
            },
        ];
        let context = ReadinessDiagnosticContext::new(
            "conversation-1",
            "attempt-1",
            ReadinessTriggerLayer::SerializerValidation,
        );

        let message = accepted_history_repair_log_message(&repairs, &context);

        assert!(message.contains("serializer accepted history repair"));
        assert!(message.contains("records=2"));
        assert!(message.contains("category=AcceptedHistoryRepair"));
        assert!(message.contains("forked_history=1"));
        assert!(message.contains("restored_legacy_history=1"));
        assert!(message.contains("conversation_id=conversation-1"));
        assert!(message.contains("trigger_layer=serializer_validation"));
        assert!(message.contains("request_attempt_id=attempt-1"));
        assert!(message.contains("task_id=task-1"));
        assert!(message.contains("assistant_tool_call_message_id=assistant-1"));
        assert!(message.contains("tool_call_id=call-1"));
        assert!(message.contains("redacted_tool_kind=local_interception:webfetch"));
        assert!(!message.contains(REPAIR_PLACEHOLDER_NOTE));
        assert!(!message.contains("secret user prompt"));
        assert!(!message.contains("raw tool arguments"));
        assert!(!message.contains("raw tool output"));
        assert!(!message.contains("raw local interception payload"));
    }

    #[test]
    fn build_chat_request_body_honors_compaction_filtering_boundaries() {
        let hidden_user = make_user_query_message("task-1", "req-1", "hidden".to_owned(), &[]);
        let hidden_call = make_tool_call_message("task-1", "req-1", "hidden-call", shell_tool());
        let summary_user = make_user_query_message("task-1", "req-2", "/compact".to_owned(), &[]);
        let summary_assistant =
            make_agent_output_message("task-1", "req-2", "redacted summary".to_owned());
        let visible_user = make_user_query_message("task-1", "req-3", "visible".to_owned(), &[]);

        let mut compaction_state = CompactionState::default();
        compaction_state.push_completed(CompletedCompaction {
            user_msg_id: summary_user.id.clone(),
            assistant_msg_id: summary_assistant.id.clone(),
            head_message_ids: vec![hidden_user.id.clone(), hidden_call.id.clone()],
            tail_start_id: Some(visible_user.id.clone()),
            summary_text: Some("redacted summary".to_owned()),
            auto: false,
            overflow: false,
        });

        let mut params = request_params(
            vec![
                hidden_user.clone(),
                hidden_call.clone(),
                summary_user.clone(),
                summary_assistant.clone(),
                visible_user.clone(),
            ],
            vec![],
        );
        params.compaction_state = Some(compaction_state.clone());

        let request = build_openai_request(&params)
            .expect("hidden historical tool-call gap should not block");
        let errors = strict_chat_completions_ordering_errors(&request.messages);
        assert!(
            errors.is_empty(),
            "request body ordering errors: {errors:?}"
        );
        assert_request_has_no_repair_placeholder(&request);

        let visible_call = make_tool_call_message("task-1", "req-3", "visible-call", shell_tool());
        let mut params = request_params(
            vec![
                hidden_user,
                hidden_call,
                summary_user,
                summary_assistant,
                visible_user,
                visible_call,
            ],
            vec![],
        );
        params.compaction_state = Some(compaction_state);

        assert_build_request_blocked(params, "MissingResultWithoutRepairSource");
    }
}

/// **accepted history repair outbound 修复行为验证**:
///
/// 本模块只覆盖已经由 `RepairRecord` 明确授权的 outbound 修复。普通 normal flow
/// 缺失结果必须先被 readiness 阻断,不能再走这里补占位。
#[cfg(test)]
mod accepted_history_repair_tests {
    use super::*;
    use crate::ai::byop_readiness::{RepairRecord, ToolCallKey, ToolCallRef};
    use genai::chat::{ChatMessage, ChatRole, ToolCall};

    fn make_tool_call(call_id: &str) -> ToolCall {
        ToolCall {
            call_id: call_id.to_owned(),
            fn_name: "dummy".to_owned(),
            fn_arguments: serde_json::json!({}),
            thought_signatures: None,
        }
    }

    fn assistant_with_calls(call_ids: &[&str]) -> ChatMessage {
        let calls: Vec<ToolCall> = call_ids.iter().map(|cid| make_tool_call(cid)).collect();
        ChatMessage::from(calls)
    }

    fn tool_response(call_id: &str, content: &str) -> ChatMessage {
        ChatMessage::from(ToolResponse::new(call_id.to_owned(), content.to_owned()))
    }

    /// 单条 Tool message 中所有 ToolResponse 的 (call_id, content) 折平,便于断言。
    fn responses_of(msg: &ChatMessage) -> Vec<(String, String)> {
        msg.content
            .tool_responses()
            .iter()
            .map(|r| (r.call_id.clone(), r.content.clone()))
            .collect()
    }

    fn accepted_repair(key: ToolCallKey, source: RepairSource) -> AcceptedRepair {
        AcceptedRepair {
            record: RepairRecord::new(source, key.clone()),
            tool_call: ToolCallRef::new(key, RedactedToolKind::new("shell")),
        }
    }

    fn outbound_groups_for_messages(messages: &[ChatMessage]) -> Vec<OutboundAssistantToolGroup> {
        let mut groups = Vec::new();
        let mut assistant_group_number = 0;
        for (message_index, message) in messages.iter().enumerate() {
            if message.role != ChatRole::Assistant || message.content.tool_calls().is_empty() {
                continue;
            }

            assistant_group_number += 1;
            let assistant_message_id = format!("assistant-{assistant_group_number}");
            groups.push(OutboundAssistantToolGroup {
                message_index,
                tool_call_keys: message
                    .content
                    .tool_calls()
                    .iter()
                    .map(|tool_call| {
                        ToolCallKey::new("task-1", &assistant_message_id, &tool_call.call_id)
                    })
                    .collect(),
            });
        }

        groups
    }

    fn repairs_for_groups(groups: &[OutboundAssistantToolGroup]) -> Vec<AcceptedRepair> {
        groups
            .iter()
            .flat_map(|group| {
                group
                    .tool_call_keys
                    .iter()
                    .cloned()
                    .map(|key| accepted_repair(key, RepairSource::ForkedHistory))
                    .collect::<Vec<_>>()
            })
            .collect()
    }

    fn repair_messages(messages: &mut Vec<ChatMessage>) {
        let groups = outbound_groups_for_messages(messages);
        let repairs = repairs_for_groups(&groups);
        repair_tool_call_pairs_for_accepted_history_gaps(messages, &repairs, &groups)
            .expect("repair helper expects all gaps to be authorized in tests");
    }

    fn assert_structured_repair_payload(content: &str, reason: &str) {
        let payload: serde_json::Value =
            serde_json::from_str(content).expect("repair placeholder should be JSON");
        assert_eq!(payload["status"], "unavailable");
        assert_eq!(payload["reason"], reason);
        assert_eq!(
            payload["note"],
            "tool result was unavailable in repaired conversation history"
        );
    }

    /// 正常 push 路径:[user, asst(a,b), tool_a, tool_b] → 合并为单条 bundled,不补码。
    #[test]
    fn normal_push_path_merges_adjacent_tool_messages_without_placeholder() {
        let mut msgs = vec![
            ChatMessage::user("hi"),
            assistant_with_calls(&["a", "b"]),
            tool_response("a", "resp_a"),
            tool_response("b", "resp_b"),
        ];
        repair_messages(&mut msgs);

        assert_eq!(msgs.len(), 3, "两条相邻 Tool 合并为一条");
        assert_eq!(msgs[0].role, ChatRole::User);
        assert_eq!(msgs[1].role, ChatRole::Assistant);
        assert_eq!(msgs[2].role, ChatRole::Tool);
        assert_eq!(
            responses_of(&msgs[2]),
            vec![
                ("a".to_owned(), "resp_a".to_owned()),
                ("b".to_owned(), "resp_b".to_owned()),
            ],
            "bundled response 顺序必须与 Assistant.tool_calls 一致"
        );
    }

    /// fork 截断场景 A:Assistant 有 tool_calls 但**所有** ToolCallResult 缺失 →
    /// 在 Assistant 后插入一条全 placeholder 的 Tool message。
    #[test]
    fn fork_truncated_missing_all_tool_responses_inserts_placeholders() {
        let mut msgs = vec![ChatMessage::user("q"), assistant_with_calls(&["a", "b"])];
        repair_messages(&mut msgs);

        assert_eq!(msgs.len(), 3, "Assistant 后必须补一条 Tool message");
        assert_eq!(msgs[2].role, ChatRole::Tool);
        let responses = responses_of(&msgs[2]);
        assert_eq!(
            responses.iter().map(|(c, _)| c.clone()).collect::<Vec<_>>(),
            vec!["a".to_owned(), "b".to_owned()]
        );
        for (_, content) in &responses {
            assert_structured_repair_payload(content, "forked_history_repair");
        }
    }

    /// fork 截断场景 B:Assistant 有 (a, b) 但只有 tool_a 被保留 → b 补 placeholder,
    /// 顺序仍按 Assistant.tool_calls 重组为 (real_a, placeholder_b)。
    #[test]
    fn fork_truncated_partial_tool_responses_fills_missing_only() {
        let mut msgs = vec![
            ChatMessage::user("q"),
            assistant_with_calls(&["a", "b"]),
            tool_response("a", "real_a"),
        ];
        repair_messages(&mut msgs);

        assert_eq!(msgs.len(), 3);
        assert_eq!(msgs[2].role, ChatRole::Tool);
        let responses = responses_of(&msgs[2]);
        assert_eq!(responses[0], ("a".to_owned(), "real_a".to_owned()));
        assert_eq!(responses[1].0, "b".to_owned());
        assert_structured_repair_payload(&responses[1].1, "forked_history_repair");
    }

    /// 孤儿 ToolResponse(call_id 不在 Assistant.tool_calls 里)被丢弃,不会污染输出。
    #[test]
    fn orphan_tool_response_with_unknown_call_id_is_dropped() {
        let mut msgs = vec![
            ChatMessage::user("q"),
            assistant_with_calls(&["a"]),
            tool_response("a", "real_a"),
            tool_response("z", "orphan_z"),
        ];
        repair_messages(&mut msgs);

        assert_eq!(msgs.len(), 3, "两条相邻 Tool 合并,孤儿 z 丢弃");
        let responses = responses_of(&msgs[2]);
        assert_eq!(
            responses,
            vec![("a".to_owned(), "real_a".to_owned())],
            "只保留 Assistant 认识的 call_id"
        );
    }

    /// Assistant.tool_calls 顺序为 (a, b) 但现有 Tool 顺序为 (b, a) →
    /// bundled 输出重组为 (real_a, real_b),对齐 Anthropic tool_use 序。
    #[test]
    fn out_of_order_tool_responses_are_reordered_per_assistant_calls() {
        let mut msgs = vec![
            ChatMessage::user("q"),
            assistant_with_calls(&["a", "b"]),
            tool_response("b", "real_b"),
            tool_response("a", "real_a"),
        ];
        repair_messages(&mut msgs);

        assert_eq!(msgs.len(), 3);
        assert_eq!(
            responses_of(&msgs[2]),
            vec![
                ("a".to_owned(), "real_a".to_owned()),
                ("b".to_owned(), "real_b".to_owned()),
            ]
        );
    }

    /// 用户打断/继续输入时,完成的工具结果可能晚于新 UserQuery 落盘。
    /// 出站请求必须把 result 搬回对应 Assistant 后,否则上游会看到孤儿 ToolResponse。
    #[test]
    fn late_tool_response_after_user_query_is_moved_back_to_tool_call() {
        let mut msgs = vec![
            ChatMessage::user("q1"),
            assistant_with_calls(&["a"]),
            ChatMessage::user("interrupt"),
            tool_response("a", "real_a"),
        ];
        repair_messages(&mut msgs);

        assert_eq!(msgs.len(), 4);
        assert_eq!(msgs[0].role, ChatRole::User);
        assert_eq!(msgs[1].role, ChatRole::Assistant);
        assert_eq!(msgs[2].role, ChatRole::Tool);
        assert_eq!(
            responses_of(&msgs[2]),
            vec![("a".to_owned(), "real_a".to_owned())]
        );
        assert_eq!(msgs[3].role, ChatRole::User);
    }

    /// 同一 call_id 连续两条真实 ToolResponse(重复持久化 / 手动重试等场景),
    /// 后到的真实值应覆盖早到的真实值 — 保持与旧实现"后到 insert 覆盖"语义一致。
    #[test]
    fn real_tool_response_is_replaced_by_later_real_tool_response() {
        let mut msgs = vec![
            ChatMessage::user("q1"),
            assistant_with_calls(&["a"]),
            tool_response("a", "real_a_v1"),
            tool_response("a", "real_a_v2"),
        ];
        repair_messages(&mut msgs);

        assert_eq!(msgs.len(), 3);
        assert_eq!(msgs[2].role, ChatRole::Tool);
        assert_eq!(
            responses_of(&msgs[2]),
            vec![("a".to_owned(), "real_a_v2".to_owned())],
            "同 call_id 多条真实 response,后到者胜出"
        );
    }

    /// placeholder 不能覆盖已存在的真实 result。
    /// 该场景出现在:上轮补过 placeholder 作为占位 → 本轮着落真实结果 →
    /// 又遇到同 call_id 的 placeholder(比如 fork 后拼接)。需保证真实不会被陯害。
    #[test]
    fn placeholder_does_not_overwrite_existing_real_response() {
        let mut msgs = vec![
            ChatMessage::user("q1"),
            assistant_with_calls(&["a"]),
            tool_response("a", "real_a"),
            tool_response("a", "(tool 执行结果未保留)"),
        ];
        repair_messages(&mut msgs);

        assert_eq!(msgs.len(), 3);
        assert_eq!(
            responses_of(&msgs[2]),
            vec![("a".to_owned(), "real_a".to_owned())],
            "placeholder 不能覆盖真实值"
        );
    }

    /// 已污染历史可能同时含 placeholder 和晚到的真实 result;真实 result 应覆盖占位。
    #[test]
    fn placeholder_is_replaced_by_late_real_tool_response() {
        let mut msgs = vec![
            ChatMessage::user("q1"),
            assistant_with_calls(&["a"]),
            tool_response("a", "(tool 执行结果未保留)"),
            ChatMessage::user("interrupt"),
            tool_response("a", "real_a"),
        ];
        repair_messages(&mut msgs);

        assert_eq!(msgs.len(), 4);
        assert_eq!(msgs[2].role, ChatRole::Tool);
        assert_eq!(
            responses_of(&msgs[2]),
            vec![("a".to_owned(), "real_a".to_owned())]
        );
        assert_eq!(msgs[3].role, ChatRole::User);
    }

    /// 多个 Assistant tool_calls 段都能独立被处理:每段 Assistant + Tool 不会互相影响。
    #[test]
    fn multiple_assistant_tool_call_groups_handled_independently() {
        let mut msgs = vec![
            ChatMessage::user("q1"),
            assistant_with_calls(&["a", "b"]),
            tool_response("a", "real_a"),
            tool_response("b", "real_b"),
            ChatMessage::user("q2"),
            assistant_with_calls(&["c"]),
            tool_response("c", "real_c"),
        ];
        repair_messages(&mut msgs);

        // 期望结果:user, asst(a,b), bundled(a,b), user, asst(c), bundled(c)
        assert_eq!(msgs.len(), 6);
        assert_eq!(msgs[2].role, ChatRole::Tool);
        assert_eq!(
            responses_of(&msgs[2]),
            vec![
                ("a".to_owned(), "real_a".to_owned()),
                ("b".to_owned(), "real_b".to_owned()),
            ]
        );
        assert_eq!(msgs[5].role, ChatRole::Tool);
        assert_eq!(
            responses_of(&msgs[5]),
            vec![("c".to_owned(), "real_c".to_owned())]
        );
    }

    /// 不含 tool_calls 的 Assistant message 不被动 — 不会付加多余 Tool。
    #[test]
    fn assistant_without_tool_calls_is_untouched() {
        let mut msgs = vec![
            ChatMessage::user("q"),
            ChatMessage::assistant("plain reply"),
        ];
        let before = msgs.len();
        repair_messages(&mut msgs);
        assert_eq!(msgs.len(), before);
        assert_eq!(msgs[1].role, ChatRole::Assistant);
        assert!(msgs[1].content.tool_calls().is_empty());
    }

    #[test]
    fn accepted_repair_placeholder_is_authorized_by_full_tool_call_key() {
        let first_key = ToolCallKey::new("task-1", "assistant-1", "dup");
        let second_key = ToolCallKey::new("task-2", "assistant-2", "dup");
        let mut msgs = vec![
            ChatMessage::user("q1"),
            assistant_with_calls(&["dup"]),
            ChatMessage::user("q2"),
            assistant_with_calls(&["dup"]),
            tool_response("dup", "real_second"),
        ];
        let groups = vec![
            OutboundAssistantToolGroup {
                message_index: 1,
                tool_call_keys: vec![first_key.clone()],
            },
            OutboundAssistantToolGroup {
                message_index: 3,
                tool_call_keys: vec![second_key],
            },
        ];
        let repairs = vec![accepted_repair(first_key, RepairSource::ForkedHistory)];

        repair_tool_call_pairs_for_accepted_history_gaps(&mut msgs, &repairs, &groups)
            .expect("second_key has a real response, so repair must succeed");

        assert_eq!(msgs.len(), 6);
        assert_eq!(msgs[2].role, ChatRole::Tool);
        let first_responses = responses_of(&msgs[2]);
        assert_eq!(first_responses[0].0, "dup");
        assert_structured_repair_payload(&first_responses[0].1, "forked_history_repair");
        assert_eq!(msgs[5].role, ChatRole::Tool);
        assert_eq!(
            responses_of(&msgs[5]),
            vec![("dup".to_owned(), "real_second".to_owned())],
            "重复 call_id 的真实结果必须留在自己的 Assistant group 后"
        );
    }

    #[test]
    fn unavailable_json_with_non_repair_reason_is_not_placeholder() {
        let real_unavailable =
            r#"{"status":"unavailable","reason":"service_down","note":"try later"}"#;
        assert!(!is_placeholder_tool_response_content(real_unavailable));
        assert!(!is_placeholder_tool_response_content(
            r#"{"status":"unavailable","reason":"forked_history_repair","note":"tool result was unavailable in repaired conversation history","extra":true}"#,
        ));
        assert!(is_placeholder_tool_response_content(
            &repair_placeholder_content(RepairSource::ForkedHistory)
        ));

        let mut msgs = vec![
            ChatMessage::user("q1"),
            assistant_with_calls(&["a"]),
            tool_response("a", real_unavailable),
            tool_response(
                "a",
                &repair_placeholder_content(RepairSource::ForkedHistory),
            ),
        ];
        repair_messages(&mut msgs);

        assert_eq!(msgs.len(), 3);
        assert_eq!(
            responses_of(&msgs[2]),
            vec![("a".to_owned(), real_unavailable.to_owned())],
            "真实 unavailable JSON 不能被 repair placeholder 覆盖"
        );
    }
}

// ---------------------------------------------------------------------------
// 测试辅助: 给同一 crate 内的 `cache_stability_tests` 使用。
// ---------------------------------------------------------------------------


/// Issue #94 回归测试:`build_chat_request` 收集多 task 消息时,历史轮 user 消息
/// 不能被乱序排到末尾、也不能因 LRC subagent 副本而重复。
#[cfg(test)]
mod issue_94_task_linearization_tests {
    use super::*;
    use crate::test_util::ai_agent_tasks::{
        create_api_subtask, create_api_task, create_message, create_subagent_tool_call_message,
    };

    /// 构造一条带 `request_id` 的 UserQuery message。
    fn user_query_msg(id: &str, task_id: &str, request_id: &str, query: &str) -> api::Message {
        api::Message {
            id: id.to_string(),
            task_id: task_id.to_string(),
            server_message_data: String::new(),
            citations: vec![],
            message: Some(api::message::Message::UserQuery(api::message::UserQuery {
                query: query.to_string(),
                ..Default::default()
            })),
            request_id: request_id.to_string(),
            timestamp: None,
        }
    }

    /// 复刻 Issue #94 的 01.json / 02.json 场景:用户只发过一句话,但在 LRC CLI
    /// subagent 派生后,这句 UserQuery 同时存在于 root task 与 subtask。
    ///
    /// - root: [UserQuery, AgentOutput, Subagent ToolCall→subtask]
    /// - subtask: [UserQuery(同 request_id+query 的副本), AgentOutput]
    fn issue_94_tasks() -> (api::Task, api::Task) {
        const REQ: &str = "req-1";
        const QUERY: &str = "帮我在这个服务器上搭建一个dns(doh)分流服务";
        let root = create_api_task(
            "root",
            vec![
                user_query_msg("m1", "root", REQ, QUERY),
                create_message("m2", "root"),
                create_subagent_tool_call_message("m3", "root", "sub-1", None),
            ],
        );
        let subtask = create_api_subtask(
            "sub-1",
            "root",
            vec![
                user_query_msg("s1", "sub-1", REQ, QUERY),
                create_message("s2", "sub-1"),
            ],
        );
        (root, subtask)
    }

    fn count_user_queries(msgs: &[&api::Message]) -> usize {
        msgs.iter()
            .filter(|m| matches!(&m.message, Some(api::message::Message::UserQuery(_))))
            .count()
    }

    fn message_ids(msgs: &[&api::Message]) -> Vec<String> {
        msgs.iter().map(|m| m.id.clone()).collect()
    }

    /// 复现:旧实现 `params.tasks.iter().flat_map(|t| t.messages.iter())` 朴素拼接 ——
    /// (1) UserQuery 重复出现两次;(2) 结果随 task 输入顺序变化(`compute_active_tasks`
    /// 用 HashMap::into_values 收集,顺序不确定)。
    #[test]
    fn naive_flat_map_reproduces_issue_94() {
        let (root, subtask) = issue_94_tasks();

        let naive = |tasks: &[api::Task]| -> Vec<String> {
            tasks
                .iter()
                .flat_map(|t| t.messages.iter())
                .map(|m| m.id.clone())
                .collect()
        };

        let root_first = naive(&[root.clone(), subtask.clone()]);
        let subtask_first = naive(&[subtask.clone(), root.clone()]);

        // (1) UserQuery 被拼了两次。
        let root_first_refs: Vec<&api::Message> = [&root, &subtask]
            .iter()
            .flat_map(|t| t.messages.iter())
            .collect();
        assert_eq!(
            count_user_queries(&root_first_refs),
            2,
            "朴素拼接会让同一条 user query 出现两次 —— 这正是 Issue #94 的 bug"
        );

        // (2) 顺序随输入 task 顺序漂移 —— subtask 在前时历史 user(m1)被甩到末尾。
        assert_ne!(
            root_first, subtask_first,
            "朴素拼接结果依赖 task 顺序,非确定性"
        );
        assert_eq!(
            subtask_first.last().map(String::as_str),
            Some("m3"),
            "subtask 排前时 root 的消息整体后移"
        );
        assert!(
            subtask_first.iter().position(|id| id == "s1").unwrap()
                < subtask_first.iter().position(|id| id == "m1").unwrap(),
            "subtask 的 UserQuery 副本(s1)排到了 root 原件(m1)之前"
        );
    }

    /// 修复验证:`collect_linearized_task_messages` 输出与 task 输入顺序无关,
    /// UserQuery 去重后只剩一条,且整体为 root→subtask 的 DFS 线性序。
    #[test]
    fn linearized_collection_is_deterministic_and_deduped() {
        let (root, subtask) = issue_94_tasks();

        let root_first = vec![root.clone(), subtask.clone()];
        let subtask_first = vec![subtask.clone(), root.clone()];

        let a = collect_linearized_task_messages(&root_first);
        let b = collect_linearized_task_messages(&subtask_first);

        // 与输入顺序无关。
        assert_eq!(
            message_ids(&a),
            message_ids(&b),
            "结果必须与 params.tasks 的输入顺序无关"
        );

        // UserQuery 去重:LRC 复制出的 subtask 副本(s1)被丢弃。
        assert_eq!(
            count_user_queries(&a),
            1,
            "重复的 UserQuery 必须被去重为一条"
        );

        // DFS 线性序:root 的消息在前,遇到 Subagent ToolCall 下钻 subtask。
        // s1 被去重,故期望 [m1, m2, m3, s2]。
        assert_eq!(message_ids(&a), vec!["m1", "m2", "m3", "s2"]);

        // 保留下来的那条 user query 是 root 原件,排在序列开头。
        assert_eq!(a.first().map(|m| m.id.as_str()), Some("m1"));
    }

    /// 普通单 task 对话(无 subagent)不受影响:消息原样、原序返回。
    #[test]
    fn single_task_conversation_unchanged() {
        let root = create_api_task(
            "root",
            vec![
                user_query_msg("m1", "root", "req-1", "你好"),
                create_message("m2", "root"),
            ],
        );
        let out = collect_linearized_task_messages(std::slice::from_ref(&root));
        assert_eq!(message_ids(&out), vec!["m1", "m2"]);
    }

    /// 不同用户轮次即使 query 文本相同,只要 `request_id` 不同就不会被误删。
    #[test]
    fn distinct_turns_with_same_text_are_kept() {
        let root = create_api_task(
            "root",
            vec![
                user_query_msg("m1", "root", "req-1", "继续"),
                create_message("m2", "root"),
                user_query_msg("m3", "root", "req-2", "继续"),
            ],
        );
        let out = collect_linearized_task_messages(std::slice::from_ref(&root));
        assert_eq!(
            count_user_queries(&out),
            2,
            "request_id 不同的两轮 user 消息都要保留"
        );
        assert_eq!(message_ids(&out), vec!["m1", "m2", "m3"]);
    }
}
