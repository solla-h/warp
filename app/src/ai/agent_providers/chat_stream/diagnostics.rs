#![allow(dead_code, unused_imports)]
//! Logging and diagnostic helpers for the chat-stream pipeline.
//!
//! Currently contains `snippet_for_log`. Additional diagnostics functions
//! (`log_chat_request_details`, `scan_suspicious_backslash`) will be extracted
//! alongside `request.rs` in a later phase.

use std::fmt::Write as _;

use genai::chat::{
    Binary, BinarySource, ChatRequest, ChatRole, ContentPart,
};
use serde_json::Value;

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

// PLACEHOLDER_SCAN_AND_LOG
