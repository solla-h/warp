//! Event constructor functions for building `warp_multi_agent_api` response
//! events and messages. These are pure data constructors with no I/O.

use serde_json::Value;
use uuid::Uuid;
use warp_multi_agent_api as api;

use super::user_context;

// ---------------------------------------------------------------------------
// AppendKind
// ---------------------------------------------------------------------------

pub(crate) enum AppendKind {
    Reasoning(String),
    Text(String),
}

// ---------------------------------------------------------------------------
// ResponseEvent constructors
// ---------------------------------------------------------------------------

pub(crate) fn make_add_messages_event(
    task_id: &str,
    messages: Vec<api::Message>,
) -> api::ResponseEvent {
    api::ResponseEvent {
        r#type: Some(api::response_event::Type::ClientActions(
            api::response_event::ClientActions {
                actions: vec![api::ClientAction {
                    action: Some(api::client_action::Action::AddMessagesToTask(
                        api::client_action::AddMessagesToTask {
                            task_id: task_id.to_owned(),
                            messages,
                        },
                    )),
                }],
            },
        )),
    }
}

/// Replace parts of an existing message using FieldMask.
pub(crate) fn make_update_message_event(
    task_id: &str,
    message: api::Message,
    mask_paths: Vec<String>,
) -> api::ResponseEvent {
    api::ResponseEvent {
        r#type: Some(api::response_event::Type::ClientActions(
            api::response_event::ClientActions {
                actions: vec![api::ClientAction {
                    action: Some(api::client_action::Action::UpdateTaskMessage(
                        api::client_action::UpdateTaskMessage {
                            task_id: task_id.to_owned(),
                            message: Some(message),
                            mask: Some(prost_types::FieldMask { paths: mask_paths }),
                        },
                    )),
                }],
            },
        )),
    }
}

pub(crate) fn make_append_event(
    task_id: &str,
    message_id: &str,
    kind: AppendKind,
) -> api::ResponseEvent {
    let (msg_inner, mask_path) = match kind {
        AppendKind::Reasoning(r) => (
            api::message::Message::AgentReasoning(api::message::AgentReasoning {
                reasoning: r,
                finished_duration: None,
            }),
            "agent_reasoning.reasoning",
        ),
        AppendKind::Text(t) => (
            api::message::Message::AgentOutput(api::message::AgentOutput { text: t }),
            "agent_output.text",
        ),
    };
    let message = api::Message {
        id: message_id.to_owned(),
        task_id: task_id.to_owned(),
        server_message_data: String::new(),
        citations: vec![],
        message: Some(msg_inner),
        request_id: String::new(),
        timestamp: None,
    };
    api::ResponseEvent {
        r#type: Some(api::response_event::Type::ClientActions(
            api::response_event::ClientActions {
                actions: vec![api::ClientAction {
                    action: Some(api::client_action::Action::AppendToMessageContent(
                        api::client_action::AppendToMessageContent {
                            task_id: task_id.to_owned(),
                            message: Some(message),
                            mask: Some(prost_types::FieldMask {
                                paths: vec![mask_path.to_owned()],
                            }),
                        },
                    )),
                }],
            },
        )),
    }
}

// ---------------------------------------------------------------------------
// Message constructors
// ---------------------------------------------------------------------------

pub(crate) fn make_reasoning_message(
    task_id: &str,
    request_id: &str,
    reasoning: String,
) -> api::Message {
    api::Message {
        id: Uuid::new_v4().to_string(),
        task_id: task_id.to_owned(),
        server_message_data: String::new(),
        citations: vec![],
        message: Some(api::message::Message::AgentReasoning(
            api::message::AgentReasoning {
                reasoning,
                finished_duration: None,
            },
        )),
        request_id: request_id.to_owned(),
        timestamp: None,
    }
}

pub(crate) fn make_agent_output_message(
    task_id: &str,
    request_id: &str,
    text: String,
) -> api::Message {
    api::Message {
        id: Uuid::new_v4().to_string(),
        task_id: task_id.to_owned(),
        server_message_data: String::new(),
        citations: vec![],
        message: Some(api::message::Message::AgentOutput(
            api::message::AgentOutput { text },
        )),
        request_id: request_id.to_owned(),
        timestamp: None,
    }
}

pub(crate) fn make_user_query_message(
    task_id: &str,
    request_id: &str,
    query: String,
    binaries: &[user_context::UserBinary],
) -> api::Message {
    let proto_binaries: Vec<api::input_context::Image> = binaries
        .iter()
        .filter_map(|b| {
            use base64::Engine;
            base64::engine::general_purpose::STANDARD
                .decode(&b.data)
                .ok()
                .map(|bytes| api::input_context::Image {
                    data: bytes,
                    mime_type: b.content_type.clone(),
                })
        })
        .collect();
    let context = if proto_binaries.is_empty() {
        None
    } else {
        Some(api::InputContext {
            images: proto_binaries,
            ..Default::default()
        })
    };
    api::Message {
        id: Uuid::new_v4().to_string(),
        task_id: task_id.to_owned(),
        server_message_data: String::new(),
        citations: vec![],
        message: Some(api::message::Message::UserQuery(api::message::UserQuery {
            query,
            context,
            ..Default::default()
        })),
        request_id: request_id.to_owned(),
        timestamp: None,
    }
}

