# Wave 1: Delete Crate cloud_object_client
# Issue: https://github.com/solla-h/warp/issues/8
# Copy this entire file content as your prompt in a new session with /implement

/implement https://github.com/solla-h/warp/issues/8

## Context

This is a Rust project — a hard fork of Warp Terminal called "Marb" (BYOP-only, no cloud).
We are physically deleting cloud code, not cfg-gating it.

Key files to read FIRST:
- `CONTEXT.md` — project context
- `PLAN.md` — execution plan (you are in Phase A, Wave 1)
- `Cargo.toml` (workspace root) — workspace members list
- `app/Cargo.toml` — app dependencies

## What to do

Physically delete the `cloud_object_client` crate:

1. **Find the crate**: Look in `crates/` for the directory
2. **Remove from workspace**: Delete from `[workspace] members` in root `Cargo.toml`
3. **Remove from app deps**: Delete from `[dependencies]` and `[features]` in `app/Cargo.toml`
4. **Delete the directory**: `rm -rf crates/cloud_object_client/`
5. **Fix dead imports**: Run `cargo check -p warp`, fix each error (usually just deleting `use` lines)
6. **Remove cfg blocks**: If there are `#[cfg(feature = "cloud")]` blocks that ONLY contained code using this crate, delete the entire block

### Extra notes
- This may not exist as a separate crate directory — verify first with `ls crates/ | grep cloud_object`
- If it doesn't exist as a standalone crate, check if it's a module inside another crate
- If truly non-existent, just close the issue as "not applicable"

## Workflow

```
cargo check -p warp          # see errors
# fix errors (delete dead use lines, remove dead cfg blocks)
cargo check -p warp          # repeat until 0 errors
warp-oss --smoke-test        # verify app still works (if smoke test exists)
git add -A && git commit     # one commit for this crate
```

## Verification

- `cargo check -p warp` — 0 errors
- `warp-oss --smoke-test` — exits 0 (if available)
- No references to `cloud_object_client` remain in any Cargo.toml
- The crate directory is gone

## Constraints

- Do NOT modify the BYOP code path (app/src/ai/agent_providers/)
- Do NOT add stub implementations — just delete dead code
- If a type from this crate is still needed on the BYOP path, STOP and report (this shouldn't happen for Wave 1 crates since types were already extracted to warp_types)
- One commit per crate, clean commit message: "feat(strip): delete crate cloud_object_client"
