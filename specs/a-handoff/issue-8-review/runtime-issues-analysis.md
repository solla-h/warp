# Runtime Issues Analysis — warp-oss BYOP Agent

## Issue 1: websearch/webfetch tools break the agent loop

### Root Cause

`saved_stream_end` is declared at line 3536 of `chat_stream.rs` but **never assigned
a value**. The `ChatStreamEvent::End(end)` handler (line 3840) processes usage and
tool buffers from the stream end event but never stores the `end` into
`saved_stream_end`.

When the agentic loop condition fires (line 4289):

```rust
if !intercepted_tool_responses.is_empty()
    && !has_non_intercepted_tool
    && agentic_loop_iter < 8
{
    agentic_loop_iter += 1;
    if let Some(end) = saved_stream_end.take() {  // ALWAYS None!
        if let Some(content) = end.captured_content {
            chat_req.messages.push(ChatMessage::assistant(content));
        }
        for tr in intercepted_tool_responses.drain(..) {
            chat_req.messages.push(ChatMessage::from(tr));
        }
    }
    continue;
}
```
`saved_stream_end.take()` returns `None`, so the `if let Some(end)` body is
skipped. The tool responses are NEVER appended to `chat_req.messages`. The model
gets re-invoked with the identical request (no assistant message, no tool results),
producing an infinite loop of the same tool calls until `agentic_loop_iter >= 8`,
at which point the stream terminates with `break` at line 4335.

### Manifest Location

- **File**: `app/src/ai/agent_providers/chat_stream.rs`
- **Line 3536**: `let mut saved_stream_end: Option<genai::chat::StreamEnd> = None;`
- **Line 3840-3887**: `ChatStreamEvent::End(end)` handler — never assigns to `saved_stream_end`
- **Line 4291**: `if let Some(end) = saved_stream_end.take()` — always `None`

### Fix Required

In the `ChatStreamEvent::End(end)` handler (around line 3840), add:

```rust
saved_stream_end = Some(end.clone());
```

(or restructure to move ownership) before processing usage/tool_bufs, so that
line 4291 can successfully extract the assistant content and tool responses.

---

## Issue 2: No thinking/tooluse display during execution

### Root Cause

The SDK CLI output path (`AgentDriver::write_exchange_output` in `driver.rs` at
line 3234) only writes exchange data when `exchange.output_status.is_finished()`
(line 2954). There is no incremental/streaming output during execution.

The BYOP streaming layer (`chat_stream.rs`) correctly emits `AgentReasoning`
messages (lines 3673-3681, 3746-3753) and `ToolCall` placeholder cards (via
`make_tool_call_message` and friends) as protobuf events. These events reach
the `BlocklistAIHistoryModel` and update the conversation model in real time —
so the GUI terminal view DOES show them.

However, the **CLI/SDK output path** in `driver.rs` only calls
`write_exchange_output` once per exchange at completion, and the
`write_exchange_inputs` only emits action results (tool execution results, not
tool invocations or reasoning).

The `output.rs` module (`format_output`) handles `AIAgentOutputMessageType::Reasoning`
and `AIAgentOutputMessageType::Action` — but only when the full exchange has
completed streaming. During streaming, nothing is written to stdout.

### Manifest Location

- **File**: `app/src/ai/agent_sdk/driver.rs`
- **Line 2954**: `if exchange.output_status.is_finished()` — only emits at end
- **File**: `app/src/ai/agent_sdk/driver/output.rs`
- No streaming/progressive output functions exist

### Fix Required

Add incremental output in the `UpdatedStreamingExchange` handler. Either:

1. Emit partial output during streaming (new `write_streaming_delta` function
   that writes reasoning chunks and tool call invocations as they arrive), or
2. At minimum, emit tool_use blocks as they start (from `AppendedExchange` or
   intermediate streaming updates) so users see activity before the turn ends.

The existing `OutputFormat::Pretty` variant could be leveraged for a rich
streaming display while `OutputFormat::Text` stays batch-oriented.

---

## Issue 3: Requests to the endpoint are too large (900k+ tokens)

### Root Cause

The BYOP compaction system has the overflow detection infrastructure but **never
triggers it automatically**. Specifically:

1. `byop_compaction::overflow::is_overflow()` (line 86 of `overflow.rs`) is
   defined and exported but **never called** anywhere in non-test runtime code.

2. `build_chat_request` (line 1182 of `chat_stream.rs`) builds the full message
   payload from `params.tasks` (line 1217: `collect_linearized_task_messages`),
   which contains ALL messages from the entire conversation history.

3. The compaction state (`params.compaction_state`) is used to filter/summarize
   messages IF compaction has previously run, but nothing triggers the initial
   compaction when token counts exceed the model's context window.

4. The `context_window_usage` percentage IS calculated and stored in
   `ConversationUsageMetadata` (lines 4308-4333), but this is purely
   informational — no code reads it back to trigger a `SummarizeConversation`
   input on the next turn.

5. The only way compaction currently triggers is via the manual `/compact` slash
   command (found in `controller/slash_command.rs` line 278), which requires
   explicit user action.

The result: as conversations grow, every new request sends the ENTIRE uncompacted
history to the LLM. With tool-heavy workflows (websearch results, file contents,
command outputs), this easily exceeds 900k tokens.

### Manifest Location

- **File**: `app/src/ai/byop_compaction/overflow.rs`
- **Line 86**: `is_overflow()` — exists but never called in runtime
- **File**: `app/src/ai/agent_providers/chat_stream.rs`
- **Line 1217**: `collect_linearized_task_messages(&params.tasks)` — no truncation
- **File**: `app/src/ai/blocklist/controller.rs`
- **Line 3108-3135**: `handle_response_stream_finished` — stores usage but never
  triggers compaction
- **File**: `app/src/ai/blocklist/controller/slash_command.rs`
- **Line 278**: Only manual `/compact` trigger exists

### Fix Required

Add automatic compaction triggering. After each successful stream completion
in `handle_response_stream_finished`, check:

```rust
let cfg = byop_compaction::CompactionConfig::from_settings(ctx);
let tokens = /* extract from conversation_usage_metadata */;
let model_limit = /* resolve from current model metadata */;
if byop_compaction::is_overflow(&cfg, tokens, model_limit) {
    // Enqueue AIAgentInput::SummarizeConversation
}
```

This closes the loop between the overflow detection infrastructure (which is
fully implemented) and the controller's response handling (which currently
ignores the overflow signal).

---

## Summary of Interaction Between Issues

Issues 1 and 3 compound each other:

- Issue 1 causes the loop to re-send the same request up to 8 times (the cap),
  each time accumulating the SAME tool responses in the message history without
  the model ever seeing them (because `saved_stream_end` is never populated).

- Issue 3 means the full uncompacted history is sent every time, and since
  compaction never fires, context grows without bound.

- Together: a single websearch call generates 8 duplicate requests, each carrying
  the full untruncated history, easily hitting 900k+ tokens per request while
  producing no useful output.



