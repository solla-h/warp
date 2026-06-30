# Issue 4: Tech Spec - Delete Hollow Cloud Module Shells

## Context

After Issue 3 flips default features, `#[cfg(feature = "cloud")]` code no longer compiles. But many modules were previously NOT cfg-gated — they were "hollowed out" (methods kept with empty bodies) to satisfy type references from non-gated callers. Now those callers' cloud branches are also dead, allowing full deletion.

## Relevant code

- `app/src/cloud_object/mod.rs` — UpdateManager (50 no-op methods, 700+ lines)
- `app/src/cloud_object/breadcrumbs.rs`, `grab_edit_access_modal.rs`, `model.rs`, `toast_message.rs`
- `app/src/sync_queue.rs` — Empty SyncQueue struct (35 lines)
- `app/src/drive/mod.rs` — DrivePanel + 11 sub-modules (all stubbed)
- `app/src/workspaces/mod.rs` — 7 sub-modules (GQL, teams, profiles)
- `app/src/billing/mod.rs` — 2 modal stubs
- `app/src/server_experiments/mod.rs` — ServerExperiment enum + convert.rs
- `app/src/pricing/full_impl.rs` — Stripe pricing stubs (only used when NOT local-only)
- `app/src/lib.rs` — Module declarations for above

## Proposed changes

### 1. Delete module directories

```powershell
Remove-Item -Recurse app/src/cloud_object/
Remove-Item -Recurse app/src/drive/
Remove-Item -Recurse app/src/workspaces/
Remove-Item -Recurse app/src/billing/
Remove-Item -Recurse app/src/server_experiments/
Remove-Item app/src/sync_queue.rs
Remove-Item app/src/pricing/full_impl.rs
```

### 2. Remove module declarations from lib.rs

In `app/src/lib.rs`, remove or cfg-gate:
```rust
// DELETE these lines:
pub mod cloud_object;
pub mod sync_queue;
pub mod drive;
pub mod workspaces;
pub mod billing;
pub mod server_experiments;
```

### 3. Fix broken references iteratively

Run `cargo check`. For each error:
- If it's a `use crate::cloud_object::...` — delete the use statement and the code that depends on it
- If it's a `use crate::drive::...` — same
- If it's a field on a struct like `update_manager: cloud_object::UpdateManager` — remove the field and any code that accesses it
- If it's in a match arm — remove the arm

Strategy: compiler-driven deletion. Each `cargo check` reveals the next set of references to remove. Repeat until 0 errors.

### 4. Fix pricing module

`app/src/pricing/mod.rs` currently switches between `full_impl` and `stub_impl` based on `local-only` feature. After flip, `local-only` is always on, so:
- Delete `full_impl.rs`
- Rename `stub_impl.rs` to the sole implementation
- Simplify `mod.rs` to just `mod stub_impl; pub use stub_impl::*;` (no cfg gate needed)

### 5. Clean up #[allow(dead_code)] annotations

Search for and remove all `#[allow(dead_code)]` that were protecting these modules:
```powershell
rg "#\[allow\(dead_code" app/src/ --files-with-matches
```

Only remove annotations that were specifically for the deleted modules. Keep any that protect legitimately-used-later code.

## Testing and validation

### Iterative compile check (the primary "test")

```powershell
# After each batch of deletions:
cargo check --bin warp-oss
# Expect errors -> fix -> repeat until 0
```

### Search assertions

```powershell
# After all deletions complete:
rg "cloud_object" app/src/ --type rust  # Should only match comments or string literals
rg "SyncQueue" app/src/ --type rust     # Should return 0
rg "DrivePanel" app/src/ --type rust    # Should return 0
rg "ServerExperiment" app/src/ --type rust  # Should return 0
```

### Final validation

```powershell
cargo check --bin warp-oss   # 0 errors
cargo test -p warp --lib     # All tests pass
```

### Acceptance criteria

- [ ] `app/src/cloud_object/` directory deleted
- [ ] `app/src/drive/` directory deleted
- [ ] `app/src/workspaces/` directory deleted
- [ ] `app/src/billing/` directory deleted
- [ ] `app/src/server_experiments/` directory deleted
- [ ] `app/src/sync_queue.rs` deleted
- [ ] `app/src/pricing/full_impl.rs` deleted, pricing simplified
- [ ] All caller references to deleted modules removed
- [ ] `cargo check --bin warp-oss` passes
- [ ] `cargo test -p warp --lib` passes
- [ ] Commit: "refactor: delete hollow cloud module shells (~2000 lines)"

## Risks and mitigations

- **Risk:** A deeply-nested reference to `UpdateManager` or `DrivePanel` that spans many files
  - **Mitigation:** Follow compiler errors one by one. Each error points to exactly which line needs fixing. Typical fix: delete the line or surrounding if-block.

- **Risk:** Removing a struct field causes a struct literal construction elsewhere to fail
  - **Mitigation:** Search for all struct literals of the affected type and remove the field assignment.

- **Risk:** Test files reference deleted modules
  - **Mitigation:** Delete the test file or test function if it only tested cloud behavior. Keep tests for non-cloud behavior.