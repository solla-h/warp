# Marb — Execution Plan

## Strategy

**Physically delete cloud code.** No cfg-gate, no feature matrix, no conditional compilation.
We don't track upstream Warp, so gating is unnecessary intermediate complexity. Delete it.

## Definition of Done

Each step must pass all three gates:

1. `cargo check -p warp` — 0 errors
2. `cargo build --release -p warp` — links successfully
3. **Smoke test passes** — `warp-oss --smoke-test` launches full app (headless), sends one
   BYOP request to the configured endpoint, receives streamed response, exits 0

"Compiles" is not enough. The app must **work**.

## Execution Order

```
Wave 0:  [#2: smoke-test]
              │
              ▼
Wave 1:  [#3:A1] [#4:A2] [#5:A3] [#6:A4] [#7:A5] [#8:A6] [#9:A7]     ← 7 parallel
              │
              ▼
Wave 2:  [#10:drive] [#11:workspaces] [#12:graphql] [#13:iap] [#14:sync_queue] [#15:cloud_objects]  ← 6 parallel
              │
              ▼
Wave 3:  [#16:ids→warp_types] [#17:telemetry stub] [#18:server_api→infra]  ← 3 parallel
              │
              ▼
Wave 4:  [#19:delete server/]  ← 1 final
```

---

## Phase C: Smoke Test Infrastructure

### What it does

`warp-oss --smoke-test` starts the app in headless mode (full singleton initialization,
no GPU window), sends a single hardcoded user message ("ping") to the configured BYOP
endpoint, waits for stream completion, then exits.

### Success criteria

- Log line: `[byop] stream stats: start=1 chunks=N` where N > 0
- Process exit code: 0

### Failure criteria

- Log line: `stream returned 0 content` → exit 1
- Timeout (30s with no stream stats) → exit 1
- Panic / crash → non-zero exit

### Implementation approach

- Add `LaunchMode::SmokeTest` variant (reuse `AppBuilder::new_headless()` path)
- After full initialization, programmatically inject one user message into BYOP path
- Wait for stream completion event, log result, exit
- Wrapper script (`scripts/smoke-test.ps1`): build release binary, run with --smoke-test,
  parse exit code

### BYOP endpoint

Real endpoint: `https://ds-api.xnurta.com/` (Anthropic format, claude model).
This is the same endpoint powering this conversation — considered reliable infrastructure.
No mock server needed.

---

## Phase A: Delete Workspace-Level Dead Crates

One commit per crate. Verify (cargo check + smoke test) after each.

### A1: warp_server_client
- Delete `crates/warp_server_client/` directory
- Remove from workspace `Cargo.toml` members
- Remove from `app/Cargo.toml` dependencies (including optional/feature refs)
- Fix dead imports (expect ~10 files)

### A2: warp_server_auth
- Delete `crates/warp_server_auth/` directory
- Remove workspace + app references
- Fix dead imports (types already extracted to warp_types in T10)

### A3: firebase_auth
- Delete `crates/firebase_auth/` directory
- Remove workspace + app references

### A4: cloud_object_persistence
- Delete `crates/cloud_object_persistence/` directory
- Remove workspace + app references

### A5: warp_graphql
- Delete `crates/warp_graphql/` directory
- Remove workspace + app references
- Fix dead imports (expect ~20 files referencing graphql types)

### A6: cloud_object_client
- Delete if exists as separate crate
- Remove workspace + app references

### A7: managed_secrets_wasm / warp_managed_secrets
- Delete both crate directories
- Remove workspace references

### Expected outcome after Phase A
- Dep tree significantly smaller
- Compile time reduced (~20-30%)
- Binary size reduced
- Zero functional regression (smoke test green)

---

## Phase B: Delete Pure-Cloud App Modules

One commit per module. Verify after each.

### Deletion layers (from CONTEXT.md analysis)

**Layer 1 — Immediately deletable (no BYOP path dependency):**

