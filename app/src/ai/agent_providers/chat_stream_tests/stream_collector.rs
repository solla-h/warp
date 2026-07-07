//! Utilities for collecting and inspecting ResponseEvent streams in tests.

use futures::StreamExt;
use warp_multi_agent_api as api;

use crate::ai::agent::api::ResponseStream;

/// Collected output from consuming a full ResponseStream.
#[derive(Debug, Default)]
pub struct CollectedStream {
    /// All events in emission order.
    pub events: Vec<api::ResponseEvent>,
    /// Text content accumulated from AppendToMessageContent events.
    pub text_content: String,
    /// Reasoning content accumulated from AppendToMessageContent(Reasoning) events.
    pub reasoning_content: String,
    /// Tool calls emitted (AddMessages with ToolCall type).
    pub tool_calls: Vec<api::Message>,
    /// Whether StreamFinished(Done) was the terminal event.
    pub finished_done: bool,
    /// Whether StreamFinished(Error) was the terminal event.
    pub finished_error: bool,
    /// Usage metadata if available.
    pub usage: Option<api::UsageMeta>,
}

impl CollectedStream {
    /// Consume the entire stream and collect all events.
    pub async fn collect_from(mut stream: ResponseStream) -> Self {
        let mut collected = Self::default();

        while let Some(event) = stream.next().await {
            collected.process_event(event);
        }

        collected
    }

    /// How many events total.
    pub fn event_count(&self) -> usize {
        self.events.len()
    }

    /// Whether a CreateTask event was emitted.
    pub fn has_create_task(&self) -> bool {
        self.events.iter().any(|e| matches!(e, api::ResponseEvent::CreateTask { .. }))
    }

    /// Get all text messages (AgentOutput type).
    pub fn text_messages(&self) -> Vec<&api::Message> {
        self.events
            .iter()
            .filter_map(|e| match e {
                api::ResponseEvent::AddMessagesToTask { messages, .. } => Some(messages),
                _ => None,
            })
            .flatten()
            .filter(|m| m.r#type == api::MessageType::AgentOutput)
            .collect()
    }

    fn process_event(&mut self, event: api::ResponseEvent) {
        match &event {
            api::ResponseEvent::AppendToMessageContent { content, .. } => {
                self.text_content.push_str(content);
            }
            api::ResponseEvent::StreamFinished(api::StreamFinishedPayload::Done { usage, .. }) => {
                self.finished_done = true;
                self.usage = usage.clone();
            }
            api::ResponseEvent::StreamFinished(api::StreamFinishedPayload::Error { .. }) => {
                self.finished_error = true;
            }
            api::ResponseEvent::AddMessagesToTask { messages, .. } => {
                for msg in messages {
                    if msg.r#type == api::MessageType::ToolCall {
                        self.tool_calls.push(msg.clone());
                    }
                }
            }
            _ => {}
        }
        self.events.push(event);
    }
}
