# Wave 3: Rewrite server/ids imports to warp_types
# Issue: https://github.com/solla-h/warp/issues/16

/implement https://github.com/solla-h/warp/issues/16

## Context

This is a Rust project — a hard fork of Warp Terminal called "Marb" (BYOP-only, no cloud).
In a prior task (T9), all ID types were physically moved to `crates/warp_types/`. The old
`app/src/server/ids` module now just re-exports them. This issue removes the re-export layer
and rewrites all import sites to point directly at `warp_types`.

Key files to read FIRST:
- `CONTEXT.md` — project context
- `PLAN.md` — execution plan (you are in Wave 3)
- `app/src/server/ids.rs` or `ids/mod.rs` — the re-export module to delete
- `crates/warp_types/src/lib.rs` — what's already exported

## What to do

Mechanical find-and-replace across the codebase:

1. **Identify all import sites**: `rg "use crate::server::ids::" app/src/`
2. **Replace each with warp_types**:
   - `use crate::server::ids::ServerId` → `use warp_types::ServerId`
   - `use crate::server::ids::{ClientId, SyncId}` → `use warp_types::{ClientId, SyncId}`
   - `use crate::server::ids::*` → `use warp_types::{ClientId, SyncId, ServerId, ...}` (expand the glob)
3. **Also check**: `use crate::server::ids` (path-style without `::` suffix)
4. **Delete the module**: Remove `app/src/server/ids.rs` (or `ids/` directory)
5. **Remove mod declaration**: Delete `mod ids` from `app/src/server/mod.rs`
6. **Ensure warp_types is in scope**: `app/Cargo.toml` should already have `warp_types` as a dependency. Verify.

## Types that live in warp_types (from T9):
- `ClientId`, `SyncId`, `ServerId`, `ObjectUid`, `HashedSqliteId`
- `HashableId` trait, `ToServerId` trait
- `server_id_traits!` macro
- `ObjectIdType` enum

## Workflow

```
rg "use crate::server::ids" app/src/ --files-with-matches | wc -l   # count files
# do the replacement (sed, editor, or manual)
# delete server/ids module
cargo check -p warp          # verify
warp-oss --smoke-test        # verify BYOP works
git add -A && git commit -m "feat(strip): rewrite server/ids imports to warp_types"
```

## Verification

- `cargo check -p warp` — 0 errors
- `warp-oss --smoke-test` — exits 0
- Zero `use crate::server::ids::` references remain
- `app/src/server/ids.rs` (or ids/ directory) is deleted

## Constraints

- This is purely mechanical — do NOT change any logic
- If a type is NOT in warp_types, STOP and add it to warp_types first
- Do NOT modify warp_types's public API beyond adding missing re-exports
- One commit: "feat(strip): rewrite server/ids imports to warp_types"
