//! Message serialization layer for `chat_stream`.
//!
//! Handles converting internal message representations to/from the genai
//! ChatMessage format, including assistant buffer management, tool call
//! serialization, and multi-task message linearization.

use std::collections::{HashMap, HashSet};

use serde_json::{json, Value};
use warp_multi_agent_api as api;

use genai::chat::{
    Binary, ChatMessage, ChatRole, ContentPart, MessageContent, ToolCall,
};

use crate::ai::byop_readiness::ToolCallKey;

use super::attachment_caps;
use super::tools;
use super::user_context;

// ---------------------------------------------------------------------------
// Constants
// ---------------------------------------------------------------------------

pub(crate) const REASONING_ECHO_PLACEHOLDER: &str = " ";

// ---------------------------------------------------------------------------
// AssistantBuffer
// ---------------------------------------------------------------------------

/// Accumulates text + tool_calls + reasoning for a single assistant turn,
/// then flushes into one or two `ChatMessage` instances.
#[derive(Default)]
pub(crate) struct AssistantBuffer {
    pub(crate) text: Option<String>,
    pub(crate) tool_calls: Vec<ToolCall>,
    pub(crate) tool_call_keys: Vec<ToolCallKey>,
    pub(crate) reasoning: Option<String>,
    pub(crate) force_echo_reasoning: bool,
}

impl AssistantBuffer {
    pub(crate) fn new(force_echo_reasoning: bool) -> Self {
        Self {
            force_echo_reasoning,
            ..Default::default()
        }
    }

    pub(crate) fn push_tool_call(&mut self, tool_call: ToolCall, key: ToolCallKey) {
        self.tool_calls.push(tool_call);
        self.tool_call_keys.push(key);
    }

    pub(crate) fn flush_into(&mut self, messages: &mut Vec<ChatMessage>) {
        let _ = self.flush_into_with_group(messages);
    }

    pub(crate) fn flush_into_with_group(
        &mut self,
        messages: &mut Vec<ChatMessage>,
    ) -> Option<OutboundAssistantToolGroup> {
        let reasoning = self.reasoning.take();
        let has_tool_calls = !self.tool_calls.is_empty();
        let echo_reasoning: Option<String> = if self.force_echo_reasoning {
            match reasoning {
                Some(r) if !r.is_empty() => Some(r),
                _ => Some(REASONING_ECHO_PLACEHOLDER.to_owned()),
            }
        } else {
            None
        };
        if let Some(t) = self.text.take() {
            let mut msg = ChatMessage::assistant(t);
            if has_tool_calls {
                if self.force_echo_reasoning {
                    msg = msg.with_reasoning_content(Some(REASONING_ECHO_PLACEHOLDER.to_owned()));
                }
            } else if let Some(r) = echo_reasoning.clone() {
                msg = msg.with_reasoning_content(Some(r));
            }
            messages.push(msg);
        }
        if has_tool_calls {
            let mut msg = ChatMessage::from(std::mem::take(&mut self.tool_calls));
            if let Some(r) = echo_reasoning {
                msg = msg.with_reasoning_content(Some(r));
            }
            let message_index = messages.len();
            messages.push(msg);
            Some(OutboundAssistantToolGroup {
                message_index,
                tool_call_keys: std::mem::take(&mut self.tool_call_keys),
            })
        } else {
            self.tool_call_keys.clear();
            None
        }
    }
}

// ---------------------------------------------------------------------------
// OutboundAssistantToolGroup
// ---------------------------------------------------------------------------

#[derive(Debug, Clone)]
pub(crate) struct OutboundAssistantToolGroup {
    pub(crate) message_index: usize,
    pub(crate) tool_call_keys: Vec<ToolCallKey>,
}

// ---------------------------------------------------------------------------
// flush_assistant_buffer
// ---------------------------------------------------------------------------

pub(crate) fn flush_assistant_buffer(
    buf: &mut AssistantBuffer,
    messages: &mut Vec<ChatMessage>,
    outbound_tool_groups: &mut Vec<OutboundAssistantToolGroup>,
) {
    if let Some(group) = buf.flush_into_with_group(messages) {
        if !group.tool_call_keys.is_empty() {
            outbound_tool_groups.push(group);
        }
    }
}

// ---------------------------------------------------------------------------
// build_user_message_with_binaries
// ---------------------------------------------------------------------------

