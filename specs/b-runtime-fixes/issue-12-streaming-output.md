# Issue 12: Show thinking and tool_use progress during agent execution

## What to build

The agent's reasoning process and tool invocations are invisible to users during execution. The CLI/SDK output path only writes data when an exchange is fully completed — users see nothing until the final response appears.

Add incremental output during streaming so users can see:
1. Thinking/reasoning tokens as they arrive
2. Tool invocation names when a tool_use block starts
3. Tool execution progress (started → completed)

The GUI terminal view already receives these events via protobuf streaming — this issue is about the output rendering path that the user actually sees.

## Acceptance criteria

- [ ] When the model emits reasoning/thinking tokens, they are displayed incrementally (not buffered until completion)
- [ ] When a tool_use block starts, the tool name is shown immediately (e.g., "Using websearch...")
- [ ] When a tool execution completes, its result status is shown before the next model turn begins
- [ ] The final response still renders correctly (no duplicate content)
- [ ] Non-streaming scenarios (instant responses with no tools) are unaffected
- [ ] Manual test: ask the agent a question requiring websearch — verify you see "thinking..." and "websearch" activity before the final answer

## Blocked by

None - can start immediately. Independent of Issues 10 and 11 (different code path: output rendering, not loop logic or context management).
