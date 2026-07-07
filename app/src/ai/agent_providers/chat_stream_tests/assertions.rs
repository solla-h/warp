//! Custom assertion helpers for chat_stream tests.

use warp_multi_agent_api as api;

use super::stream_collector::CollectedStream;

/// Assert the stream completed successfully with Done status.
pub fn assert_stream_done(collected: &CollectedStream) {
    assert!(
        collected.finished_done,
        "Expected StreamFinished(Done), got: finished_done={}, finished_error={}",
        collected.finished_done, collected.finished_error
    );
}

/// Assert the stream finished with an error.
pub fn assert_stream_error(collected: &CollectedStream) {
    assert!(
        collected.finished_error,
        "Expected StreamFinished(Error), but stream finished with Done or is incomplete"
    );
}

/// Assert that text content was produced.
pub fn assert_has_text_content(collected: &CollectedStream) {
    assert!(
        !collected.text_content.is_empty(),
        "Expected non-empty text content in stream output"
    );
}

/// Assert the stream produced at least N tool calls.
pub fn assert_tool_call_count(collected: &CollectedStream, expected: usize) {
    assert_eq!(
        collected.tool_calls.len(),
        expected,
        "Expected {} tool calls, got {}",
        expected,
        collected.tool_calls.len()
    );
}

/// Assert a CreateTask event was emitted (first turn behavior).
pub fn assert_has_create_task(collected: &CollectedStream) {
    assert!(
        collected.has_create_task(),
        "Expected CreateTask event in stream (first turn)"
    );
}

/// Assert no CreateTask event (subsequent turn behavior).
pub fn assert_no_create_task(collected: &CollectedStream) {
    assert!(
        !collected.has_create_task(),
        "Expected NO CreateTask event (subsequent turn)"
    );
}

/// Assert that streaming produced events in the expected order pattern.
///
/// Pattern elements:
/// - "init" - StreamInit
/// - "create_task" - CreateTask
/// - "text" - AddMessages with AgentOutput
/// - "reasoning" - AddMessages with AgentReasoning
/// - "tool_call" - AddMessages with ToolCall
/// - "done" - StreamFinished(Done)
/// - "error" - StreamFinished(non-Done)
///
/// Use "*" as wildcard to match any number of events.
pub fn assert_event_order(collected: &CollectedStream, pattern: &[&str]) {
    let event_tags: Vec<&str> = collected
        .events
        .iter()
        .filter_map(|e| event_tag(e))
        .collect();

    // Simple ordered subsequence check (pattern must appear in order, ignoring gaps)
    let mut pattern_idx = 0;
    for tag in &event_tags {
        if pattern_idx >= pattern.len() {
            break;
        }
        if pattern[pattern_idx] == "*" {
            pattern_idx += 1;
            if pattern_idx >= pattern.len() {
                break;
            }
        }
        if *tag == pattern[pattern_idx] {
            pattern_idx += 1;
        }
    }

    assert_eq!(
        pattern_idx,
        pattern.len(),
        "Event order mismatch.\nExpected pattern: {:?}\nActual events: {:?}",
        pattern,
        event_tags
    );
}

/// Assert that the `_byop_intercepted` flag is present and true in a tool result.
pub fn assert_byop_intercepted(json_str: &str) {
    let v: serde_json::Value = serde_json::from_str(json_str)
        .unwrap_or_else(|e| panic!("Not valid JSON: {e}\nContent: {json_str}"));
    assert_eq!(
        v.get("_byop_intercepted"),
        Some(&serde_json::Value::Bool(true)),
        "Expected _byop_intercepted:true in tool result JSON"
    );
}

/// Classify a ResponseEvent into a tag for ordering assertions.
fn event_tag(event: &api::ResponseEvent) -> Option<&'static str> {
    match &event.r#type {
        Some(api::response_event::Type::Init(_)) => Some("init"),
        Some(api::response_event::Type::Finished(f)) => match &f.reason {
            Some(api::response_event::stream_finished::Reason::Done(_)) => Some("done"),
            Some(_) => Some("error"),
            None => Some("error"),
        },
        Some(api::response_event::Type::ClientActions(ca)) => {
            for client_action in &ca.actions {
                match &client_action.action {
                    Some(api::client_action::Action::CreateTask(_)) => return Some("create_task"),
                    Some(api::client_action::Action::AddMessagesToTask(add)) => {
                        if add.messages.iter().any(|m| {
                            matches!(&m.message, Some(api::message::Message::AgentOutput(_)))
                        }) {
                            return Some("text");
                        }
                        if add.messages.iter().any(|m| {
                            matches!(
                                &m.message,
                                Some(api::message::Message::AgentReasoning(_))
                            )
                        }) {
                            return Some("reasoning");
                        }
                        if add.messages.iter().any(|m| {
                            matches!(&m.message, Some(api::message::Message::ToolCall(_)))
                        }) {
                            return Some("tool_call");
                        }
                    }
                    _ => {}
                }
            }
            None
        }
        None => None,
    }
}