/// Construct a user `ChatMessage`, switching to multimodal `Parts` form
/// when binary attachments are present and supported.
pub(crate) fn build_user_message_with_binaries(
    text: String,
    binaries: Vec<user_context::UserBinary>,
    caps: attachment_caps::AttachmentCaps,
) -> ChatMessage {
    if binaries.is_empty() {
        return ChatMessage::user(text);
    }

    let mut parts: Vec<ContentPart> = Vec::with_capacity(1 + binaries.len());
    parts.push(ContentPart::Text(text));

    let mut error_replacements: Vec<(String, String)> = Vec::new();
    for bin in binaries {
        if !caps.supports_mime(&bin.content_type) {
            let modality = mime_to_modality(&bin.content_type);
            let name = if bin.name.is_empty() {
                modality.to_string()
            } else {
                format!("\"{}\"", bin.name)
            };
            let err_text = format!(
                "ERROR: Cannot read {name} (this model does not support {modality} input). Inform the user."
            );
            error_replacements.push((bin.name.clone(), bin.content_type.clone()));
            parts.push(ContentPart::Text(err_text));
            continue;
        }
        parts.push(ContentPart::Binary(Binary::from_base64(
            bin.content_type,
            bin.data,
            Some(bin.name),
        )));
    }

    if !error_replacements.is_empty() {
        log::info!(
            "[byop] {} attachment(s) replaced with ERROR text — caps={caps:?} does not support: {error_replacements:?}",
            error_replacements.len()
        );
    }

    if parts.len() == 1 {
        if let Some(ContentPart::Text(t)) = parts.into_iter().next() {
            return ChatMessage::user(t);
        }
        return ChatMessage::user("");
    }

    ChatMessage {
        role: ChatRole::User,
        content: MessageContent::from_parts(parts),
        options: None,
    }
}

// ---------------------------------------------------------------------------
// mime_to_modality
// ---------------------------------------------------------------------------

/// MIME to modality string mapping (aligned with opencode `mimeToModality`).
pub(crate) fn mime_to_modality(mime: &str) -> &'static str {
    let lower = mime.trim().to_ascii_lowercase();
    if lower.starts_with("image/") {
        "image"
    } else if lower.starts_with("audio/") {
        "audio"
    } else if lower.starts_with("video/") {
        "video"
    } else if lower == "application/pdf" {
        "pdf"
    } else {
        "file"
    }
}

// ---------------------------------------------------------------------------
// collect_linearized_task_messages
// ---------------------------------------------------------------------------

/// Collect `params.tasks` into a stable linear message sequence (Issue #94 fix).
///
/// DFS from root task, following Subagent tool calls into subtasks.
/// UserQuery messages are deduplicated by `(request_id, query)`.
/// Orphan tasks are appended sorted by id for determinism.
pub(crate) fn collect_linearized_task_messages(tasks: &[api::Task]) -> Vec<&api::Message> {
    if tasks.is_empty() {
        return Vec::new();
    }

    let by_id: HashMap<&str, &api::Task> = tasks.iter().map(|t| (t.id.as_str(), t)).collect();

    let root = tasks.iter().find(|t| match t.dependencies.as_ref() {
        None => true,
        Some(dep) => {
            dep.parent_task_id.is_empty() || !by_id.contains_key(dep.parent_task_id.as_str())
        }
    });

    fn push_msg<'a>(
        msg: &'a api::Message,
        out: &mut Vec<&'a api::Message>,
        seen_user_queries: &mut HashSet<(&'a str, &'a str)>,
    ) {
        if let Some(api::message::Message::UserQuery(u)) = &msg.message {
            if !msg.request_id.is_empty()
                && !seen_user_queries.insert((msg.request_id.as_str(), u.query.as_str()))
            {
                return;
            }
        }
        out.push(msg);
    }

    fn dfs<'a>(
        task: &'a api::Task,
        by_id: &HashMap<&'a str, &'a api::Task>,
        visited_tasks: &mut HashSet<&'a str>,
        out: &mut Vec<&'a api::Message>,
        seen_user_queries: &mut HashSet<(&'a str, &'a str)>,
    ) {
        if !visited_tasks.insert(task.id.as_str()) {
            return;
        }
        for msg in &task.messages {
            push_msg(msg, out, seen_user_queries);
            if let Some(api::message::Message::ToolCall(tc)) = &msg.message {
                if let Some(api::message::tool_call::Tool::Subagent(sub)) = &tc.tool {
                    if let Some(subtask) = by_id.get(sub.task_id.as_str()) {
                        dfs(subtask, by_id, visited_tasks, out, seen_user_queries);
                    }
                }
            }
        }
    }

    let mut out: Vec<&api::Message> = Vec::new();
    let mut visited_tasks: HashSet<&str> = HashSet::new();
    let mut seen_user_queries: HashSet<(&str, &str)> = HashSet::new();

    if let Some(root) = root {
        dfs(
            root,
            &by_id,
            &mut visited_tasks,
            &mut out,
            &mut seen_user_queries,
        );
    }

    // Orphan task fallback: sort by id for determinism.
    let mut orphans: Vec<&api::Task> = tasks
        .iter()
        .filter(|t| !visited_tasks.contains(t.id.as_str()))
        .collect();
    orphans.sort_by(|a, b| a.id.cmp(&b.id));
    for task in orphans {
        if !visited_tasks.insert(task.id.as_str()) {
            continue;
        }
        for msg in &task.messages {
            push_msg(msg, &mut out, &mut seen_user_queries);
        }
    }

    out
}

