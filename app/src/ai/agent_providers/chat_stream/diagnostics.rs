//! Logging and diagnostic helpers for the chat-stream pipeline.
//!
//! Functions: `snippet_for_log`, `json_value_for_log`, `binary_for_log`,
//! `log_chat_request_details`.

use std::fmt::Write as _;

use genai::chat::{
    Binary, BinarySource, ChatRequest, ChatRole, ContentPart,
};
use serde_json::Value;

use crate::settings::AgentProviderApiType;

use super::client::adapter_kind_for;
use super::repair::is_placeholder_tool_response_content;

/// 诊断 snippet 截取最大字符数。
pub(crate) const BYOP_DIAG_SNIPPET_CHARS: usize = 240;

pub(crate) fn snippet_for_log(s: &str, max_chars: usize) -> String {
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

pub(crate) fn json_value_for_log(value: &Value) -> (usize, String) {
    let json = serde_json::to_string(value)
        .unwrap_or_else(|_| "<failed-to-serialize-json-value>".to_owned());
    (json.len(), snippet_for_log(&json, BYOP_DIAG_SNIPPET_CHARS))
}

pub(crate) fn binary_for_log(binary: &Binary) -> String {
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

pub(crate) fn log_chat_request_details(
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
