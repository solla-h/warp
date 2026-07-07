//! Test fixture definitions.
//!
//! Fixtures live as SSE response files under:
//!   fixtures/{provider}/{scenario}/response_NNN.txt
//!
//! Available scenarios:
//!
//! ## anthropic/
//! - simple_text         — Single text response, 3 chunks
//! - tool_call_single    — Text + one tool_use (read_file)
//! - empty_response      — Message with 0 content blocks
//! - reasoning_with_text — Thinking block followed by text block
//!
//! ## Adding new fixtures:
//! 1. Create a directory: fixtures/{provider}/{scenario}/
//! 2. Add response_000.txt (and _001, _002 for multi-turn agentic loop scenarios)
//! 3. Format: Standard SSE (event: ...\ndata: {...}\n\n)

/// Path to fixtures directory (relative to CARGO_MANIFEST_DIR).
pub const FIXTURES_REL_PATH: &str = "src/ai/agent_providers/chat_stream_tests/fixtures";

/// List of available fixture scenarios for validation.
pub const AVAILABLE_SCENARIOS: &[(&str, &str)] = &[
    ("anthropic", "simple_text"),
    ("anthropic", "tool_call_single"),
    ("anthropic", "empty_response"),
    ("anthropic", "reasoning_with_text"),
];
