# Issue 10: Fix agentic loop — tool results never appended to next request

## What to build

The BYOP agentic loop silently breaks after any intercepted tool execution (websearch, webfetch, todowrite, etc.). The model never receives tool results, loops 8 times with identical requests, then terminates.

The root cause is in the chat stream driver: `saved_stream_end` is declared as `None` and never assigned a value when `ChatStreamEvent::End(end)` fires. The subsequent loop iteration checks `saved_stream_end.take()`, gets `None`, and skips appending the assistant message + tool responses to the request.

The fix: store the stream end event when it arrives, so the loop can extract the assistant's content (including tool_use blocks) and the intercepted tool responses for the next iteration.

## Acceptance criteria

- [ ] After a websearch/webfetch tool executes, the agent continues analyzing the results and produces a follow-up response
- [ ] Tool results are correctly appended to `chat_req.messages` as `ChatMessage::from(tool_response)` before the next iteration
- [ ] The agentic loop correctly iterates (up to 8 times) when multiple sequential tool calls are needed
- [ ] Single-tool-call conversations (no loop needed) still work correctly
- [ ] Manual test: ask the agent to "search the web for Rust 2024 edition changes and summarize" — it should execute websearch, then produce a summary

## Blocked by

None - can start immediately. This is the highest-priority fix.
