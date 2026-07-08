//! Unit tests for the `events` submodule of `chat_stream`.

#[cfg(test)]
mod tests {
    use std::collections::HashMap;
    use warp_multi_agent_api as api;

    use crate::ai::agent_providers::chat_stream::events::{
        create_subtask_event, create_task_event, extract_search_pages_from_exa_results,
        make_agent_output_message, make_finished_done, make_reasoning_message,
        make_tool_call_message,
    };

    // -----------------------------------------------------------------------
    // make_agent_output_message
    // -----------------------------------------------------------------------

    #[test]
    fn test_make_agent_output_message_has_correct_type() {
        let msg = make_agent_output_message("task-1", "req-1", "hello".to_string());
        assert_eq!(msg.task_id, "task-1");
        assert_eq!(msg.request_id, "req-1");
        match msg.message {
            Some(api::message::Message::AgentOutput(ref out)) => {
                assert_eq!(out.text, "hello");
            }
            other => panic!("expected AgentOutput, got {other:?}"),
        }
    }

    // -----------------------------------------------------------------------
    // make_reasoning_message
    // -----------------------------------------------------------------------

    #[test]
    fn test_make_reasoning_message_has_correct_type() {
        let msg = make_reasoning_message("task-2", "req-2", "thinking...".to_string());
        assert_eq!(msg.task_id, "task-2");
        assert_eq!(msg.request_id, "req-2");
        match msg.message {
            Some(api::message::Message::AgentReasoning(ref r)) => {
                assert_eq!(r.reasoning, "thinking...");
                assert!(r.finished_duration.is_none());
            }
            other => panic!("expected AgentReasoning, got {other:?}"),
        }
    }

    // -----------------------------------------------------------------------
    // make_tool_call_message
    // -----------------------------------------------------------------------

    #[test]
    fn test_make_tool_call_message_structure() {
        // Use a simple tool variant to verify structure
        let tool = api::message::tool_call::Tool::Grep(api::message::tool_call::Grep {
            queries: vec!["search".to_owned()],
            path: ".".to_owned(),
        });
        let msg = make_tool_call_message("task-3", "req-3", "call-42", tool);
        assert_eq!(msg.task_id, "task-3");
        assert_eq!(msg.request_id, "req-3");
        match msg.message {
            Some(api::message::Message::ToolCall(ref tc)) => {
                assert_eq!(tc.tool_call_id, "call-42");
                assert!(tc.tool.is_some());
            }
            other => panic!("expected ToolCall, got {other:?}"),
        }
    }

    // -----------------------------------------------------------------------
    // make_finished_done
    // -----------------------------------------------------------------------

    #[test]
    fn test_make_finished_done_includes_usage() {
        let usage =
            api::response_event::stream_finished::ConversationUsageMetadata {
                context_window_usage: 42.0,
                summarized: false,
                credits_spent: 1.5,
                #[allow(deprecated)]
                token_usage: Vec::new(),
                tool_usage_metadata: None,
                warp_token_usage: std::collections::HashMap::new(),
                byok_token_usage: std::collections::HashMap::new(),
                custom_endpoint_token_usage: std::collections::HashMap::new(),
                platform_credits_spent: 0.0,
            };
        let event = make_finished_done(Some(usage));
        match event.r#type {
            Some(api::response_event::Type::Finished(ref fin)) => {
                assert!(fin.conversation_usage_metadata.is_some());
                let meta = fin.conversation_usage_metadata.as_ref().unwrap();
                assert_eq!(meta.context_window_usage, 42.0);
                assert_eq!(meta.credits_spent, 1.5);
            }
            other => panic!("expected Finished, got {other:?}"),
        }
    }

    // -----------------------------------------------------------------------
    // create_task_event
    // -----------------------------------------------------------------------

    #[test]
    fn test_create_task_event_has_task_id() {
        let event = create_task_event("my-task-123");
        match event.r#type {
            Some(api::response_event::Type::ClientActions(ref ca)) => {
                assert_eq!(ca.actions.len(), 1);
                match &ca.actions[0].action {
                    Some(api::client_action::Action::CreateTask(ct)) => {
                        let task = ct.task.as_ref().unwrap();
                        assert_eq!(task.id, "my-task-123");
                        assert!(task.dependencies.is_none());
                    }
                    other => panic!("expected CreateTask, got {other:?}"),
                }
            }
            other => panic!("expected ClientActions, got {other:?}"),
        }
    }

    #[test]
    fn test_create_subtask_event_has_parent() {
        let event = create_subtask_event("sub-1", "parent-1");
        match event.r#type {
            Some(api::response_event::Type::ClientActions(ref ca)) => {
                match &ca.actions[0].action {
                    Some(api::client_action::Action::CreateTask(ct)) => {
                        let task = ct.task.as_ref().unwrap();
                        assert_eq!(task.id, "sub-1");
                        let deps = task.dependencies.as_ref().unwrap();
                        assert_eq!(deps.parent_task_id, "parent-1");
                    }
                    other => panic!("expected CreateTask, got {other:?}"),
                }
            }
            other => panic!("expected ClientActions, got {other:?}"),
        }
    }

    // -----------------------------------------------------------------------
    // extract_search_pages_from_exa_results
    // -----------------------------------------------------------------------

    #[test]
    fn test_extract_search_pages_parses_urls() {
        let input = r#"Title: Rust Blog Post
URL: https://blog.rust-lang.org/2026/04/16/Rust-1.95.0/
Published: 2026-04-16T00:00:00.000Z
Author: N/A
---
Title: Another Page
URL: https://example.com/page
---
"#;
        let pages = extract_search_pages_from_exa_results(input);
        assert_eq!(pages.len(), 2);
        assert_eq!(pages[0].0, "https://blog.rust-lang.org/2026/04/16/Rust-1.95.0/");
        assert_eq!(pages[0].1, "Rust Blog Post");
        assert_eq!(pages[1].0, "https://example.com/page");
        assert_eq!(pages[1].1, "Another Page");
    }

    #[test]
    fn test_extract_search_pages_markdown_links() {
        let input = "[Example](https://example.com/a) and [Other](https://other.org/b)";
        let pages = extract_search_pages_from_exa_results(input);
        assert_eq!(pages.len(), 2);
        assert_eq!(pages[0].0, "https://example.com/a");
        assert_eq!(pages[0].1, "Example");
        assert_eq!(pages[1].0, "https://other.org/b");
        assert_eq!(pages[1].1, "Other");
    }

    #[test]
    fn test_extract_search_pages_deduplication() {
        let input = r#"Title: Page
URL: https://example.com/dup
---
Title: Page Again
URL: https://example.com/dup
"#;
        let pages = extract_search_pages_from_exa_results(input);
        assert_eq!(pages.len(), 1);
    }
}
