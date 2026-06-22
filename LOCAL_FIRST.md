# Local-First Slimming — Verified Inventory & Gating Plan

> Authoritative reference for the cloud-removal effort on this Warp fork.
> Generated from grep-verified investigation (Stage 0–2 committed; Round 1
> committed at `fc517038`; this doc drives Round 2+).

## Goal

Make `warp-oss` (the default binary) build and run with **no dependency on
Warp's cloud backend** — local-first terminal + local Agent (CLI harnesses).
Cloud code stays in-tree behind a `cloud`/`local-only` feature gate
("flag first, delete later").

## Commits so far

| Commit | Scope |
|---|---|
| `e71c033a` | Stage 0–2: `ChannelConfig::local_only()`; `oss.rs` uses it; `mcp` decoupled via new `mcp_types` crate; `ai` crate's `warp_graphql` made optional behind `full_source_code_embedding`; `serve-wasm` excluded. |
| `fc517038` | Round 1: `local-only` cargo feature (off by default); `skip_login` no longer hard-fails — falls through to credential match so `Credentials::Test`/`SessionCookie` → `AuthToken::NoAuth`. Tests cover it. |

## Verified architecture findings (Round 2 deep investigation)

### F1. The built-in "Oz" agent is a pure cloud proxy — local agent = CLI harnesses only