/// Emit a `WebSearch(Searching{query})` loading card.
pub(crate) fn make_web_search_searching_message(
    task_id: &str,
    request_id: &str,
    query: String,
) -> api::Message {
    api::Message {
        id: Uuid::new_v4().to_string(),
        task_id: task_id.to_owned(),
        server_message_data: String::new(),
        citations: vec![],
        message: Some(api::message::Message::WebSearch(api::message::WebSearch {
            status: Some(api::message::web_search::Status {
                r#type: Some(api::message::web_search::status::Type::Searching(
                    api::message::web_search::status::Searching { query },
                )),
            }),
        })),
        request_id: request_id.to_owned(),
        timestamp: None,
    }
}

/// Extract (url, title) pairs from exa search results text.
pub(crate) fn extract_search_pages_from_exa_results(s: &str) -> Vec<(String, String)> {
    let mut pages = Vec::new();
    let mut seen = std::collections::HashSet::new();

    // Route 1: Title:/URL: line format
    let mut current_title: Option<String> = None;
    for line in s.lines() {
        let trimmed = line.trim_start();
        if let Some(rest) = trimmed.strip_prefix("Title:") {
            current_title = Some(rest.trim().to_owned());
        } else if let Some(rest) = trimmed.strip_prefix("URL:") {
            let url = rest.trim().to_owned();
            let title = current_title.take().unwrap_or_default();
            if (url.starts_with("http://") || url.starts_with("https://"))
                && seen.insert(url.clone())
            {
                pages.push((url, title));
            }
        }
    }

    // Route 2: markdown link `[title](url)` fallback (dedup already active)
    let bytes = s.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'[' {
            if let Some(rel_close_text) = s[i + 1..].find("](") {
                let text_end = i + 1 + rel_close_text;
                let url_start = text_end + 2;
                if let Some(rel_close_url) = s[url_start..].find(')') {
                    let url_end = url_start + rel_close_url;
                    let title = s[i + 1..text_end].trim().to_owned();
                    let url = s[url_start..url_end].trim().to_owned();
                    if (url.starts_with("http://") || url.starts_with("https://"))
                        && seen.insert(url.clone())
                    {
                        pages.push((url, title));
                    }
                    i = url_end + 1;
                    continue;
                }
            }
        }
        i += 1;
    }

    pages
}

/// Build websearch Success/Error status message from result JSON.
pub(crate) fn make_web_search_status_from_result(
    task_id: &str,
    request_id: &str,
    query: &str,
    result_json: &Value,
) -> api::Message {
    let is_error = result_json.get("status").and_then(|v| v.as_str()) == Some("error");
    let r#type = if is_error {
        api::message::web_search::status::Type::Error(())
    } else {
        let pages = result_json
            .get("results")
            .and_then(|v| v.as_str())
            .map(extract_search_pages_from_exa_results)
            .unwrap_or_default()
            .into_iter()
            .map(
                |(url, title)| api::message::web_search::status::success::SearchedPage {
                    url,
                    title,
                },
            )
            .collect();
        api::message::web_search::status::Type::Success(
            api::message::web_search::status::Success {
                query: query.to_owned(),
                pages,
            },
        )
    };
    api::Message {
        id: Uuid::new_v4().to_string(),
        task_id: task_id.to_owned(),
        server_message_data: String::new(),
        citations: vec![],
        message: Some(api::message::Message::WebSearch(api::message::WebSearch {
            status: Some(api::message::web_search::Status {
                r#type: Some(r#type),
            }),
        })),
        request_id: request_id.to_owned(),
        timestamp: None,
    }
}

/// Emit a `WebFetch(Fetching{urls})` loading card.
pub(crate) fn make_web_fetch_fetching_message(
    task_id: &str,
    request_id: &str,
    urls: Vec<String>,
) -> api::Message {
    api::Message {
        id: Uuid::new_v4().to_string(),
        task_id: task_id.to_owned(),
        server_message_data: String::new(),
        citations: vec![],
        message: Some(api::message::Message::WebFetch(api::message::WebFetch {
            status: Some(api::message::web_fetch::Status {
                r#type: Some(api::message::web_fetch::status::Type::Fetching(
                    api::message::web_fetch::status::Fetching { urls },
                )),
            }),
        })),
        request_id: request_id.to_owned(),
        timestamp: None,
    }
}

