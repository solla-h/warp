#![allow(dead_code, unused_imports)]
//! Logging and diagnostic helpers for the chat-stream pipeline.
//!
//! Extracted from `mod.rs`: `snippet_for_log`, `log_chat_request_details`,
//! `scan_suspicious_backslash`, `log_accepted_history_repair`,
//! `accepted_history_repair_log_message`.

use std::fmt::Write as _;

use genai::chat::{
    Binary, BinarySource, ChatRequest, ChatRole, ContentPart,
};
use serde_json::Value;

use crate::ai::byop_readiness::{
    AcceptedRepair, ReadinessCategory, ReadinessDiagnosticContext, RepairSource,
};
use crate::settings::AgentProviderApiType;

use super::client::adapter_kind_for;

/// 诊断 snippet 截取最大字符数。
const BYOP_DIAG_SNIPPET_CHARS: usize = 240;

pub(crate) fn snippet_for_log(s: &str, max_chars: usize) -> String {
    use std::fmt::Write as _;

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

pub(crate) fn log_accepted_history_repair(
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

// PLACEHOLDER_SCAN_AND_LOG

