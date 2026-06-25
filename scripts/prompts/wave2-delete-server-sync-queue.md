# Wave 2: Delete server/sync_queue module
# Issue: https://github.com/solla-h/warp/issues/14
# Copy this entire file content as your prompt in a new session with /implement

/implement https://github.com/solla-h/warp/issues/14

## Context

This is a Rust project — a hard fork of Warp Terminal called "Marb" (BYOP-only, no cloud).
We are physically deleting cloud modules from app/src/.

Key files to read FIRST:
- `CONTEXT.md` — project context, architecture
- `PLAN.md` — execution plan (you are in Phase B, Wave 2)
- `app/src/lib.rs` — mod declarations
- `app/src/server/mod.rs` — where the `mod sync_queue` declaration lives

## What to do

Physically delete `app/src/server/sync_queue.rs (or sync_queue/ directory)`:

1. **Delete the directory/file**: `rm -rf app/src/server/sync_queue.rs (or sync_queue/ directory)`
2. **Remove mod declaration**: Delete `mod sync_queue` from `app/src/server/mod.rs`
3. **Fix dead imports iteratively**:
   - Run `cargo check -p warp 2>&1 | head -50`
   - Fix errors (typically: delete `use crate::server::sync_queue::*` lines)
   - If a function/type from this module was called elsewhere, check if that caller is also dead code (only reachable from other cloud modules). If so, delete the caller too.
   - Repeat until 0 errors
4. **Handle cascading dead code**:
   - When you delete a `use` line, the imported name may have been used below. Check if the usage is in a code block that's now entirely dead (e.g., a match arm for a cloud feature). Delete the dead block.
   - If a struct field referenced a type from this module, and that struct is only used in cloud paths, delete the struct.

### Extra notes
- Check if it's a file (sync_queue.rs) or directory (sync_queue/) — delete whichever exists
- Also delete sync_queue_tests.rs if present
- Low reference count — should be a quick cleanup

## Workflow

```
rm -rf app/src/server/sync_queue.rs (or sync_queue/ directory)
# remove mod declaration from app/src/server/mod.rs
cargo check -p warp 2>&1 | head -50     # see first batch of errors
# fix errors
cargo check -p warp                      # repeat until 0 errors
warp-oss --smoke-test                    # verify BYOP still works
git add -A && git commit -m "feat(strip): delete sync_queue module"
```

## Verification

- `cargo check -p warp` — 0 errors
- `warp-oss --smoke-test` — exits 0
- No `use crate::server::sync_queue::` references remain
- app/src/server/sync_queue.rs (or sync_queue/ directory) directory is gone

## Constraints

- Do NOT modify the BYOP code path (app/src/ai/agent_providers/, app/src/ai/blocklist/)
- Do NOT create stubs — just delete dead code and dead callers
- If something on the BYOP path actually USES a type/function from this module, STOP and report — it means our dependency analysis was wrong
- If the cascade goes deeper than ~50 files, take it in batches (fix 20, check, fix 20, check)
- One commit, message: "feat(strip): delete sync_queue module"
