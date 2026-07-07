//! Utilities for collecting and inspecting ResponseEvent streams in tests.

use std::sync::Arc;

use futures::StreamExt;
use warp_multi_agent_api as api;

use crate::ai::agent::api::ResponseStream;
use crate::ai::api_error::AIApiError;

/// Collected output from consuming a full ResponseStream.
#[derive(Debug, Default)]
pub struct CollectedStream {
    /// All events in emission order.
    pub events: Vec<api::ResponseEvent>,
    /// Text content accumulated from AppendToMessageContent(AgentOutput) events.
    pub text_content: String,
    /// Reasoning content accumulated from AppendToMessageContent(AgentReasoning) events.
    pub reasoning_content: String,
    /// Tool calls emitted (AddMessages with ToolCall type).
    pub tool_calls: Vec<api::Message>,
    /// Whether StreamFinished(Done) was the terminal event.
    pub finished_done: bool,
    /// Whether StreamFinished with a non-Done reason was the terminal event.
    pub finished_error: bool,
    /// Usage metadata if available (context_window_usage from ConversationUsageMetadata).
    pub usage: Option<api::response_event::stream_finished::ConversationUsageMetadata>,
    /// Per-model token usage from StreamFinished.
    pub token_usage: Vec<api::response_event::stream_finished::TokenUsage>,
}

impl CollectedStream {
    /// Consume the entire stream and collect all events.
    pub async fn collect_from(mut stream: ResponseStream) -> Self {
        let mut collected = Self::default();

        while let Some(item) = stream.next().await {
            match item {
                Ok(event) => collected.process_event(event),
                Err(_err) => {
                    collected.finished_error = true;
                }
            }
        }

        collected
    }

    /// How many events total.
    pub fn event_count(&self) -> usize {
        self.events.len()
    }

    /// Whether a CreateTask action was emitted.
    pub fn has_create_task(&self) -> bool {
        self.events.iter().any(|e| Self::is_create_task(e))
    }

    /// Get all AgentOutput messages.
    pub fn text_messages(&self) -> Vec<&api::Message> {
        self.events
            .iter()
            .filter_map(|e| Self::extract_add_messages(e))
            .flatten()
            .filter(|m| matches!(&m.message, Some(api::message::Message::AgentOutput(_))))
            .collect()
    }

    fn is_create_task(event: &api::ResponseEvent) -> bool {
        Self::for_each_action(event, |action| {
            matches!(action, api::client_action::Action::CreateTask(_))
        })
    }

    fn extract_add_messages(event: &api::ResponseEvent) -> Option<&Vec<api::Message>> {
        if let Some(api::response_event::Type::ClientActions(ref ca)) = event.r#type {
            for client_action in &ca.actions {
                if let Some(api::client_action::Action::AddMessagesToTask(ref add)) =
                    client_action.action
                {
                    return Some(&add.messages);
                }
            }
        }
        None
    }

    fn for_each_action(event: &api::ResponseEvent, mut f: impl FnMut(&api::client_action::Action) -> bool) -> bool {
        if let Some(api::response_event::Type::ClientActions(ref ca)) = event.r#type {
            for client_action in &ca.actions {
                if let Some(ref action) = client_action.action {
                    if f(action) {
                        return true;
                    }
                }
            }
        }
        false
    }

    fn process_event(&mut self, event: api::ResponseEvent) {
        match &event.r#type {
            Some(api::response_event::Type::ClientActions(ca)) => {
                for client_action in &ca.actions {
                    match &client_action.action {
                        Some(api::client_action::Action::AppendToMessageContent(append)) => {
                            if let Some(ref msg) = append.message {
                                match &msg.message {
                                    Some(api::message::Message::AgentOutput(out)) => {
                                        self.text_content.push_str(&out.text);
                                    }
                                    Some(api::message::Message::AgentReasoning(r)) => {
                                        self.reasoning_content.push_str(&r.reasoning);
                                    }
                                    _ => {}
                                }
                            }
                        }
                        Some(api::client_action::Action::AddMessagesToTask(add)) => {
                            for msg in &add.messages {
                                if matches!(&msg.message, Some(api::message::Message::ToolCall(_)))
                                {
                                    self.tool_calls.push(msg.clone());
                                }
                            }
                        }
                        _ => {}
                    }
                }
            }
            Some(api::response_event::Type::Finished(finished)) => {
                match &finished.reason {
                    Some(api::response_event::stream_finished::Reason::Done(_)) => {
                        self.finished_done = true;
                        self.usage = finished.conversation_usage_metadata.clone();
                        self.token_usage = finished.token_usage.clone();
                    }
                    Some(_) => {
                        self.finished_error = true;
                    }
                    None => {
                        self.finished_error = true;
                    }
                }
            }
            _ => {}
        }
        self.events.push(event);
    }
}
