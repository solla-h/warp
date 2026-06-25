# Marb — Project Context

## What Is This

This is a **hard fork** of [Warp Terminal](https://github.com/warpdotdev/warp) (open-sourced 2025),
rebranded as **Marb**. The goal is a fully standalone AI-powered terminal that runs **entirely on
BYOP (Bring Your Own Provider)** — no Warp cloud dependency, no account required.

We do **not** track upstream Warp. Cherry-picking useful changes is fine, but rebase/merge with
upstream is not a goal.

## Current State (as of 2026-06-25)

### Branch layout

| Branch | Purpose |
|--------|---------|
| `warp-slim-to-marb` | Working branch with all prior cfg-gate work + BYOP fixes |
| `marb/strip-cloud` | **Active branch** — physically removing cloud code |

### What's already done (see `archive/MASTER_TODO.md` for details)

- BYOP direct provider wired through genai + agent_providers + response_stream
- Settings UI bridge: custom_endpoints → BYOP runtime path
- All cloud modules runtime-gated (Channel::Oss check)
- firebase, cloud_object_persistence, warp_server_auth, warp_server_client → optional features
- warp_graphql → optional
- AWS SDK, OpenTelemetry → optional features
- Binary compiles and runs on Windows (Intel Arc GPU, DX12)
- Agent conversation works end-to-end with BYOP endpoint

### What's broken / incomplete

- `cloud_objects` crate still linked (102 file references, holds shared ID types)
- Dead cloud code is cfg-gated but still physically present (~large cognitive noise)
- Two parallel provider data models: `CustomEndpoint` (legacy) and `AgentProvider` (new)
- `chat_stream.rs` is 7314 lines with 7 responsibilities (god file)
- Controller has 17 `send_*` method variants
- No automated tests or smoke test

## Architecture (BYOP path only)

### Data flow: User types → Agent replies

```
User input
  → BlocklistAIController.send_user_query_in_conversation()
    → ResponseStream::new()
      → byop_dispatch_info() — resolves LLMId → (AgentProvider, api_key, model_id)
        → lookup_byop() — tries byop: prefix decode, then legacy CustomEndpoint UUID fallback
      → generate_byop_output(ByopOutputInput)
        → build_chat_request() — serializes history + tools + system prompt
        → build_chat_options() — reasoning effort, headers, context window
        → genai::Client.exec_chat_stream() — HTTP POST to user's endpoint
        → stream event loop — emits chunks to UI
```

### Key modules

| Module | Location | Responsibility |
|--------|----------|---------------|
| agent_providers | `app/src/ai/agent_providers/` | BYOP execution: request building, streaming, reasoning |
| blocklist controller | `app/src/ai/blocklist/controller/` | Request lifecycle, retry, resume, dispatch |
| response_stream | `app/src/ai/blocklist/controller/response_stream.rs` | Per-stream state machine: new → retry → finish |
| custom_inference_modal | `app/src/settings_view/custom_inference_modal.rs` | Endpoint config UI |
| ApiKeyManager | `crates/ai/src/api_keys.rs` | Legacy endpoint storage (CustomEndpoint) |
| AISettings | `app/src/settings/ai.rs` | New-style provider storage (AgentProvider) |
| genai (forked) | `lib/rust-genai/` | LLM SDK — adapters for Anthropic/OpenAI/Gemini/etc |

### Known architectural debt

1. **Dual provider model**: `CustomEndpoint` (stringly-typed api_type, no stable ID, no capabilities)
   coexists with `AgentProvider` (typed enum, stable ID, rich capabilities). Bridged by
   `custom_endpoints_as_providers()` which uses fragile positional-index IDs.

2. **God file**: `chat_stream.rs` (7314 lines) handles serialization, request building, streaming,
   caching, repair, and diagnostics. Should be split into serializer / request_builder / stream_loop.

3. **Duplicated dispatch**: `ResponseStream::new()` and `retry()` copy-paste the BYOP/cloud
   branching logic. Needs a `RequestDispatcher` trait.

4. **God controller**: `BlocklistAIController` (3452 lines, 17 public send methods). Auto-resume
   scheduling mixes UI modal state with transport recovery.

## Execution Plan

See `PLAN.md` for the full execution plan (physical deletion strategy, smoke test design,
phase ordering, and verification gates).

## BYOP Endpoint Configuration

- Endpoint URL: `https://ds-api.xnurta.com/`
- API Type: **Anthropic**
- Model: `claude-opus-4-6`
- Reasoning Effort: **Auto** (default — does not inject thinking parameters)

## Design Principles for Marb

1. **BYOP-only**: No fallback to any cloud service. If provider is misconfigured, show a clear
   error and stop — don't retry uselessly.
2. **Single data model**: One way to represent a provider endpoint. No legacy/new duality.
3. **Deep modules**: Few public methods, lots of implementation hidden. `chat_stream.rs` should
   be three files, not one.
4. **No cargo-cult from Zap/Warp**: Reference good patterns, but never copy architecture wholesale.
   If something is "too rough" in the original, redesign it.
5. **Compilation speed matters**: Dead code = dead weight. Remove it, don't gate it.