| Module | Files | Notes |
|--------|-------|-------|
| `app/src/drive/` | 125 refs | Cloud Drive sharing, zero local use |
| `app/src/workspaces/` | 192 refs | Team workspace management |
| `app/src/server/graphql/` | — | GraphQL client for Warp server |
| `app/src/server/iap/` | — | In-app purchase |
| `app/src/server/sync_queue/` | — | Cloud sync queue |
| `app/src/server/cloud_objects/` | — | Cloud object management |

**Layer 2 — Migrate then delete (BYOP path depends on interfaces):**

| Module | Refs | Action |
|--------|------|--------|
| `server/server_api` | 323 in 196 files | Extract BYOP-needed traits to `app/src/infra/` |
| `server/telemetry` | 163 in 154 files | Replace with local telemetry trait (no-op or file) |
| `server/ids` | 193 in 190 files | Rewrite imports to `warp_types::` (already extracted) |

### B1: drive/
- Delete `app/src/drive/` directory
- Remove `mod drive` from lib.rs
- Fix dead imports in ~125 files (mostly removing unused `use` lines)

### B2: workspaces/
- Delete `app/src/workspaces/` directory
- Remove `mod workspaces` from lib.rs
- Fix dead imports in ~192 files

### B3: server/graphql/ + server/iap/ + server/sync_queue/ + server/cloud_objects/
- Delete these subdirectories
- Remove their `mod` declarations from server/mod.rs
- Fix dead imports

### B4: server/ids → warp_types rewrite
- Replace all `use crate::server::ids::X` with `use warp_types::X`
- 190 files, mechanical find-and-replace
- Delete `server/ids/` module

### B5: server/telemetry → local stub
- Create `app/src/telemetry.rs` with a no-op `TelemetryEvent` enum (same variants, send() is no-op)
- Replace all `use crate::server::telemetry::` with `use crate::telemetry::`
- Delete `server/telemetry/`
- 154 files affected

### B6: server/server_api → infra extraction
- Create `app/src/infra/` with:
  - `ServiceProvider` struct (renamed from ServerApiProvider)
  - `AIClient` trait (subset needed by BYOP)
- Replace imports in BYOP-path files
- Delete remaining server_api code
- ~196 files affected (but many will already be gone from B1-B3)

### B7: Delete server/ shell
- After B3-B6, `server/` should be empty or near-empty
- Delete remaining files, remove `mod server` from lib.rs

---

## Phase B2: Architecture Clean (Future)

Not in current scope. Track as separate work after Phase B ships.

- Unify provider model: delete `CustomEndpoint`, single `AgentProvider` data model
- Split `chat_stream.rs` (7314 lines) into serializer / request_builder / stream_loop
- Add `RequestDispatcher` trait to eliminate duplicated dispatch in ResponseStream
- Split `BlocklistAIController` (3452 lines, 17 send methods)
- Add BYOP integration tests (T15 from old plan)

---

## Verification Checklist

| Gate | Command | Expected |
|------|---------|----------|
| Compile | `cargo check -p warp` | 0 errors |
| Build | `cargo build --release -p warp` | success |
| Smoke | `warp-oss --smoke-test` | exit 0 |
| Dep tree | `cargo tree -p warp \| wc -l` | decreasing after each phase |

---

## Rollback

Each deletion is one git commit. If smoke test fails after a commit:
1. `git revert HEAD` — immediate rollback to last-known-good
2. Diagnose: which import/type was still alive on the BYOP path
3. Extract that dependency first, then retry deletion

---

## What's Already Done (reference)

See `archive/MASTER_TODO.md` for full history. Key completed work:
- BYOP direct provider wired end-to-end
- All cloud modules runtime-gated (Channel::Oss)
- Core ID types extracted to warp_types (T9/T10)
- 8+ auth gate bypasses for Oss channel
- Binary compiles and runs on Windows
