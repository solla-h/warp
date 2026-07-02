# Issue 11: Auto-trigger context compaction before overflow

## What to build

The BYOP agent sends the entire uncompacted conversation history in every request. With tool-heavy workflows (websearch results, file contents), requests easily exceed 900k tokens — far beyond model context windows. The compaction infrastructure exists but is never triggered automatically.

Wire the existing `byop_compaction::overflow::is_overflow()` check into the response stream completion handler. When token usage approaches the model's context limit, automatically enqueue a `SummarizeConversation` input to compact history before the next turn.

## Acceptance criteria

- [ ] After each successful stream completion, the controller checks whether the conversation has exceeded the compaction threshold
- [ ] When overflow is detected, a `SummarizeConversation` action is automatically enqueued (same behavior as manual `/compact`)
- [ ] The threshold should use the existing `CompactionConfig` settings (no new magic numbers)
- [ ] Requests to the LLM endpoint stay within the model's advertised context window
- [ ] The `/compact` manual command continues to work as before
- [ ] Manual test: have a multi-turn conversation with several websearch calls — verify that requests stay under the model's token limit (check logs or network)

## Blocked by

None - can start immediately. Independent of Issue 10 (different code path).
