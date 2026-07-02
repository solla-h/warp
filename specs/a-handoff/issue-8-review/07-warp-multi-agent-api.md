# Requirement 7: Remove `warp_multi_agent_api`

## Verdict: Cannot Remove

## Reason summary

`warp_multi_agent_api` is a protobuf-generated crate (from `warpdotdev/warp-proto-apis`)
that defines the entire wire protocol for the multi-agent system. It is imported by
**91 source files** across **4 workspace crates** with **251 total use-sites**. It
provides core request/response types, tool-call enums, streaming event types, and
conversation serialization types. Removing it would break the agent communication
stack entirely.

## What this crate provides

External git dependency:
```
git = "https://github.com/warpdotdev/warp-proto-apis.git"
rev = "97d1b367b955c562812e0a1315a6ec7ee6a5389e"
```

Key type families it generates:

- **Request/Response**: `Request`, `ResponseEvent`, `ConversationData`
- **Streaming events**: `response_event::StreamInit`, `StreamFinished`, `TokenUsage`, `ClientActions`
- **Finish reasons**: `ContextWindowExceeded`, `QuotaLimit`, `InvalidApiKey`, `MaxTokenLimit`, etc.
- **Messages**: `Message`, `Task`, `message::UserQuery`, `message::ToolCall`, `message::SystemQuery`
- **Tool types**: `ToolType` enum (ApplyFileDiffs, SuggestPrompt, etc.)
- **Agent config**: `AutonomyLevel`, `IsolationLevel`, `AgentType`, `UserQueryMode`
- **File/diff**: `FileContent`, `AnyFileContent`, `BinaryFileContent`, `diff_set::DiffHunk`, `FileContentLineRange`
- **Git refs**: `CurrentRef`, `BaseRef`, `base_ref::Ref::*`, `current_ref::Ref::*`
- **Tool results**: `ask_user_question_result`, `apply_file_diffs_result`, `run_shell_command_result`
- **Settings**: `request::settings::ApiKeys`, `LlmProvider`
- **Actions**: `ClientAction`, `client_action::Action`
- **Skills**: `skill_descriptor::SkillReference`
- **Lifecycle**: `LifecycleEventType`, `ShellCommandFinished`

## Source files using warp_multi_agent_api (91 files total)

### app/src/ai/agent/api/ (core protocol layer — 8 files)

- `api.rs` — Struct definitions for `RequestParams` and `ConversationData` use `Task`, `ApiKeys`, `AutonomyLevel`, `IsolationLevel`, `ToolType`, `ResponseEvent`
- `convert_conversation.rs` — Restores persisted MAA tasks back into UI exchange state; uses `api::*` alias throughout, plus `ask_user_question_result`
- `convert_from.rs` — Converts API message types into client-side `AIAgentOutputMessage`
- `convert_to.rs` — Converts client types (inputs, attachments, contexts) into API `Request` subtypes; converts `UserQueryMode`, tool call results
- `impl.rs` — Implements API request building/encoding logic
- `*_tests.rs` — 4 test files constructing API types for assertions

### app/src/ai/agent/ (agent domain — 10+ files)

- `conversation.rs` — Core conversation model; uses `stream_finished`, `TokenUsage`, `StreamInit`, `client_action::Action`; `compute_active_tasks()` returns `Vec<Task>`
- `mod.rs` — `From` impls: `CurrentHead → CurrentRef`, `DiffBase → BaseRef`, `DiffHunk::convert_to_api()`; imports `AgentEvent`, `AgentType`
- `linearization.rs` — Uses `api` alias for linearization logic
- `conversation_yaml.rs` — YAML serialization using `api` types
- `task.rs`, `task/helper.rs`, `task_store.rs` — Task management using `api::Task`

### app/src/ai/agent_providers/ (BYOP chat stream — 19 files)

- `chat_stream.rs` — Translates `RequestParams` → genai `ChatRequest`, responses back to `ResponseEvent`
- `tools/*.rs` (14 files) — Each tool (ask, documents, edit, files, shell, mcp, search, skill, suggest, todowrite, webfetch, websearch, long_shell, markers) uses `api` alias to construct tool-call response events
- `user_context.rs` — Builds user context from API types
- `cache_stability_tests.rs` — Tests cache key stability with API types

### app/src/ai/blocklist/ (UI layer — 12 files)

- `controller.rs`, `controller/response_stream.rs`, `controller/shared_session.rs` — Stream orchestration
- `action_model/execute/fetch_conversation.rs` — Fetches conversation, converts tasks
- `orchestration_event_streamer.rs`, `orchestration_events.rs` — Multi-agent orchestration events
- `passive_suggestions/maa.rs` — MAA passive suggestion handling
- `history_model.rs`, `history_model/conversation_loader.rs` — History loading from persisted tasks
- `block/status_bar.rs` — Status bar displaying agent state
- Various `*_tests.rs` files

### app/src/ai/ (other subsystems — 9 files)

