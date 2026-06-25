# Wave 3: Extract BYOP interfaces from server_api to infra/
# Issue: https://github.com/solla-h/warp/issues/18

/implement https://github.com/solla-h/warp/issues/18

## Context

This is a Rust project — a hard fork of Warp Terminal called "Marb" (BYOP-only, no cloud).
`ServerApiProvider` is a singleton struct that provides access to various client trait objects.
The BYOP path only needs `get_ai_client()`. We extract the minimal interface needed and delete
the rest.

Key files to read FIRST:
- `CONTEXT.md` — project context, architecture (especially "Data flow" section)
- `PLAN.md` — execution plan (you are in Wave 3)
- `app/src/server/server_api.rs` — the ServerApiProvider struct definition (~line 1547)
- `app/src/ai/blocklist/controller.rs` — how BYOP path uses ServerApiProvider
- `app/src/ai/agent_sdk/driver.rs` — another BYOP consumer

## What the BYOP path actually uses

Search for `ServerApiProvider::as_ref(ctx)` in the ai/ directory to find all usage sites.
The BYOP path needs:
- `ServerApiProvider::as_ref(ctx)` — singleton access (this is a gpui pattern)
- `.get_ai_client()` → returns `Arc<dyn AIClient>`
- `.get()` → returns `Arc<ServerApi>` (some paths use this for error reporting)
- `AIClient` trait — defines methods for cloud agent operations

## What to build

### Step 1: Audit actual usage
```
rg "ServerApiProvider" app/src/ai/ --files-with-matches
```
For each file, check WHICH methods are called. Build a minimal interface.

### Step 2: Create app/src/infra/mod.rs
- `ServiceProvider` struct — renamed from ServerApiProvider
- Implements the gpui singleton pattern (same as current ServerApiProvider)
- `get_ai_client()` → `Arc<dyn AIClient>`
- `get()` method if needed by BYOP callers
- `new()` and `new_for_test()` constructors
- `AIClient` trait definition — only the methods actually called by BYOP code

### Step 3: Rewrite BYOP imports
Replace `use crate::server::server_api::ServerApiProvider` with `use crate::infra::ServiceProvider`
Replace `use crate::server::server_api::ai::AIClient` with `use crate::infra::AIClient`

### Step 4: Delete dead server_api code
- Delete cloud-only client traits (WorkspaceClient, TeamClient, ReferralsClient, BlockClient)
- Delete `app/src/server/server_api.rs` (or server_api/ directory)
- Remove from server/mod.rs

## Critical constraint

The BYOP conversation MUST continue to work after this change. The data flow is:
```
BlocklistAIController → ServerApiProvider::as_ref(ctx).get_ai_client()
                      → AIClient methods for cloud operations
                      → (but BYOP path actually bypasses AIClient and goes through genai directly)
```

Check carefully: does the BYOP path actually USE AIClient, or does it bypass it and go through
`generate_byop_output()` directly? If it bypasses, the extraction is much simpler.

## Workflow

```
# 1. Audit: which ServerApiProvider methods does BYOP actually call?
rg "ServerApiProvider" app/src/ai/ -C 3
# 2. Create infra/ module with minimal interface
# 3. Rewrite imports in BYOP-path files
# 4. Delete dead server_api code
# 5. Verify
cargo check -p warp
warp-oss --smoke-test
git add -A && git commit -m "feat(strip): extract BYOP interfaces from server_api to infra/"
```

## Verification

- `cargo check -p warp` — 0 errors
- `warp-oss --smoke-test` — exits 0 (BYOP conversation works!)
- `app/src/infra/` exists with ServiceProvider + AIClient
- Zero `use crate::server::server_api::` references remain

## Constraints

- Do NOT break the BYOP conversation path — this is the most critical constraint
- If you're unsure whether a method is used by BYOP, KEEP IT in the extracted interface (conservative approach)
- The singleton pattern (gpui model) must be preserved exactly
- If ServerApi has internal state needed by BYOP (like auth_state), include it in ServiceProvider
- One commit: "feat(strip): extract BYOP interfaces from server_api to infra/"