// ---------------------------------------------------------------------------
// serialize_outgoing_tool_call
// ---------------------------------------------------------------------------

/// Serialize an outgoing tool call into `(function_name, args_json_value)`.
pub(crate) fn serialize_outgoing_tool_call(
    tc: &api::message::ToolCall,
    mcp_ctx: Option<&crate::ai::agent::MCPContext>,
    server_message_data: &str,
) -> (String, Value) {
    use api::message::tool_call::Tool;

    // BYOP from_args parse-failure carrier restoration.
    if tc.tool.is_none() {
        if let Some((fn_name, raw_args)) = server_message_data.split_once('\n') {
            if !fn_name.is_empty() {
                let args_value = serde_json::from_str(raw_args)
                    .unwrap_or_else(|_| Value::String(raw_args.to_owned()));
                return (fn_name.to_owned(), args_value);
            }
        }
    }

    let (name, args_str) = match &tc.tool {
        Some(Tool::CallMcpTool(c)) => tools::mcp::serialize_outgoing_call(c, mcp_ctx),
        Some(Tool::ReadMcpResource(r)) => tools::mcp::serialize_outgoing_read_resource(r, mcp_ctx),
        Some(Tool::RunShellCommand(c)) => (
            "run_shell_command".to_owned(),
            json!({
                "command": c.command,
                "is_read_only": c.is_read_only,
                "uses_pager": c.uses_pager,
                "is_risky": c.is_risky,
            })
            .to_string(),
        ),
        Some(Tool::ReadFiles(r)) => {
            let files: Vec<Value> = r
                .files
                .iter()
                .map(|f| {
                    json!({
                        "path": f.name,
                        "line_ranges": f.line_ranges.iter().map(|lr| json!({
                            "start": lr.start, "end": lr.end
                        })).collect::<Vec<_>>(),
                    })
                })
                .collect();
            (
                "read_files".to_owned(),
                json!({ "files": files }).to_string(),
            )
        }
        Some(Tool::Grep(g)) => (
            "grep".to_owned(),
            json!({ "queries": g.queries, "path": g.path }).to_string(),
        ),
        Some(Tool::AskUserQuestion(a)) => {
            let questions: Vec<Value> = a
                .questions
                .iter()
                .map(|q| {
                    let (options, recommended_index, multi_select, supports_other) =
                        match &q.question_type {
                            Some(
                                api::ask_user_question::question::QuestionType::MultipleChoice(mc),
                            ) => (
                                mc.options
                                    .iter()
                                    .map(|o| o.label.clone())
                                    .collect::<Vec<_>>(),
                                mc.recommended_option_index,
                                mc.is_multiselect,
                                mc.supports_other,
                            ),
                            None => (vec![], 0, false, false),
                        };
                    json!({
                        "question": q.question,
                        "options": options,
                        "recommended_index": recommended_index,
                        "multi_select": multi_select,
                        "supports_other": supports_other,
                    })
                })
                .collect();
            (
                "ask_user_question".to_owned(),
                json!({ "questions": questions }).to_string(),
            )
        }
        Some(Tool::FileGlobV2(g)) => (
            "file_glob".to_owned(),
            json!({
                "patterns": g.patterns,
                "search_dir": g.search_dir,
                "limit": g.max_matches,
            })
            .to_string(),
        ),
        Some(Tool::ApplyFileDiffs(a)) => {
            let mut operations: Vec<Value> = Vec::new();
            for d in &a.diffs {
                operations.push(json!({
                    "op": "edit",
                    "file_path": d.file_path,
                    "search": d.search,
                    "replace": d.replace,
                }));
            }
            for f in &a.new_files {
                operations.push(json!({
                    "op": "create",
                    "file_path": f.file_path,
                    "content": f.content,
                }));
            }
            for f in &a.deleted_files {
                operations.push(json!({
                    "op": "delete",
                    "file_path": f.file_path,
                }));
            }
            (
                "apply_file_diffs".to_owned(),
                json!({ "summary": a.summary, "operations": operations }).to_string(),
            )
        }
        Some(Tool::WriteToLongRunningShellCommand(w)) => {
            use api::message::tool_call::write_to_long_running_shell_command::mode::Mode as M;
            let mode = match w.mode.as_ref().and_then(|m| m.mode.as_ref()) {
                Some(M::Raw(_)) => "raw",
                Some(M::Block(_)) => "block",
                _ => "line",
            };
            (
                "write_to_long_running_shell_command".to_owned(),
                json!({
                    "command_id": w.command_id,
                    "input": String::from_utf8_lossy(&w.input).to_string(),
                    "mode": mode,
                })
                .to_string(),
            )
        }
        Some(Tool::ReadDocuments(r)) => {
            let docs: Vec<Value> = r
                .documents
                .iter()
                .map(|d| {
                    json!({
                        "document_id": d.document_id,
                        "line_ranges": d.line_ranges.iter().map(|lr| json!({
                            "start": lr.start, "end": lr.end
                        })).collect::<Vec<_>>(),
                    })
                })
                .collect();
            (
                "read_documents".to_owned(),
                json!({ "documents": docs }).to_string(),
            )
        }
        Some(Tool::EditDocuments(e)) => {
            let diffs: Vec<Value> = e
                .diffs
                .iter()
                .map(|d| {
                    json!({
                        "document_id": d.document_id,
                        "search": d.search,
                        "replace": d.replace,
                    })
                })
                .collect();
            (
                "edit_documents".to_owned(),
                json!({ "diffs": diffs }).to_string(),
            )
        }
        Some(Tool::CreateDocuments(c)) => {
            let new_documents: Vec<Value> = c
                .new_documents
                .iter()
                .map(|d| json!({ "title": d.title, "content": d.content }))
                .collect();
            (
                "create_documents".to_owned(),
                json!({ "new_documents": new_documents }).to_string(),
            )
        }
        Some(Tool::SuggestNewConversation(s)) => (
            "suggest_new_conversation".to_owned(),
            json!({ "message_id": s.message_id }).to_string(),
        ),
        Some(Tool::SuggestPrompt(s)) => {
            use api::message::tool_call::suggest_prompt::DisplayMode;
            let (prompt, label) = match &s.display_mode {
                Some(DisplayMode::PromptChip(c)) => (c.prompt.clone(), c.label.clone()),
                Some(DisplayMode::InlineQueryBanner(b)) => (b.query.clone(), b.title.clone()),
                None => (String::new(), String::new()),
            };
            (
                "suggest_prompt".to_owned(),
                json!({ "prompt": prompt, "label": label }).to_string(),
            )
        }
        Some(Tool::OpenCodeReview(_)) => ("open_code_review".to_owned(), "{}".to_owned()),
        Some(Tool::TransferShellCommandControlToUser(t)) => (
            "transfer_shell_command_control_to_user".to_owned(),
            json!({ "reason": t.reason }).to_string(),
        ),
        Some(Tool::ReadSkill(r)) => {
            use api::message::tool_call::read_skill::SkillReference;
            let name = match &r.skill_reference {
                Some(SkillReference::SkillPath(s)) => s.clone(),
                Some(SkillReference::BundledSkillId(id)) => format!("@warp-skill:{id}"),
                None => String::new(),
            };
            (
                "read_skill".to_owned(),
                json!({ "name": name }).to_string(),
            )
        }
        Some(Tool::ReadShellCommandOutput(r)) => {
            use api::message::tool_call::read_shell_command_output::Delay;
            let delay_seconds = match &r.delay {
                Some(Delay::Duration(d)) => Some(d.seconds),
                Some(Delay::OnCompletion(_)) | None => None,
            };
            let mut args = json!({ "command_id": r.command_id });
            if let Some(s) = delay_seconds {
                args["delay_seconds"] = json!(s);
            }
            ("read_shell_command_output".to_owned(), args.to_string())
        }
        Some(other) => {
            let variant_name = format!("{other:?}")
                .split('(')
                .next()
                .unwrap_or("UnknownVariant")
                .to_owned();
            (format!("warp_internal_{}", variant_name), "{}".to_owned())
        }
        None => ("warp_internal_empty".to_owned(), "{}".to_owned()),
    };
    let args_value: Value =
        serde_json::from_str(&args_str).unwrap_or(Value::Object(Default::default()));
    (name, args_value)
}

// ---------------------------------------------------------------------------
// serialize_outgoing_tool_call_for_test
// ---------------------------------------------------------------------------

pub(crate) fn serialize_outgoing_tool_call_for_test(
    tc: &api::message::ToolCall,
    mcp_ctx: Option<&crate::ai::agent::MCPContext>,
    server_message_data: &str,
) -> (String, Value) {
    serialize_outgoing_tool_call(tc, mcp_ctx, server_message_data)
}
