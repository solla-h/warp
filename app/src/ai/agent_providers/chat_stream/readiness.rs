//! Pre-flight readiness validation and projection building for BYOP chat requests.
//!
//! This module implements the readiness check logic that ensures the conversation
//! message history is structurally valid before serialization. It builds projections
//! from `RequestParams` and validates them against the `byop_readiness` classifier.
//!
//! Key responsibilities:
//! - `SerializerProjectionBuilder`: accumulates `ProjectionItem`s from message history
//! - `classify_byop_controller_readiness*`: controller-layer readiness (pre-serialization)
//! - `validate_byop_serializer_readiness`: serializer-layer validation (during request build)
//! - Helper functions for tool call kind classification and result kind determination

use std::collections::{HashMap, HashSet};

use serde_json::Value;
use warp_multi_agent_api as api;

use crate::ai::agent::api::RequestParams;
use crate::ai::agent::{AIAgentActionResult, AIAgentInput};
use ai::agent::convert::ConvertToAPITypeError;
use crate::ai::byop_readiness::{
    classify_projection, AcceptedRepair, BlockedByopReadinessError, LiveToolCall,
    LiveToolCallState, ProjectedToolCall, ProjectedToolResult, ProjectionItem, ReadinessCategory,
    ReadinessContext, ReadinessDiagnosticCoalescer, ReadinessDiagnosticContext,
    ReadinessDiagnosticLevel, ReadinessReport, ReadinessState, ReadinessTriggerLayer,
    RedactedToolKind, RepairSource, RepairStateStatus, TerminalResultKind, ToolCallKey,
    ToolCallRef, ToolResultSource,
};

// ---------------------------------------------------------------------------
// SerializerProjectionBuilder
// ---------------------------------------------------------------------------

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

    pub(crate) fn should_skip_tool_result(&self, task_id: &str, tool_call_id: &str) -> bool {
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

// ---------------------------------------------------------------------------
// Helper classifiers
// ---------------------------------------------------------------------------

pub(crate) fn redacted_tool_kind_for_tool_call(tool_call: &api::message::ToolCall) -> RedactedToolKind {
    use crate::ai::agent::task::helper::ToolExt;

    tool_call
        .tool
        .as_ref()
        .map(|tool| RedactedToolKind::new(tool.name()))
        .unwrap_or_default()
}

pub(crate) fn current_input_result_kind(result: &AIAgentActionResult) -> TerminalResultKind {
    if result.result.is_cancelled() {
        TerminalResultKind::Cancellation
    } else {
        TerminalResultKind::Real
    }
}

pub(crate) fn persisted_tool_result_kind(
    msg: &api::Message,
    compacted_tool_msg_ids: Option<&HashSet<String>>,
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

pub(crate) fn current_input_task_id(params: &RequestParams) -> String {
    params
        .byop_target_task_id
        .clone()
        .or_else(|| params.tasks.first().map(|task| task.id.clone()))
        .unwrap_or_else(|| "current_input".to_owned())
}

// ---------------------------------------------------------------------------
// Serializer projection builder
// ---------------------------------------------------------------------------

pub(crate) fn build_serializer_readiness_projection(
    params: &RequestParams,
    all_msgs: &[&api::Message],
    summarize_head_end: Option<usize>,
    summary_inserts: &HashMap<String, String>,
    hidden_msg_ids: &HashSet<String>,
    compacted_tool_msg_ids: &HashSet<String>,
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

// ---------------------------------------------------------------------------
// Controller readiness
// ---------------------------------------------------------------------------

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

// ---------------------------------------------------------------------------
// Serializer readiness validation
// ---------------------------------------------------------------------------

pub(crate) fn validate_byop_serializer_readiness(
    params: &RequestParams,
    all_msgs: &[&api::Message],
    summarize_head_end: Option<usize>,
    summary_inserts: &HashMap<String, String>,
    hidden_msg_ids: &HashSet<String>,
    compacted_tool_msg_ids: &HashSet<String>,
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

pub(crate) fn validate_serializer_readiness_projection(
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

pub(crate) fn validate_serializer_readiness_projection_with_repair_state(
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

// ---------------------------------------------------------------------------
// Diagnostics helpers (used internally by validation)
// ---------------------------------------------------------------------------

fn log_accepted_history_repair(
    repairs: &[AcceptedRepair],
    diagnostic_context: &ReadinessDiagnosticContext<'_>,
) {
    log::info!(
        "{}",
        accepted_history_repair_log_message(repairs, diagnostic_context)
    );
}

pub(crate) fn accepted_history_repair_log_message(
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