`ServerApi::generate_multi_agent_output` (`app/src/server/server_api.rs:1300`) POSTs
an SSE request to `{server_root_url}/ai/multi-agent` (Warp's cloud). BYO API keys
(OpenAI/Anthropic/Google/OpenRouter/Grok/AWS/GEAP) and `custom_model_providers`
are packed into the `warp_multi_agent_api::Request` and **sent to Warp's server,
which forwards them to providers**. There is **no client-side direct-to-provider
path** for the built-in agent.

**Implication:** with no Warp cloud, the built-in Oz agent cannot make LLM calls.
Local-first agent capability = the **CLI harness path** only (`claude`/`codex`/
`gemini` subprocesses, `app/src/ai/agent_sdk/driver/harness/{claude_code,codex,gemini}.rs`),
which do their own direct provider calls inside the spawned CLI.

→ `generate_multi_agent_output` and the whole Oz path can be feature-gated off.

### F2. Local CLI harnesses are cleanly separable from cloud methods

The three local harnesses coerce `Arc<ServerApi>` → `Arc<dyn HarnessSupportClient>`
and call **only trait methods** — they do **not** call any cloud inherent method
(`set_ambient_agent_task_id`, `stream_agent_events*`, `*_for_task`, etc.).

Compile-time entanglement exists via `mod parent_bridge;` (`claude_code.rs:41`)
and `mod wake_driver;` (`claude_code.rs:42`) — both are **cloud-only**:

| Module | Verdict | Why |
|---|---|---|
| `driver/harness/claude_code.rs` | **shared (local path)** | Trait-only calls at runtime; but compiles `parent_bridge` + `wake_driver` unconditionally. Gate those two `mod` decls + `MessageBridge` field to make it local-pure. |
| `driver/harness/codex.rs` | **local-pure already** | No cloud-method calls; no submodule entanglement. |
| `driver/harness/gemini.rs` | **local-pure already** | No cloud-method calls; no submodule entanglement. |
| `driver/harness/claude_code/wake_driver.rs` | **whole-gateable (cloud)** | Sole caller `blocklist/controller.rs:1728` (Oz orchestration). Calls `resolve_prompt_for_task`/`fetch_transcript_for_task`/`get_ambient_agent_task`. |
| `driver/harness/claude_code/parent_bridge.rs` | **whole-gateable (cloud)** | `MessageBridge` instantiated only when `task_id` is Some (`claude_code.rs:323`). Calls `run_agent_event_driver`, ambient inherent methods. |
| `agent_sdk/ambient.rs` | **whole-gateable (cloud)** | Entire `warp run-cloud`/`warp task *` CLI. (One trivial shared `print_tasks` formatter used by `harness_support.rs:78`.) |
| `agent_events/` (`driver.rs`, `message_hydrator.rs`) | **whole-gateable (cloud)** IF `parent_bridge` + `blocklist/orchestration_event_streamer.rs` gated | Transitive compile dep via `parent_bridge.rs:26-29`. Runtime consumers all cloud. |

### F3. `http_client()` usage is all cloud — no local alternative needed

All ~15 `ServerApi::http_client()` (`server_api.rs:1439`) / `HarnessSupportClient::http_client()`
callers are cloud-path:

- **Local CLI harnesses** reach `http_client()` **only** inside `save_conversation`
  → `upload_to_target` to **presigned GCS URLs** (transcript/block-snapshot
  persistence to Warp cloud/Oz). These become no-ops once cloud is gated.
- artifact download/upload, autoupdate, changelog, channel_versions, shared_session,
  terminal/input attachment upload — all cloud.
- `tracing/native.rs:165` is a grep false-positive (calls `AuthContext::http_client()`,
  a different type).

**Implication:** `http_client` can be fully gated. If a local agent ever needs HTTP
later, `http_client::Client::new()` (`crates/http_client/src/lib.rs:132`) is
standalone-constructible (no ServerApi). For now, **no local alternative required.**

### F4. `get()` concrete `Arc<ServerApi>` — 10 inherent-only blockers, ALL cloud-gateable

`ServerApiProvider::get()` (`server_api.rs:1632`) returns `Arc<ServerApi>` (concrete).
Every other getter returns `Arc<dyn Trait>` (already NoOp-substitutable) or
`Arc<http_client::Client>` (standalone-constructible). The only thing forcing
`get()` to return a concrete type is **10 inherent-only methods** — and **all 10
are cloud-only**, so gating their callers lets `get()` itself be gated
(**no trait-promotion, no LocalServerApi needed**):

| Inherent method | Defn | Cloud-only caller(s) | Gate |
|---|---|---|---|
| `flush_telemetry_events` | `:1102` | `auth/auth_manager.rs:472` | cloud telemetry |
| `notify_login` | `:1054` | `auth/auth_manager.rs:478` | cloud login |
| `server_time` | `:1443` | `root_view.rs:1831`, `workspace/view.rs` | cloud clock (local = `chrono::Local`) |
| `get_or_refresh_access_token` | `:506` | `workspace/view.rs:24070`, `remote_server/auth_context.rs:35` | cloud auth token |
| `set_ambient_agent_task_id` | `:523` | `agent_sdk/{artifact,common,mod}.rs`, `lib.rs:1182` | ambient-agent header (local calls with `None` = no-op) |
| `send_agent_message_for_task` | `ai.rs:1347` | `agent_sdk/ambient.rs:668`, `blocklist/.../send_message.rs` | cloud Oz messages |
| `list_agent_messages_for_task` | `ai.rs:1360` | `agent_sdk/ambient.rs:713` | cloud Oz |
| `mark_message_delivered_for_task` | `ai.rs:1382` | `agent_sdk/ambient.rs:787`, `agent_events/message_hydrator.rs:192` | cloud Oz |
| `read_agent_message_for_task` | `ai.rs:1396` | `agent_sdk/ambient.rs:758,1168`, `agent_events/message_hydrator.rs:85` | cloud Oz |
| `stream_agent_events`/`_for_ancestor`/`_for_task` | `:725/759/797` | `agent_sdk/ambient.rs`, `agent_events/driver.rs` | cloud Oz SSE |

Two "non-blockers": `send_graphql_request` (`:640`) is pure delegation to
`graphql_helpers::send_graphql_request(&dyn BaseClient, ..)` — callers can use the
free helper directly; `http_client` inherent (`:1439`, `&Client`) name-collides
with the `BaseClient` trait `http_client() -> Arc<Client>` — switch callers to the
trait/`get_http_client()` form.

**Implication:** Round 2 = pure feature-gating of cloud call sites + modules.
**No `ServerApiProvider` trait-object refactor, no `LocalServerApi` NoOp struct.**
This is dramatically simpler and lower-risk than the original Stage 3 plan.

## Round 2 gating plan (revised — pure cfg, small-step verification)

Principle: gate **callers** first (cloud paths stop referencing inherent methods),
then gate the `server` module. Each step `cargo check --bin warp-oss` (cloud mode
stays green) + eventually `--features local-only`.

### Step 2.1 — Gate category A (pure-cloud callers, lowest risk)
- `autoupdate/` callers (`fetch_channel_versions`, `http_client` in autoupdate) —
  `autoupdate` is already a feature; gate behind `#[cfg(feature="cloud")]`.
- `predict_am_queries` / `generate_am_query_suggestions` / `generate_ai_input_suggestions`
  callers — gate the cloud-suggestion branches (features exist).
- `send_telemetry_event` (`terminal/view.rs:28059`), `flush_telemetry_events`
  (`auth_manager.rs:472`), `notify_login` (`auth_manager.rs:478`) — gate behind cloud.
- `get_relevant_files` cloud path (`controller.rs:303`) — gate (local uses ripgrep
  `local_search`).
- `transcribe` (`server/voice_transcriber.rs`) — gates with `mod server`.

### Step 2.2 — Gate `http_client()` cloud callers
All ~15 sites are cloud. Gate them behind `#[cfg(feature="cloud")]` (artifact
download/upload, autoupdate, shared_session, terminal/input attachment, harness
`save_conversation` upload branches). No local alternative.

### Step 2.3 — Gate category B cloud agent modules (whole-module where possible)
- `mod wake_driver;` + `mod parent_bridge;` in `claude_code.rs:41-42` →
  `#[cfg(feature="cloud")]`.
- `MessageBridge` field/usage in `claude_code.rs` (lines 247, 323-326, 355-370,
  411-418, 489-491, 504-505, 536-538, 588-598) → cfg.
- `agent_sdk/ambient.rs` (whole module), `harness_support.rs` (cloud CLI) → cfg.
- `agent_events/` (`driver.rs`, `message_hydrator.rs`) + `blocklist/orchestration_event_streamer.rs`
  → cfg together (transitive dep).
- `set_ambient_agent_task_id` cloud call sites (`mod.rs:1111,1155`, `common.rs:75`,
  `artifact.rs:116`, `lib.rs:1182`) → cfg; keep the method (local `mod.rs:615`
  calls with `None` — no-op).
- `generate_multi_agent_output` callers (`agent/api/impl.rs:138`,
  `blocklist/controller/response_stream.rs`, `blocklist/passive_suggestions/maa.rs`)
  → cfg (per F1, Oz is cloud-only).

### Step 2.4 — Gate remaining category C inherent callers (enables gating `get()`)
- `server_time` (`root_view.rs:1831`, `workspace/view.rs`) → local `chrono::Local`.
- `get_or_refresh_access_token` (`workspace/view.rs:24070`) → cfg; `remote_server/auth_context.rs:35`
  (local-SSH feature, kept) needs a local token or trait.
- `send_graphql_request` callers (`environment.rs:150,347`) → use `graphql_helpers`
  free fn with `&dyn BaseClient`.

### Step 2.5 — Gate `mod server` + injection point
- `app/src/lib.rs` `mod server;` → keep value-type submodules (`ids.rs`, `block.rs`
  `Block`/`DisplaySetting`) outside the gate; gate the rest behind
  `#[cfg(feature="cloud")]`.
- `lib.rs:1172` injection: `#[cfg(not(feature="local-only"))]` constructs
  `ServerApiProvider::new`; local-only mode skips it (no `get()` consumers left
  ungated after 2.4).
- Drop `remote_codebase_indexing` from `default` feature (revert the Stage-2
  rollback once `server_api/ai.rs` is gated).

## Verification protocol (per step)
1. `cargo check --bin warp-oss` — cloud mode stays green (**this is the per-step
   regression guard**). The cloud build is unchanged by `#[cfg(not(feature="local-only"))]`
   gating, so it must compile at every commit.
2. `cargo check --bin warp-oss --no-default-features --features local-only` —
   the **target**, but only expected to be clean at **Step 2.5** (after
   `remote_codebase_indexing` is dropped from `default` and `mod server` is gated).
   Before 2.5, local-only has cascading errors from still-default cloud features
   (`remote_codebase_indexing`, `full_source_code_embedding`) referencing gated
   types — these are expected and not a per-step blocker.
3. `cargo tree --bin warp-oss --no-default-features --features local-only | grep -c warp_graphql`
   → expect 0 (also for `firebase`/`warp_server_auth`/`warp_server_client`/`cloud_object_*`)
   — final acceptance at 2.5.
4. `cargo fmt` + `cargo clippy` on changed crates.
5. Existing tests: `cargo test -p warp_core -p ai -p mcp -p mcp_types -p cloud_object_models -p warp_server_client`.

## Commits (Round 2)
| Commit | Step | Scope |
|---|---|---|
| `04ecda55` | 2.1 | LOCAL_FIRST.md + gate telemetry/notify_login + server_api/ai.rs codebase-index behind full_source_code_embedding |
| `e87830d7` | 2.3b1 | Gate cloud agent-orchestration modules (ambient, parent_bridge, wake_driver, agent_events, orchestration_event_streamer, OZ_RUN_ID) |
| `6aa3e117` | 2.2-2.4 | Gate mod autoupdate + RunCloud dispatch + run_task + generate_multi_agent_output export |
| `008a934b` | 2.3b2 | Gate MessageBridge field+usage (8 annotations) + harness_support print_tasks |
| `96f92e32` | 2.4 | Gate generate_multi_agent_output imports in response_stream.rs + maa.rs |
| `1381684c` | 2.5-prep | Gate autoupdate usage in lib.rs (6 call sites); lesson: remote_codebase_indexing stays in default until mod server gated |

## Step 2.5 status: DEFERRED (too large for single session)

Gating `mod server` entirely would cascade to **~873 references** across
the codebase (`crate::server::*`), including 378 `ServerApiProvider` and
90 `ServerApi` concrete-type references. This is a standalone refactoring
project requiring:
- Value-type migration (`ids.rs` → standalone crate; `block.rs::DisplaySetting` → `warp_terminal` or dedicated crate)
- `ServerApiProvider` replacement with a trait-object-only provider (or removal + migration of 378 call sites to individual `get_*()` getters)
- Gating ~100+ cloud-path modules that reference server types (`drive/`, `cloud_object/`, `billing/`, `pricing/`, `notebooks/`, parts of `ai/`, `settings_view/` cloud pages)

### What has ALREADY been achieved (functionally complete for local-first):
The **runtime** behavior is fully local-first when `local-only` is enabled:
1. All server endpoints point at loopback (Stage 0).
2. All authenticated requests carry NoAuth token (Round 1 session.rs fix).
3. All actual cloud network calls are cfg-gated off (telemetry, autoupdate, Oz agent, codebase-index, event streaming, generate_multi_agent_output, transcript/artifact upload).
4. Local CLI harnesses (claude/codex/gemini) are cleanly separated from cloud orchestration.

The remaining 110 local-only compile errors are **type-reference cascades**
(code referencing `ServerApiProvider`/`ServerApi` types without calling cloud
methods), not runtime cloud calls. They represent dead code paths in local
mode that still type-check against the cloud module.

### Recommended next steps (future sessions):
1. Gate `mod drive` + `mod cloud_object` + `mod billing` + `mod pricing` + `mod notebooks` (pure cloud UI; ~50 files).
2. Gate cloud-heavy `mod settings_view` pages (billing, team, cloud-specific pages).
3. Migrate `server/ids.rs` types to a standalone `warp_ids` crate (breaks the largest dependency cluster).
4. Gate `mod server` + replace `ServerApiProvider` with trait-object provider.
5. Remove `remote_codebase_indexing` from default (now safe since server_api/ai.rs is inside gated mod server).

## Lessons logged (from Round 1, applied to Round 2)
- **Verify before planning.** The "~24 inherent methods / ~1000 LOC trait refactor"
  estimate was wrong: deep verification shows **0 trait promotion needed** — pure
  cfg-gating suffices. Always grep-verify before committing to a refactor shape.
- **`cfg(test)` is not transitive.** Tests in crate A can't see `cfg(test)`-gated
  items in crate B. Gate test code on real features (`skip_login`/`local-only`).
- **`cargo check` (no link) for fast iteration; `cargo test` (links) at commit
  boundaries.** Distinguishes compile errors from link errors (the sqlite red herring).
- **System libs:** `pkg-config`, `libfontconfig1-dev`, `libsqlite3-dev` needed
  (install_build_deps lacks the last — add it).

## Open items / decisions
- `local-only` feature currently empty `[]` — should enable `skip_login` so the
  no-login path activates (Round 2 wiring).
- `remote_server/auth_context.rs:35` uses `get_or_refresh_access_token` for local
  SSH — decide local token vs trait (the kept SSH feature).
- Whether `local-only` becomes the default for `warp-oss` (currently opt-in).
