# Wave 1: Delete Crate managed_secrets_wasm and warp_managed_secrets
# Issue: https://github.com/solla-h/warp/issues/9
# Copy this entire file content as your prompt in a new session with /implement

/implement https://github.com/solla-h/warp/issues/9

## Context

This is a Rust project — a hard fork of Warp Terminal called "Marb" (BYOP-only, no cloud).
We are physically deleting cloud code, not cfg-gating it.

Key files to read FIRST:
- `CONTEXT.md` — project context
- `PLAN.md` — execution plan (you are in Phase A, Wave 1)
- `Cargo.toml` (workspace root) — workspace members list
- `app/Cargo.toml` — app dependencies

## Worktree Isolation (Parallel Execution)

**CRITICAL**: This issue is being worked on IN PARALLEL with other issues in the same wave.
Each agent works in its own git worktree — a physically separate directory with its own branch.

### Before you start — VERIFY your environment:

```
# 1. Confirm you are in the correct worktree directory:
pwd
# Expected: a path ending in `wave1-managed-secrets` (NOT the main warp repo)

# 2. Confirm you are on the correct branch:
git branch --show-current
# Expected: wave1/delete-managed-secrets

# 3. Confirm the worktree is clean:
git status
# Expected: nothing to commit, working tree clean
```

**If ANY of these checks fail, STOP IMMEDIATELY. Do NOT proceed.** You may be in the wrong
worktree or the wrong branch. Ask the user to verify your working directory.

### Rules:
- You are ALREADY in the correct worktree and branch (the user set this up before pasting this prompt)
- Do all work in THIS directory — do NOT cd to other directories or switch branches
- You CAN and SHOULD run `cargo check -p warp` — it will pass because you only deleted your crate (other crates still exist on your branch)
- Do NOT run `warp-oss --smoke-test` — that will be run AFTER all wave branches are merged in the main repo
- After your work is done, commit on your branch and STOP. The user will merge branches sequentially in the main repo.
- Do NOT run `git worktree` commands — the user manages worktree lifecycle

## What to do

Physically delete the `managed_secrets_wasm` and `warp_managed_secrets` crates:

1. **Find the crate**: Look in `crates/` for the directory
2. **Remove from workspace**: Delete from `[workspace] members` in root `Cargo.toml`
3. **Remove from app deps**: Delete from `[dependencies]` and `[features]` in `app/Cargo.toml`
4. **Delete the directory**: `rm -rf crates/managed_secrets_wasm/ crates/warp_managed_secrets/`
5. **Fix dead imports**: Run `cargo check -p warp`, fix each error (usually just deleting `use` lines)
6. **Remove cfg blocks**: If there are `#[cfg(feature = "cloud")]` blocks that ONLY contained code using this crate, delete the entire block

### Extra notes
- Delete BOTH crates: `managed_secrets_wasm` (WASM build) and `warp_managed_secrets` (native)
- They are a wasm/native pair for Warp's managed secrets service
- Check both `crates/managed_secrets_wasm/` and `crates/warp_managed_secrets/`
- In step 4, delete both directories
- The commit message should be: "feat(strip): delete crates managed_secrets_wasm + warp_managed_secrets"

## Workflow

```
cargo check -p warp          # see errors
# fix errors (delete dead use lines, remove dead cfg blocks)
cargo check -p warp          # repeat until 0 errors
# NOTE: smoke test will be run after all wave branches are merged
git add -A && git commit     # one commit for this crate
```

## Verification

- `cargo check -p warp` — 0 errors
- [ ] Branch committed and ready for merge (smoke test runs post-merge)
- No references to `managed_secrets_wasm` or `warp_managed_secrets` remain in any Cargo.toml
- The crate directories are gone

## Constraints

- Do NOT modify the BYOP code path (app/src/ai/agent_providers/)
- Do NOT add stub implementations — just delete dead code
- If a type from this crate is still needed on the BYOP path, STOP and report (this shouldn't happen for Wave 1 crates since types were already extracted to warp_types)
- One commit per crate, clean commit message: "feat(strip): delete crates managed_secrets_wasm + warp_managed_secrets"
