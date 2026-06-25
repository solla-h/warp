# Wave 3: Replace server/telemetry with local no-op stub
# Issue: https://github.com/solla-h/warp/issues/17

/implement https://github.com/solla-h/warp/issues/17

## Context

This is a Rust project — a hard fork of Warp Terminal called "Marb" (BYOP-only, no cloud).
`TelemetryEvent` is a large enum (1268+ lines, hundreds of variants) referenced by ~154 files.
In Marb, telemetry events go nowhere — but the call sites must still compile. We create a
compile-compatible no-op stub that replaces the cloud telemetry module.

Key files to read FIRST:
- `CONTEXT.md` — project context
- `PLAN.md` — execution plan (you are in Wave 3)
- `app/src/server/telemetry/events.rs` — the TelemetryEvent enum definition (READ THIS CAREFULLY)
- `app/src/server/telemetry/mod.rs` — module structure, public exports
- `app/src/server/telemetry/macros.rs` — any macros used by call sites

## Worktree Isolation (Parallel Execution)

**CRITICAL**: This issue is being worked on IN PARALLEL with other issues in the same wave.
Each agent works in its own git worktree — a physically separate directory with its own branch.

### Before you start — VERIFY your environment:

```
# 1. Confirm you are in the correct worktree directory:
pwd
# Expected: a path ending in `wave3-telemetry-stub` (NOT the main warp repo)

# 2. Confirm you are on the correct branch:
git branch --show-current
# Expected: wave3/telemetry-stub

# 3. Confirm the worktree is clean:
git status
# Expected: nothing to commit, working tree clean
```

**If ANY of these checks fail, STOP IMMEDIATELY. Do NOT proceed.** You may be in the wrong
worktree or the wrong branch. Ask the user to verify your working directory.

### Rules:
- You are ALREADY in the correct worktree and branch (the user set this up before pasting this prompt)
- Do all work in THIS directory — do NOT cd to other directories or switch branches
- You CAN and SHOULD run `cargo check -p warp` — it will pass because you only deleted your module (other modules still exist on your branch)
- Do NOT run `warp-oss --smoke-test` — that will be run AFTER all wave branches are merged in the main repo
- After your work is done, commit on your branch and STOP. The user will merge branches sequentially in the main repo.
- Do NOT run `git worktree` commands — the user manages worktree lifecycle

## What to build

A local no-op telemetry module that is API-compatible with the existing server/telemetry:

### Step 1: Understand the current API surface
Read `server/telemetry/` to understand:
- All public types (TelemetryEvent enum, helper enums, source enums)
- The `send()` / `emit()` / `track()` methods and their signatures
- Any macros that call sites use
- Any traits or helper functions exported

### Step 2: Create the stub
Create `app/src/telemetry/mod.rs` (or `telemetry.rs`) with:
- Same `TelemetryEvent` enum — ALL variants preserved (copy the enum definition)
- Same helper types that appear in variant fields
- `send()` → no-op (empty body, just drop self)
- Any macros → no-op versions
- Preserve ALL public type signatures so callers compile without changes

### Step 3: Rewrite imports
Replace all `use crate::server::telemetry::` with `use crate::telemetry::`

### Step 4: Delete and rewire
- Delete `app/src/server/telemetry/` directory
- Remove `mod telemetry` from `server/mod.rs`
- Add `mod telemetry` to `app/src/lib.rs` (or appropriate parent)

## Key insight

The stub does NOT need to actually send telemetry. Every method body can be `{}`.
The goal is compile compatibility, not runtime behavior.

## Workflow

```
# 1. Read the current telemetry module API
# 2. Create app/src/telemetry/ with no-op stubs
# 3. Add `mod telemetry;` to lib.rs
# 4. Replace imports in all files:
rg "use crate::server::telemetry" app/src/ --files-with-matches
# 5. Delete server/telemetry/
# 6. Verify
cargo check -p warp
# NOTE: smoke test will be run after all wave branches are merged
git add -A && git commit -m "feat(strip): replace server/telemetry with local no-op stub"
```

## Verification

- `cargo check -p warp` — 0 errors
- [ ] Branch committed and ready for merge (smoke test runs post-merge)
- Zero `use crate::server::telemetry::` references remain
- `app/src/server/telemetry/` deleted
- `app/src/telemetry/` exists with no-op implementation

## Constraints

- Preserve ALL enum variants — do NOT simplify the enum (callers construct specific variants)
- Preserve ALL public type signatures — callers must compile without ANY code changes beyond import path
- No-op bodies only — do not add logging, file writing, or any side effects
- If a helper type is too complex to stub (e.g., depends on types from deleted modules), replace its fields with `()` or remove the field from the variant using `#[allow(dead_code)]`
- One commit: "feat(strip): replace server/telemetry with local no-op stub"
