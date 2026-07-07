//! Unit tests for the `serialization` submodule of `chat_stream`.

#[cfg(test)]
mod tests {
    use genai::chat::{ChatRole, ContentPart, ToolCall};
    use serde_json::json;
    use warp_multi_agent_api as api;

    use crate::ai::agent_providers::chat_stream::serialization::{
        collect_linearized_task_messages, flush_assistant_buffer, serialize_outgoing_tool_call,
        AssistantBuffer, OutboundAssistantToolGroup, REASONING_ECHO_PLACEHOLDER,
    };
    use crate::ai::byop_readiness::ToolCallKey;
    use crate::test_util::ai_agent_tasks::{
        create_api_subtask, create_api_task, create_message, create_subagent_tool_call_message,
    };

    fn reasoning_part(msg: &genai::chat::ChatMessage) -> Option<&str> {
        for p in msg.content.parts() {
            if let ContentPart::ReasoningContent(r) = p {
                return Some(r.as_str());
            }
        }
        None
    }

    fn text_content(msg: &genai::chat::ChatMessage) -> Option<&str> {
        for p in msg.content.parts() {
            if let ContentPart::Text(t) = p {
                return Some(t.as_str());
            }
        }
        None
    }

    // -----------------------------------------------------------------------
    // AssistantBuffer tests
    // -----------------------------------------------------------------------

    #[test]
    fn test_assistant_buffer_flush_text_only() {
        let mut buf = AssistantBuffer::new(false);
        buf.text = Some("Hello world".to_string());
        let mut msgs = Vec::new();
        buf.flush_into(&mut msgs);
        assert_eq!(msgs.len(), 1);
        assert_eq!(msgs[0].role, ChatRole::Assistant);
        assert_eq!(text_content(&msgs[0]), Some("Hello world"));
    }

    #[test]
    fn test_assistant_buffer_flush_with_tool_call() {
        let mut buf = AssistantBuffer::new(false);
        buf.text = Some("calling tool".to_string());
        let tc = ToolCall {
            call_id: "call_1".to_string(),
            fn_name: "read_files".to_string(),
            fn_arguments: json!({"files": []}),
            thought_signatures: None,
        };
        let key = ToolCallKey {
            task_id: "t1".to_string(),
            assistant_tool_call_message_id: "m1".to_string(),
            tool_call_id: "call_1".to_string(),
        };
        buf.push_tool_call(tc, key);
        let mut msgs = Vec::new();
        buf.flush_into(&mut msgs);
        // text + tool_calls = 2 messages
        assert_eq!(msgs.len(), 2);
        assert_eq!(msgs[0].role, ChatRole::Assistant);
        assert_eq!(text_content(&msgs[0]), Some("calling tool"));
        assert_eq!(msgs[1].role, ChatRole::Assistant);
        assert!(!msgs[1].content.tool_calls().is_empty());
    }

    #[test]
    fn test_assistant_buffer_reasoning_gate_on() {
        let mut buf = AssistantBuffer::new(true);
        buf.text = Some("response".to_string());
        buf.reasoning = Some("deep thinking".to_string());
        let mut msgs = Vec::new();
        buf.flush_into(&mut msgs);
        assert_eq!(msgs.len(), 1);
        assert_eq!(reasoning_part(&msgs[0]), Some("deep thinking"));
    }

    #[test]
    fn test_assistant_buffer_reasoning_gate_off() {
        let mut buf = AssistantBuffer::new(false);
        buf.text = Some("response".to_string());
        buf.reasoning = Some("internal thought".to_string());
        let mut msgs = Vec::new();
        buf.flush_into(&mut msgs);
        assert_eq!(msgs.len(), 1);
        assert!(
            reasoning_part(&msgs[0]).is_none(),
            "reasoning must be dropped when gate is off"
        );
    }

