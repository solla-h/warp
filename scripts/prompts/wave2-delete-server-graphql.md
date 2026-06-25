# Wave 2: Delete server/graphql/ module
# Issue: https://github.com/solla-h/warp/issues/12
# Copy this entire file content as your prompt in a new session with /implement

/implement https://github.com/solla-h/warp/issues/12

## Context

This is a Rust project — a hard fork of Warp Terminal called "Marb" (BYOP-only, no cloud).
We are physically deleting cloud modules from app/src/.

Key files to read FIRST:
- `CONTEXT.md` — project context, architecture
- `PLAN.md` — execution plan (you are in Phase B, Wave 2)
- `app/src/lib.rs` — mod declarations
- `app/src/server/mod.rs` — where the `mod graphql` declaration lives

## What to do

Physically delete `app/src/server/graphql/`:

1. **Delete the directory/file**: `rm -rf app/src/server/graphql/`
2. **Remove mod declaration**: Delete `mod graphql` from `app/src/server/mod.rs`
3. **Fix dead imports iteratively**:
   - Run `cargo check -p warp 2>&1 | head -50`
   - Fix errors (typically: delete `use crate::server::graphql::*` lines)
   - If a function/type from this module was called elsewhere, check if that caller is also dead code (only reachable from other cloud modules). If so, delete the caller too.
   - Repeat until 0 errors
4. **Handle cascading dead code**:
   - When you delete a `use` line, the imported name may have been used below. Check if the usage is in a code block that's now entirely dead (e.g., a match arm for a cloud feature). Delete the dead block.
   - If a struct field referenced a type from this module, and that struct is only used in cloud paths, delete the struct.

### Extra notes
- This is the GraphQL client for Warp's cloud API
- `GraphQLError` is referenced in `server/retry_strategies.rs` — remove that reference or simplify the retry logic
- The `cynic` crate dependency may become unused after this deletion — check and remove from app/Cargo.toml

## Workflow

```
rm -rf app/src/server/graphql/
# remove mod declaration from app/src/server/mod.rs
cargo check -p warp 2>&1 | head -50     # see first batch of errors
# fix errors
cargo check -p warp                      # repeat until 0 errors
warp-oss --smoke-test                    # verify BYOP still works
git add -A && git commit -m "feat(strip): delete graphql module"
```

## Verification

- `cargo check -p warp` — 0 errors
- `warp-oss --smoke-test` — exits 0
- No `use crate::server::graphql::` references remain
- app/src/server/graphql/ directory is gone

## Constraints

- Do NOT modify the BYOP code path (app/src/ai/agent_providers/, app/src/ai/blocklist/)
- Do NOT create stubs — just delete dead code and dead callers
- If something on the BYOP path actually USES a type/function from this module, STOP and report — it means our dependency analysis was wrong
- If the cascade goes deeper than ~50 files, take it in batches (fix 20, check, fix 20, check)
- One commit, message: "feat(strip): delete graphql module"
