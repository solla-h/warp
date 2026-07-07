//! Tests for chat_stream::diagnostics module.

use crate::ai::agent_providers::chat_stream::diagnostics::snippet_for_log;

#[test]
fn test_snippet_for_log_truncates() {
    let long = "a".repeat(300);
    let result = snippet_for_log(&long, 10);
    assert!(result.ends_with("..."), "should end with ...: {result}");
    // 10 chars + "..." = 13 max length
    assert!(result.len() <= 13, "should be truncated: len={}", result.len());
}

#[test]
fn test_snippet_for_log_short_passes_through() {
    let short = "hello";
    let result = snippet_for_log(short, 100);
    assert_eq!(result, "hello");
}

#[test]
fn test_snippet_for_log_escapes_control_chars() {
    let with_newline = "line1\nline2\ttab";
    let result = snippet_for_log(with_newline, 100);
    assert!(result.contains("\\n"), "newline should be escaped: {result}");
    assert!(result.contains("\\t"), "tab should be escaped: {result}");
    assert!(!result.contains('\n'), "raw newline should not appear");
}

#[test]
fn test_snippet_for_log_empty_string() {
    let result = snippet_for_log("", 100);
    assert_eq!(result, "");
}