- `byop_compaction/` (commit.rs, message_view.rs, state.rs, tests.rs) — Conversation compaction using `api::Message`
- `artifacts/mod.rs` — Artifact handling
- `document/ai_document_model.rs` — Document pane model
- `conversation_details_panel_tests.rs`

### app/src/ (non-AI subsystems — 12 files)

- `infra/server_api.rs`, `infra/server_api/ai.rs` — Server API client for protobuf encode/decode
- `persistence/agent.rs`, `persistence/mod.rs` — SQLite persistence of conversation data
- `pane_group/child_agent/hydration.rs` — Child agent pane hydration
- `terminal/shared_session/ai_agent.rs`, `terminal/shared_session/replay_agent_conversations.rs`
- `terminal/view/load_ai_conversation.rs` — Loading conversations from disk
- `test_util/ai_agent_tasks.rs` — Test utilities
- `integration_testing/agent_mode/` (assertions.rs, llm_judge/mod.rs, step.rs)

## Crate-level consumers

### crates/ai/ (9 files)

Cargo.toml: `warp_multi_agent_api.workspace = true`

Used for:
- `agent/action/convert.rs` — Converts API tool-call messages into domain `Action` types; `From<FileContent>`, `From<AnyFileContent>`
- `agent/action/mod.rs` — Re-exports `LifecycleEventType`
- `agent/action_result/convert.rs` — Converts domain results back to API result types (`UpdatedFileContent`, answer items)
- `agent/action_result/mod.rs` — Builds `Vec<FileContent>` from domain results
- `agent/citation.rs` — Citation handling using API types
- `agent/orchestration_config.rs` — Orchestration config using API types
- `api_keys.rs`, `aws_credentials.rs`, `geap_credentials.rs` — Credential type conversions
- `skills/conversion.rs`, `skills/skill_reference.rs` — Skill descriptor conversions

### crates/integration/ (1 file)

Cargo.toml: `warp_multi_agent_api.workspace = true`

Used for:
- `src/test/agent_mode.rs` — Integration test harness that constructs API types for end-to-end agent tests

### crates/persistence/ (2 files)

Cargo.toml: `warp_multi_agent_api.workspace = true`

Used for:
- `src/model.rs` — Database model types that reference `stream_finished` and `api` for token usage and conversation metadata persistence
- `src/model_tests.rs` — Tests for persistence model

## Cargo.toml references

| File | Declaration |
|------|------------|
| `Cargo.toml` (workspace root) | `warp_multi_agent_api = { git = "https://github.com/warpdotdev/warp-proto-apis.git", rev = "97d1b367..." }` |
| `app/Cargo.toml` | `warp_multi_agent_api.workspace = true` |
| `crates/ai/Cargo.toml` | `warp_multi_agent_api.workspace = true` |
| `crates/integration/Cargo.toml` | `warp_multi_agent_api.workspace = true` |
| `crates/persistence/Cargo.toml` | `warp_multi_agent_api.workspace = true` |

## Architecture role

`warp_multi_agent_api` is the **protocol definition layer** for the entire agent stack.
It defines the protobuf schema that governs:

1. **Request serialization** — How client inputs (user queries, tool call results, context)
   are encoded into `Request` protobufs for the server (or in BYOP mode, used as the
   internal canonical representation before translation to provider-specific formats).

2. **Response deserialization** — How streaming `ResponseEvent` protobufs (init, messages,
   tool calls, finish signals) are decoded and routed through the conversation controller.

3. **Conversation persistence** — `Task` and `Message` types are serialized to SQLite for
   conversation history, and deserialized back when loading/restoring conversations.

4. **Cross-layer contract** — The `crates/ai` domain layer, `crates/persistence` storage
   layer, and `app` UI/controller layer all share these types as the common vocabulary.

In BYOP mode (the current fork's focus), `chat_stream.rs` translates TO/FROM these
protocol types at the boundary, so the entire internal pipeline still speaks the same
protobuf language even though no server is involved.

## What would need to happen to eventually remove it

Removing `warp_multi_agent_api` would require:

1. **Define local replacement types** — Reimplement ~80 protobuf-generated types as native
   Rust structs/enums in a local crate (e.g., `crates/agent-protocol/`). This includes
   deeply nested oneofs like `message::Message`, `response_event::Type`, and all tool
   result variants.

2. **Remove prost/protobuf dependency** — The current types use prost for encode/decode.
   Local types would need serde-based serialization for persistence (already partially
   done in BYOP mode) and drop the binary protobuf wire format.

3. **Rewrite 91 files** — Every file listed above would need import path changes and
   potentially structural adjustments where protobuf patterns (Optional fields, oneof
   enums) differ from idiomatic Rust.

4. **Maintain API compatibility** — If any server communication remains (even optional),
   a conversion layer between local types and the proto types would be needed.

5. **Estimated effort** — 2-4 weeks of focused work given the breadth (251 use-sites
   across 91 files in 4 crates), plus risk of subtle serialization bugs in the
   persistence layer where existing conversations are stored in proto format.

This is not viable as a quick cleanup and is only worth pursuing if the project fully
commits to never communicating with the Warp server again AND migrates the persistence
format.

