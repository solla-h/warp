//! Real LLM provider integration tests.
//!
//! These tests hit an actual BYOP endpoint and verify the full pipeline works.
//! They are marked `#[ignore]` and only run when:
//!   - `BYOP_TEST_API_KEY` env var is set
//!   - Run explicitly with: `cargo nextest run -- --ignored`
//!
//! Configuration (env vars):
//!   BYOP_TEST_BASE_URL  -- default: "https://ds-api.xnurta.com/"
//!   BYOP_TEST_API_KEY   -- required (no default)
//!   BYOP_TEST_MODEL     -- default: "claude-opus-4-8"
//!   BYOP_TEST_API_TYPE  -- default: "Anthropic"

use std::time::Instant;

use super::builders;
use super::stream_collector::CollectedStream;
use super::assertions;

/// Skip helper: returns true if the real provider env is not configured.
fn should_skip() -> bool {
    std::env::var("BYOP_TEST_API_KEY").is_err()
}

#[tokio::test]
#[ignore]
async fn test_real_provider_simple_text() {
    if should_skip() {
        eprintln!("Skipping: BYOP_TEST_API_KEY not set");
        return;
    }

    let input = builders::real_provider_input().expect("Failed to build real provider input");
    let stream = crate::ai::agent_providers::chat_stream::generate_byop_output(input)
        .await
        .expect("generate_byop_output failed");

    let collected = CollectedStream::collect_from(stream).await;

    assertions::assert_stream_done(&collected);
    assertions::assert_has_text_content(&collected);
    assert!(
        !collected.text_content.is_empty(),
        "Expected non-empty text response from real provider"
    );
    eprintln!(
        "[real_provider] Got {} events, text length: {}",
        collected.event_count(),
        collected.text_content.len()
    );
}

#[tokio::test]
#[ignore]
async fn test_real_provider_streaming_is_incremental() {
    if should_skip() {
        return;
    }

    let input = builders::real_provider_input().expect("Failed to build real provider input");
    let start = Instant::now();

    let stream = crate::ai::agent_providers::chat_stream::generate_byop_output(input)
        .await
        .expect("generate_byop_output failed");

    let mut first_event_time = None;
    let mut events = Vec::new();

    use futures::StreamExt;
    let mut stream = stream;
    while let Some(event) = stream.next().await {
        if first_event_time.is_none() {
            first_event_time = Some(start.elapsed());
        }
        events.push(event);
    }

    let total_time = start.elapsed();
    let first_time = first_event_time.expect("No events received");

    // First event should arrive much sooner than total completion
    // (streaming, not buffered)
    let ratio = first_time.as_millis() as f64 / total_time.as_millis().max(1) as f64;
    eprintln!(
        "[real_provider] First event at {:?}, total {:?}, ratio: {:.2}",
        first_time, total_time, ratio
    );

    assert!(
        ratio < 0.5,
        "First event arrived at {:.0}% of total time -- response may be buffered, not streamed",
        ratio * 100.0
    );
}

#[tokio::test]
#[ignore]
async fn test_real_provider_has_usage_metadata() {
    if should_skip() {
        return;
    }

    let input = builders::real_provider_input().expect("Failed to build real provider input");
    let stream = crate::ai::agent_providers::chat_stream::generate_byop_output(input)
        .await
        .expect("generate_byop_output failed");

    let collected = CollectedStream::collect_from(stream).await;

    assertions::assert_stream_done(&collected);
    // Token usage is reported per-model in the StreamFinished event.
    assert!(
        !collected.token_usage.is_empty(),
        "Expected token_usage in StreamFinished event"
    );

    let first = &collected.token_usage[0];
    eprintln!(
        "[real_provider] Usage: input={}, output={}",
        first.total_input, first.output,
    );
    // At minimum, output tokens should be > 0 for a non-empty response
    assert!(first.output > 0, "Expected output tokens > 0");
}

#[tokio::test]
#[ignore]
async fn test_real_provider_first_turn_creates_task() {
    if should_skip() {
        return;
    }

    let input = builders::real_provider_input().expect("Failed to build real provider input");
    let stream = crate::ai::agent_providers::chat_stream::generate_byop_output(input)
        .await
        .expect("generate_byop_output failed");

    let collected = CollectedStream::collect_from(stream).await;

    assertions::assert_stream_done(&collected);
    assertions::assert_has_create_task(&collected);
}

#[tokio::test]
#[ignore]
async fn test_real_provider_event_order_correct() {
    if should_skip() {
        return;
    }

    let input = builders::real_provider_input().expect("Failed to build real provider input");
    let stream = crate::ai::agent_providers::chat_stream::generate_byop_output(input)
        .await
        .expect("generate_byop_output failed");

    let collected = CollectedStream::collect_from(stream).await;

    // Verify fundamental event ordering: init -> create_task -> text -> done
    assertions::assert_event_order(&collected, &["init", "create_task", "*", "done"]);
}