    #[test]
    fn test_assistant_buffer_placeholder_injection() {
        let mut buf = AssistantBuffer::new(true);
        buf.text = Some("ok".to_string());
        buf.reasoning = None; // no reasoning received
        let mut msgs = Vec::new();
        buf.flush_into(&mut msgs);
        assert_eq!(msgs.len(), 1);
        assert_eq!(
            reasoning_part(&msgs[0]),
            Some(REASONING_ECHO_PLACEHOLDER),
            "placeholder must be injected when force_echo=true but no reasoning"
        );
    }

    // -----------------------------------------------------------------------
    // serialize_outgoing_tool_call
    // -----------------------------------------------------------------------

    #[test]
    fn test_serialize_outgoing_tool_call_roundtrip() {
        let tc = api::message::ToolCall {
            tool_call_id: "tc_1".to_string(),
            tool: Some(api::message::tool_call::Tool::RunShellCommand(
                api::message::tool_call::RunShellCommand {
                    command: "ls -la".to_string(),
                    is_read_only: true,
                    uses_pager: false,
                    is_risky: false,
                    citations: vec![],
                    risk_category: 0,
                    wait_until_complete_value: None,
                },
            )),
        };
        let (name, args) = serialize_outgoing_tool_call(&tc, None, "");
        assert_eq!(name, "run_shell_command");
        assert_eq!(args["command"], "ls -la");
        assert_eq!(args["is_read_only"], true);
    }

    // -----------------------------------------------------------------------
    // collect_linearized_task_messages
    // -----------------------------------------------------------------------

    #[test]
    fn test_collect_linearized_task_messages_order() {
        // root task with 2 messages, one of which spawns a subtask
        let root = create_api_task(
            "root",
            vec![
                create_message("m1", "root"),
                create_subagent_tool_call_message("m2", "root", "child", None),
                create_message("m3", "root"),
            ],
        );
        let child = create_api_subtask(
            "child",
            "root",
            vec![
                create_message("c1", "child"),
                create_message("c2", "child"),
            ],
        );
        let tasks = vec![root, child];
        let msgs = collect_linearized_task_messages(&tasks);
        let ids: Vec<&str> = msgs.iter().map(|m| m.id.as_str()).collect();
        // DFS order: root m1, root m2 (subagent call), then child c1, c2, then root m3
        assert_eq!(ids, vec!["m1", "m2", "c1", "c2", "m3"]);
    }

    // -----------------------------------------------------------------------
    // build_user_message_with_binaries
    // -----------------------------------------------------------------------

    #[test]
    fn test_build_user_message_with_binaries_text_only() {
        use crate::ai::agent_providers::attachment_caps::AttachmentCaps;
        use crate::ai::agent_providers::chat_stream::serialization::build_user_message_with_binaries;

        let msg = build_user_message_with_binaries(
            "Hello".to_string(),
            vec![],
            AttachmentCaps::default(),
        );
        assert_eq!(msg.role, ChatRole::User);
        assert_eq!(text_content(&msg), Some("Hello"));
    }

    // -----------------------------------------------------------------------
    // OutboundAssistantToolGroup via flush_into_with_group
    // -----------------------------------------------------------------------

    #[test]
    fn test_outbound_tool_group_tracks_indices() {
        let mut buf = AssistantBuffer::new(false);
        buf.text = Some("prefix".to_string());
        let tc = ToolCall {
            call_id: "call_x".to_string(),
            fn_name: "grep".to_string(),
            fn_arguments: json!({}),
            thought_signatures: None,
        };
        let key = ToolCallKey {
            task_id: "t1".to_string(),
            assistant_tool_call_message_id: "m1".to_string(),
            tool_call_id: "call_x".to_string(),
        };
        buf.push_tool_call(tc, key.clone());
        let mut msgs = Vec::new();
        let group = buf.flush_into_with_group(&mut msgs);
        let group = group.expect("group must be Some when tool_calls present");
        // message_index should point at the tool_calls message (index 1, after text msg at 0)
        assert_eq!(group.message_index, 1);
        assert_eq!(group.tool_call_keys.len(), 1);
        assert_eq!(group.tool_call_keys[0].tool_call_id, "call_x");
    }
}
