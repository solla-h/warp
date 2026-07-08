//! Unit tests for the `chat_stream::title` module.

use crate::ai::agent_providers::chat_stream::title::sanitize_title;

/// Surrounding quotes (ASCII double-quotes) are stripped.
#[test]
fn test_sanitize_title_strips_quotes() {
    let result = sanitize_title("\"Hello World\"").unwrap();
    assert_eq!(result, "Hello World");
}

/// Reasoning tags like `<think>...</think>` are stripped, leaving only the title.
#[test]
fn test_sanitize_title_strips_reasoning_tags() {
    let input = "<think>Let me think about a good title...</think>My Great Title";
    let result = sanitize_title(input).unwrap();
    assert_eq!(result, "My Great Title");
}

/// Titles longer than 50 characters are truncated and suffixed with ellipsis.
#[test]
fn test_sanitize_title_truncates_at_50() {
    let long_title = "A".repeat(60);
    let result = sanitize_title(&long_title).unwrap();
    let chars: Vec<char> = result.chars().collect();
    assert_eq!(chars.len(), 50, "must be exactly 50 chars after truncation");
    assert_eq!(*chars.last().unwrap(), '\u{2026}', "must end with ellipsis");
    // First 49 chars should be 'A'
    assert!(chars[..49].iter().all(|&c| c == 'A'));
}

/// Multiline input: only the first non-empty line is used as the title.
#[test]
fn test_sanitize_title_takes_first_line() {
    let input = "\n\n  First Line  \nSecond Line\nThird Line";
    let result = sanitize_title(input).unwrap();
    assert_eq!(result, "First Line");
}

/// Empty or whitespace-only input returns None.
#[test]
fn test_sanitize_title_empty_returns_none() {
    assert!(sanitize_title("").is_none());
    assert!(sanitize_title("   ").is_none());
    assert!(sanitize_title("\n\n").is_none());
}

/// Common prefixes like "Title:" are stripped.
#[test]
fn test_sanitize_title_strips_prefix() {
    assert_eq!(
        sanitize_title("Title: My Conversation").unwrap(),
        "My Conversation"
    );
    assert_eq!(
        sanitize_title("Subject: Bug Report").unwrap(),
        "Bug Report"
    );
}

/// Trailing punctuation is removed.
#[test]
fn test_sanitize_title_strips_trailing_punctuation() {
    assert_eq!(
        sanitize_title("Hello World.").unwrap(),
        "Hello World"
    );
    assert_eq!(
        sanitize_title("Question?").unwrap(),
        "Question"
    );
}
