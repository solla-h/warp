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
warp-oss --smoke-test        # verify app still works (if smoke test exists)
git add -A && git commit     # one commit for this crate
```

## Verification

- `cargo check -p warp` — 0 errors
- `warp-oss --smoke-test` — exits 0 (if available)
- No references to `managed_secrets_wasm` or `warp_managed_secrets` remain in any Cargo.toml
- The crate directories are gone

## Constraints

- Do NOT modify the BYOP code path (app/src/ai/agent_providers/)
- Do NOT add stub implementations — just delete dead code
- If a type from this crate is still needed on the BYOP path, STOP and report (this shouldn't happen for Wave 1 crates since types were already extracted to warp_types)
- One commit per crate, clean commit message: "feat(strip): delete crates managed_secrets_wasm + warp_managed_secrets"