/// Build webfetch Success/Error status message from result JSON.
pub(crate) fn make_web_fetch_status_from_result(
    task_id: &str,
    request_id: &str,
    fallback_urls: &[String],
    result_json: &Value,
) -> api::Message {
    let is_error = result_json.get("status").and_then(|v| v.as_str()) == Some("error");
    let r#type = if is_error {
        api::message::web_fetch::status::Type::Error(())
    } else {
        let url = result_json
            .get("url")
            .and_then(|v| v.as_str())
            .map(|s| s.to_owned())
            .unwrap_or_else(|| fallback_urls.first().cloned().unwrap_or_default());
        let success = result_json
            .get("status")
            .and_then(|v| v.as_u64())
            .map(|c| (200..300).contains(&c))
            .unwrap_or(true);
        api::message::web_fetch::status::Type::Success(
            api::message::web_fetch::status::Success {
                pages: vec![api::message::web_fetch::status::success::FetchedPage {
                    url,
                    title: String::new(),
                    success,
                }],
            },
        )
    };
    api::Message {
        id: Uuid::new_v4().to_string(),
        task_id: task_id.to_owned(),
        server_message_data: String::new(),
        citations: vec![],
        message: Some(api::message::Message::WebFetch(api::message::WebFetch {
            status: Some(api::message::web_fetch::Status {
                r#type: Some(r#type),
            }),
        })),
        request_id: request_id.to_owned(),
        timestamp: None,
    }
}

// ---------------------------------------------------------------------------
// Tool call message constructors
// ---------------------------------------------------------------------------

pub(crate) fn make_tool_call_result_message(
    task_id: &str,
    request_id: &str,
    tool_call_id: String,
    content: String,
) -> api::Message {
    api::Message {
        id: Uuid::new_v4().to_string(),
        task_id: task_id.to_owned(),
        server_message_data: content,
        citations: vec![],
        message: Some(api::message::Message::ToolCallResult(
            api::message::ToolCallResult {
                tool_call_id,
                context: None,
                result: None,
            },
        )),
        request_id: request_id.to_owned(),
        timestamp: None,
    }
}

/// Carrier message for tool calls that failed `from_args` parsing.
pub(crate) fn make_tool_call_carrier_message(
    task_id: &str,
    request_id: &str,
    tool_call_id: &str,
    fn_name: &str,
    args_str: &str,
) -> api::Message {
    let carrier = format!("{}\n{}", fn_name, args_str);
    api::Message {
        id: Uuid::new_v4().to_string(),
        task_id: task_id.to_owned(),
        server_message_data: carrier,
        citations: vec![],
        message: Some(api::message::Message::ToolCall(api::message::ToolCall {
            tool_call_id: tool_call_id.to_owned(),
            tool: None,
        })),
        request_id: request_id.to_owned(),
        timestamp: None,
    }
}

pub(crate) fn make_tool_call_message(
    task_id: &str,
    request_id: &str,
    tool_call_id: &str,
    tool: api::message::tool_call::Tool,
) -> api::Message {
    api::Message {
        id: Uuid::new_v4().to_string(),
        task_id: task_id.to_owned(),
        server_message_data: String::new(),
        citations: vec![],
        message: Some(api::message::Message::ToolCall(api::message::ToolCall {
            tool_call_id: tool_call_id.to_owned(),
            tool: Some(tool),
        })),
        request_id: request_id.to_owned(),
        timestamp: None,
    }
}

// ---------------------------------------------------------------------------
// Task/subtask event constructors
// ---------------------------------------------------------------------------

pub(crate) fn create_task_event(task_id: &str) -> api::ResponseEvent {
    api::ResponseEvent {
        r#type: Some(api::response_event::Type::ClientActions(
            api::response_event::ClientActions {
                actions: vec![api::ClientAction {
                    action: Some(api::client_action::Action::CreateTask(
                        api::client_action::CreateTask {
                            task: Some(api::Task {
                                id: task_id.to_owned(),
                                description: String::new(),
                                dependencies: None,
                                messages: vec![],
                                summary: String::new(),
                                server_data: String::new(),
                            }),
                        },
                    )),
                }],
            },
        )),
    }
}

/// Create a subtask event with `dependencies.parent_task_id`.
pub(crate) fn create_subtask_event(
    subtask_id: &str,
    parent_task_id: &str,
) -> api::ResponseEvent {
    api::ResponseEvent {
        r#type: Some(api::response_event::Type::ClientActions(
            api::response_event::ClientActions {
                actions: vec![api::ClientAction {
                    action: Some(api::client_action::Action::CreateTask(
                        api::client_action::CreateTask {
                            task: Some(api::Task {
                                id: subtask_id.to_owned(),
                                description: String::new(),
                                dependencies: Some(api::task::Dependencies {
                                    parent_task_id: parent_task_id.to_owned(),
                                }),
                                messages: vec![],
                                summary: String::new(),
                                server_data: String::new(),
                            }),
                        },
                    )),
                }],
            },
        )),
    }
}

// ---------------------------------------------------------------------------
// Finished event
// ---------------------------------------------------------------------------

pub(crate) fn make_finished_done(
    usage_metadata: Option<api::response_event::stream_finished::ConversationUsageMetadata>,
) -> api::ResponseEvent {
    api::ResponseEvent {
        r#type: Some(api::response_event::Type::Finished(
            api::response_event::StreamFinished {
                reason: Some(api::response_event::stream_finished::Reason::Done(
                    api::response_event::stream_finished::Done {},
                )),
                conversation_usage_metadata: usage_metadata,
                token_usage: vec![],
                should_refresh_model_config: false,
                request_cost: None,
            },
        )),
    }
}
