//! Tests for the extracted `readiness` module.

use crate::ai::agent_providers::chat_stream::readiness::{
    accepted_history_repair_log_message, classify_byop_controller_readiness,
    current_input_result_kind, current_input_task_id, persisted_tool_result_kind,
    redacted_tool_kind_for_tool_call, validate_serializer_readiness_projection,
};
use crate::ai::byop_readiness::{
    ProjectedToolCall, ProjectedToolResult, ProjectionItem, ReadinessCategory,
    ReadinessDiagnosticContext, ReadinessState, ReadinessTriggerLayer, RedactedToolKind,
    TerminalResultKind, ToolResultSource,
};
use warp_multi_agent_api as api;

#[test]
fn current_input_task_id_uses_byop_target_if_present() {
    use crate::ai::agent::api::RequestParams;

    let mut params = RequestParams::new_for_test(vec![], vec![]);
    params.byop_target_task_id = Some("explicit-task".to_owned());
    assert_eq!(current_input_task_id(&params), "explicit-task");
}

#[test]
fn current_input_task_id_falls_back_to_first_task() {
    use crate::ai::agent::api::RequestParams;

    let task = api::Task {
        id: "first-task".to_owned(),
        messages: vec![],
        dependencies: None,
        description: String::new(),
        summary: String::new(),
        server_data: String::new(),
    };
    let params = RequestParams::new_for_test(vec![], vec![task]);
    assert_eq!(current_input_task_id(&params), "first-task");
}

#[test]
fn current_input_task_id_default_when_empty() {
    use crate::ai::agent::api::RequestParams;

    let params = RequestParams::new_for_test(vec![], vec![]);
    assert_eq!(current_input_task_id(&params), "current_input");
}

#[test]
fn persisted_tool_result_kind_real_for_non_tool_call_result() {
    let msg = api::Message {
        id: "msg-1".to_owned(),
        task_id: "task-1".to_owned(),
        message: Some(api::message::Message::AgentOutput(
            api::message::AgentOutput {
                text: "hello".to_owned(),
            },
        )),
        server_message_data: String::new(),
        ..Default::default()
    };
    assert_eq!(persisted_tool_result_kind(&msg, None), TerminalResultKind::Real);
}

#[test]
fn persisted_tool_result_kind_local_interception() {
    let msg = api::Message {
        id: "msg-1".to_owned(),
        task_id: "task-1".to_owned(),
        message: Some(api::message::Message::ToolCallResult(
            api::message::ToolCallResult {
                tool_call_id: "call-1".to_owned(),
                result: None,
                ..Default::default()
            },
        )),
        server_message_data: r#"{"_byop_intercepted":true,"status":"ok"}"#.to_owned(),
        ..Default::default()
    };
    assert_eq!(
        persisted_tool_result_kind(&msg, None),
        TerminalResultKind::LocalInterception
    );
}

#[test]
fn persisted_tool_result_kind_structured_error() {
    let msg = api::Message {
        id: "msg-1".to_owned(),
        task_id: "task-1".to_owned(),
        message: Some(api::message::Message::ToolCallResult(
            api::message::ToolCallResult {
                tool_call_id: "call-1".to_owned(),
                result: None,
                ..Default::default()
            },
        )),
        server_message_data: r#"{"error":"invalid_arguments","tool":"shell"}"#.to_owned(),
        ..Default::default()
    };
    assert_eq!(
        persisted_tool_result_kind(&msg, None),
        TerminalResultKind::StructuredError
    );
}

#[test]
fn validate_projection_ready_for_complete_tool_cycle() {
    fn kind() -> RedactedToolKind {
        RedactedToolKind::new("shell")
    }

    let projection = vec![
        ProjectionItem::assistant_tool_calls(
            "task-1",
            "assistant-1",
            vec![ProjectedToolCall::new("task-1", "assistant-1", "call-1", kind())],
        ),
        ProjectionItem::tool_result(ProjectedToolResult::new(
            "task-1",
            "result-1",
            Some("assistant-1".to_owned()),
            "call-1",
            kind(),
            ToolResultSource::PersistedHistory,
            TerminalResultKind::Real,
        )),
    ];

    let report = validate_serializer_readiness_projection(projection)
        .expect("complete tool cycle should be ready");
    assert_eq!(report.state, ReadinessState::Ready);
}

#[test]
fn validate_projection_blocks_missing_result() {
    fn kind() -> RedactedToolKind {
        RedactedToolKind::new("shell")
    }

    let projection = vec![ProjectionItem::assistant_tool_calls(
        "task-1",
        "assistant-1",
        vec![ProjectedToolCall::new("task-1", "assistant-1", "call-1", kind())],
    )];

    let err = validate_serializer_readiness_projection(projection)
        .expect_err("missing result should block");
    assert!(err.to_string().contains("Can't continue"));
}

#[test]
fn accepted_history_repair_log_message_format() {
    use crate::ai::byop_readiness::{AcceptedRepair, RepairRecord, RepairSource, ToolCallKey, ToolCallRef};

    let repair = AcceptedRepair {
        record: RepairRecord::new(
            RepairSource::ForkedHistory,
            ToolCallKey::new("task-1", "assistant-1", "call-1"),
        ),
        tool_call: ToolCallRef::new(
            ToolCallKey::new("task-1", "assistant-1", "call-1"),
            RedactedToolKind::new("shell"),
        ),
    };
    let context = ReadinessDiagnosticContext::new(
        "conv-123",
        "attempt-456",
        ReadinessTriggerLayer::SerializerValidation,
    );

    let message = accepted_history_repair_log_message(&[repair], &context);
    assert!(message.contains("conv-123"));
    assert!(message.contains("attempt-456"));
    assert!(message.contains("forked_history=1"));
    assert!(message.contains("call-1"));
}
