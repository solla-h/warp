# Wave 2: Delete App Module Template
# This is a TEMPLATE. Replace {MODULE}, {ISSUE_URL}, {MOD_PATH}, {MOD_PARENT}, {REF_COUNT}, {EXTRA_NOTES}
# Wave 2 modules (all parallel):
#   - #10: drive/ — {MOD_PATH: app/src/drive/, MOD_PARENT: lib.rs, REF_COUNT: ~125}
#   - #11: workspaces/ — {MOD_PATH: app/src/workspaces/, MOD_PARENT: lib.rs, REF_COUNT: ~192}
#   - #12: server/graphql/ — {MOD_PATH: app/src/server/graphql/, MOD_PARENT: server/mod.rs, REF_COUNT: moderate}
#   - #13: server/iap/ — {MOD_PATH: app/src/server/iap/, MOD_PARENT: server/mod.rs, REF_COUNT: low}
#   - #14: server/sync_queue/ — {MOD_PATH: app/src/server/sync_queue.rs, MOD_PARENT: server/mod.rs, REF_COUNT: low}
#   - #15: server/cloud_objects/ — {MOD_PATH: app/src/server/cloud_objects/, MOD_PARENT: server/mod.rs, REF_COUNT: ~110}

/implement {ISSUE_URL}

## Context

This is a Rust project — a hard fork of Warp Terminal called "Marb" (BYOP-only, no cloud).
We are physically deleting cloud modules from app/src/.

Key files to read FIRST:
- `CONTEXT.md` — project context, architecture
- `PLAN.md` — execution plan (you are in Phase B, Wave 2)
- `app/src/lib.rs` — mod declarations
- `{MOD_PARENT}` — where the `mod {MODULE}` declaration lives

## What to do

Physically delete `{MOD_PATH}`:

1. **Delete the directory/file**: `rm -rf {MOD_PATH}`
2. **Remove mod declaration**: Delete `mod {MODULE}` from `{MOD_PARENT}`
3. **Fix dead imports iteratively**:
   - Run `cargo check -p warp 2>&1 | head -50`
   - Fix errors (typically: delete `use crate::{MODULE}::*` lines)
   - If a function/type from this module was called elsewhere, check if that caller is also dead code (only reachable from other cloud modules). If so, delete the caller too.
   - Repeat until 0 errors
4. **Handle cascading dead code**:
   - When you delete a `use` line, the imported name may have been used below. Check if the usage is in a code block that's now entirely dead (e.g., a match arm for a cloud feature). Delete the dead block.
   - If a struct field referenced a type from this module, and that struct is only used in cloud paths, delete the struct.

{EXTRA_NOTES}

## Workflow

```
rm -rf {MOD_PATH}
# remove mod declaration from {MOD_PARENT}
cargo check -p warp 2>&1 | head -50     # see first batch of errors
# fix errors
cargo check -p warp                      # repeat until 0 errors
warp-oss --smoke-test                    # verify BYOP still works
git add -A && git commit -m "feat(strip): delete {MODULE} module"
```

## Verification

- `cargo check -p warp` — 0 errors
- `warp-oss --smoke-test` — exits 0
- No `use crate::{MODULE}::` references remain
- {MOD_PATH} directory is gone

## Constraints

- Do NOT modify the BYOP code path (app/src/ai/agent_providers/, app/src/ai/blocklist/)
- Do NOT create stubs — just delete dead code and dead callers
- If something on the BYOP path actually USES a type/function from this module, STOP and report — it means our dependency analysis was wrong
- If the cascade goes deeper than ~50 files, take it in batches (fix 20, check, fix 20, check)
- One commit, message: "feat(strip): delete {MODULE} module"
