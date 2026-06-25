# Wave 4: Delete server/ shell (final cleanup)
# Issue: https://github.com/solla-h/warp/issues/19

/implement https://github.com/solla-h/warp/issues/19

## Context

This is a Rust project — a hard fork of Warp Terminal called "Marb" (BYOP-only, no cloud).
This is the FINAL issue in the cloud removal plan. By this point, all server submodules
have been deleted or extracted:
- server/graphql/ — deleted in Wave 2
- server/iap/ — deleted in Wave 2
- server/sync_queue/ — deleted in Wave 2
- server/cloud_objects/ — deleted in Wave 2
- server/ids — rewritten to warp_types in Wave 3
- server/telemetry — replaced with local stub in Wave 3
- server/server_api — extracted to infra/ in Wave 3

Key files to read FIRST:
- `CONTEXT.md` — project context
- `PLAN.md` — execution plan (you are in Wave 4, the final wave)
- `app/src/server/mod.rs` — what's left in the server module
- `app/src/lib.rs` — where `mod server` is declared

## What to do

Delete whatever remains in `app/src/server/` after all prior waves completed.

### Likely remaining files:
- `server/mod.rs` — now mostly empty
- `server/network_logging.rs` — if it has BYOP utility, move to infra/ first
- `server/retry_strategies.rs` — if BYOP uses retry logic, move to infra/ first

### Steps:
1. **Audit remaining files**: `ls app/src/server/`
2. **Check for BYOP usage**: For each remaining file, grep if it's used by the ai/ path
   - If YES → move to `app/src/infra/` before deletion
   - If NO → delete directly
3. **Delete the directory**: `rm -rf app/src/server/`
4. **Remove mod declaration**: Delete `mod server` from `app/src/lib.rs`
5. **Fix any remaining imports**: `rg "use crate::server::" app/src/`
6. **Verify**

### Milestone significance:
After this commit, the word "server" no longer exists as a top-level module in Marb.
This is the architectural milestone marking the completion of cloud removal.

## Workflow

```
ls app/src/server/                          # what's left?
rg "use crate::server::" app/src/ | wc -l  # any remaining refs?
# move useful files to infra/ if needed
rm -rf app/src/server/
# remove `mod server` from lib.rs
cargo check -p warp
warp-oss --smoke-test
git add -A && git commit -m "feat(strip): delete server/ module — cloud removal complete"
```

## Verification

- `cargo check -p warp` — 0 errors
- `warp-oss --smoke-test` — exits 0
- `app/src/server/` directory completely gone
- `mod server` removed from lib.rs
- Zero `use crate::server::` references anywhere in the codebase
- BYOP conversation works end-to-end

## Constraints

- Do NOT delete files that are still used by the BYOP path — move them to infra/ instead
- If there are more remaining files than expected, STOP and assess — it may mean a prior wave didn't complete properly
- This should be a CLEAN deletion with minimal fixup if prior waves were done correctly
- One commit: "feat(strip): delete server/ module — cloud removal complete"
