//! Unit tests for the `chat_stream::repair` module.

use crate::ai::agent_providers::chat_stream::repair::{
    is_placeholder_tool_response_content, repair_placeholder_content, REPAIR_PLACEHOLDER_NOTE,
};
use crate::ai::byop_readiness::RepairSource;

/// Known structured placeholder JSON must be detected as a placeholder.
#[test]
fn test_is_placeholder_detection_positive() {
    let forked = repair_placeholder_content(RepairSource::ForkedHistory);
    assert!(
        is_placeholder_tool_response_content(&forked),
        "ForkedHistory placeholder not detected: {forked}"
    );

    let restored = repair_placeholder_content(RepairSource::RestoredLegacyHistory);
    assert!(
        is_placeholder_tool_response_content(&restored),
        "RestoredLegacyHistory placeholder not detected: {restored}"
    );

    // Legacy Chinese placeholder string
    assert!(is_placeholder_tool_response_content(
        "(tool \u{6267}\u{884c}\u{7ed3}\u{679c}\u{672a}\u{4fdd}\u{7559})"
    ));
}

/// Normal tool output content must NOT be detected as a placeholder.
#[test]
fn test_is_placeholder_detection_negative() {
    assert!(!is_placeholder_tool_response_content(r#"{"status":"ok"}"#));
    assert!(!is_placeholder_tool_response_content("plain text result"));
    assert!(!is_placeholder_tool_response_content(""));

    // JSON that looks similar but has wrong/extra fields
    assert!(!is_placeholder_tool_response_content(
        r#"{"status":"unavailable","reason":"service_down","note":"try later"}"#
    ));
    // Extra key breaks the len==3 check
    assert!(!is_placeholder_tool_response_content(
        r#"{"status":"unavailable","reason":"forked_history_repair","note":"tool result was unavailable in repaired conversation history","extra":true}"#
    ));
}

/// `repair_placeholder_content` must produce the exact structured shape:
/// `{"status":"unavailable","reason":"<source>","note":"<REPAIR_PLACEHOLDER_NOTE>"}`
#[test]
fn test_repair_placeholder_content_format() {
    let content = repair_placeholder_content(RepairSource::ForkedHistory);
    let parsed: serde_json::Value = serde_json::from_str(&content).expect("must be valid JSON");
    let obj = parsed.as_object().expect("must be an object");

    assert_eq!(obj.len(), 3, "must have exactly 3 keys");
    assert_eq!(obj["status"], "unavailable");
    assert_eq!(obj["reason"], "forked_history_repair");
    assert_eq!(obj["note"], REPAIR_PLACEHOLDER_NOTE);

    // Also check restored legacy variant
    let content2 = repair_placeholder_content(RepairSource::RestoredLegacyHistory);
    let parsed2: serde_json::Value = serde_json::from_str(&content2).unwrap();
    assert_eq!(parsed2["reason"], "restored_legacy_history_repair");
}
