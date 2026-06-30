# Agent Handoff Context

> Generated: 2026-06-30 | Branch: marb | Previous commit: be8ba1f3 (Issue 2)

## TL;DR

Issues 1 and 2 are **completed and committed**. Issue 3 is **in progress with uncommitted changes** — `cargo check` passes but `cargo test` is hitting a stale cargo cache issue. Issues 4-8 are **not started**.

---

## Full Task List (8 Issues)

| # | Issue | Status | Commit |
|---|-------|--------|--------|
| 1 | Physical cleanup (PDB, yarn, sqlite, stale docs) | ✅ DONE | `be8ba1f3` (partial — combined with Issue 2) |
| 2 | Purge dead dirs (specs/, .agents/skills/, docker/agent-dev/, images/, archive/) | ✅ DONE | `be8ba1f3` |
| 3 | Flip default features to BYOP-only | 🔧 IN PROGRESS (uncommitted) | — |
| 4 | Delete hollow cloud module shells | ⬜ NOT STARTED | — |
| 5 | Collapse telemetry/ to single-file no-op | ⬜ NOT STARTED | — |
| 6 | Collapse auth/ to BYOP-only | ⬜ NOT STARTED | — |
| 7 | Prune FeatureFlag enum (326→~50) | ⬜ NOT STARTED | — |
| 8 | Remove dead crates + patches | ⬜ NOT STARTED | — |

---

## Execution Order & Dependencies

```
Issue 1 (independent) ────────────────────────────────→ ✅ DONE
Issue 2 (independent) ────────────────────────────────→ ✅ DONE

Issue 3 ──→ Issue 4 ──→ Issue 7 ──→ Issue 8
              |
Issue 5 ─────┘ (depends on Issue 3)
Issue 6 ─────┘ (depends on Issue 3)
```

**Next steps**: Finish Issue 3 → Then Issue 4, 5, 6 (can be parallel) → Issue 7 → Issue 8

---

## Issue 3: Current State (IN PROGRESS)

### What was done

1. **`app/Cargo.toml`** — Replaced the `default = [...]` feature list:
   - **Removed**: `otel`, `aws-bedrock`, `cloud_ui`, `viewing_shared_sessions`, `shared_with_me`, `session_sharing_acls`, `loginless_conversion`, `global_ai_analytics_collection`, `usage_based_pricing`, `billing_and_usage_page_v2`, `agent_shared_sessions`, `shared_session_long_running_commands`, `cloud_mode_from_local_session`, `cloud_mode_image_context`, `cloud_mode_input_v2`, `cloud_mode_setup_v2`, `orchestration_launch_modal`, `orchestration_viewer_streamer`, `owner_orchestration_ancestor_streamer`, `oz_changelog_updates`, `hoa_code_review`, `hoa_notifications`, `hoa_onboarding_flow`, `hoa_remote_control`, `transfer_control_tool`, `drive_objects_as_context`
   - **Added**: `local-only`, `full_source_code_embedding`
   - **Kept**: `cloud` (CRITICAL — see "Strategy Adjustment" below)
   - All other original features retained

2. **`app/src/lib.rs`** — Removed `#[cfg(feature = "cloud")]` gate from `pub use crate::telemetry::{AgentModeEntrypoint, ...}` (line ~322):
   ```rust
   // BEFORE:
   #[cfg(feature = "cloud")]
   pub use crate::telemetry::{AgentModeEntrypoint, ...};
   // AFTER:
   pub use crate::telemetry::{AgentModeEntrypoint, ...};
   ```

3. **`app/src/pricing/stub_impl.rs`** — Added missing types that `full_impl.rs` had but `stub_impl.rs` lacked:
   - `StripeSubscriptionPlan` enum
   - `StripeSubscriptionPlanInfo` struct
   - `PlanPricing` struct
   - `AddonCreditsOption` struct + `rate()` method
   - `TryFrom<&BillingMetadata>` impl (cfg-gated on `cloud`)
   - `plan_pricing()`, `addon_credits_options()`, `plans()` methods on `PricingInfoModel`

4. **`app/src/uri/uri_tests.rs`** — Deleted 2 broken test functions:
   - `test_warp_web_link_notebook()` — referenced non-existent `ObjectType::Notebook`
   - `test_warp_web_link_workflow()` — referenced non-existent `ObjectType::Workflow`

### Strategy Adjustment (IMPORTANT)

The original TECH.md for Issue 3 says to **remove `cloud` from default entirely**. However, after attempting this, ~100+ compile errors appeared because many modules reference `crate::infra`, `cloud_objects`, `crate::workspaces`, `crate::server_block` etc. **without** `#[cfg(feature = "cloud")]` gates. These references are too deeply wired to fix in one pass.

**Decision**: Keep `cloud` in the `default` feature list for now. The `cloud` feature enables `dep:cloud_objects` and `dep:cloud_object_models` (both optional deps). Physical deletion of cloud code happens in Issues 4-6, and the `cloud` feature + crate deps are fully removed in Issue 8.

This still achieves meaningful slimming:
- `otel` removed → no more opentelemetry crates in dep tree
- `aws-bedrock` removed → no more AWS SDK crates
- `cloud_ui` removed → no more cloud_ui sub-features (12+ flags)
- All session/oz/billing/orchestration flags removed from default
- Dep tree: 3852 → 3503 lines (-349, -9%)

### Compilation Status

- **`cargo check --bin warp-oss`**: ✅ PASSES (0 errors, 9 warnings — all pre-existing)
- **`cargo test -p warp --lib`**: ❌ FAILS — but due to stale cargo cache, NOT real errors

### Stale Cache Issue (MUST FIX BEFORE COMMITTING)

The `cargo test` fails with:
```
error: couldn't read `app\src\drive\export.rs`: mod tests; at line 553
```

But `app/src/drive/export.rs` only has 549 lines — there is no `mod tests;` in it. This is a **cargo fingerprint cache issue**. The file was modified in a prior commit (Wave 1-4) but cargo's incremental cache still references the old version.

**Fix**: Run `cargo clean -p warp` (already done once) then rebuild. If the issue persists, also try:
```powershell
# Remove warp-specific fingerprint and incremental caches
Remove-Item -Recurse -Force target/debug/.fingerprint/warp-* -ErrorAction SilentlyContinue
Remove-Item -Recurse -Force target/debug/incremental/warp-* -ErrorAction SilentlyContinue
# Then re-run
cargo test -p warp --lib
```

There may be similar stale-cache issues with other files (e.g., `app/src/ids.rs` had a similar issue earlier where cargo saw `mod tests;` at line 76 but the file only has 72 lines).

### What remains to complete Issue 3

1. **Fix the stale cargo cache** (see above)
2. **Run `cargo test -p warp --lib`** — confirm all tests pass
3. **Git commit** with message: `feat: flip default features to BYOP-only minimum`
   ```powershell
   git add app/Cargo.toml app/src/lib.rs app/src/pricing/stub_impl.rs app/src/uri/uri_tests.rs
   git commit -m "feat: flip default features to BYOP-only minimum"
   ```

### Files modified but NOT yet committed

| File | Change |
|------|--------|
| `app/Cargo.toml` | Replaced `default = [...]` feature list |
| `app/src/lib.rs` | Removed `#[cfg(feature = "cloud")]` from telemetry re-export |
| `app/src/pricing/stub_impl.rs` | Added missing types from full_impl |
| `app/src/uri/uri_tests.rs` | Removed 2 broken test functions |

### Temp files to clean up

- `deptree_before.txt` — baseline dep tree (3852 lines)
- `deptree_after_3.txt` — post-Issue-3 dep tree (3503 lines)
- `cargo_check_3.txt`, `cargo_check_3b.txt`, `cargo_check_3c.txt`, `cargo_check_3d.txt` — check outputs
- `cargo_test_3.txt`, `cargo_test_3b.txt`, `cargo_test_3c.txt` — test outputs

---

## Issues 4-8: Summary of What's Next

### Issue 4: Delete Hollow Cloud Module Shells
- Delete: `app/src/cloud_object/`, `app/src/drive/`, `app/src/workspaces/`, `app/src/billing/`, `app/src/server_experiments/`, `app/src/sync_queue.rs`, `app/src/pricing/full_impl.rs`
- Remove module declarations from `app/src/lib.rs`
- Fix all caller references iteratively via `cargo check`
- Simplify pricing module (delete `full_impl.rs`, make `stub_impl` the sole impl)
- Commit: `refactor: delete hollow cloud module shells (~2000 lines)`

### Issue 5: Collapse telemetry/ to Single-File No-Op
- Replace 12-file `app/src/telemetry/` directory with single `app/src/telemetry.rs` (~30 lines)
- Preserve public API: `TelemetryApi`, `TelemetryEvent`, `AgentModeEntrypoint`, etc.
- Remove `otel` feature definition from `app/Cargo.toml`
- Commit: `refactor: collapse telemetry to single-file no-op`

### Issue 6: Collapse auth/ to BYOP-Only
- Reduce 18-file `app/src/auth/` to 2-3 files
- Keep: `mod.rs`, `auth_state.rs`, `user.rs`
- Delete: `auth_manager.rs`, `credentials.rs`, `slides/`, `sso/`, etc.
- AuthState always reports `is_logged_in() == true`
- Remove `oauth2` dependency
- Commit: `refactor: collapse auth to BYOP-only (18 files -> 3)`

### Issue 7: Prune FeatureFlag Enum (326→~50)
- Delete dead variants from `crates/warp_features/src/lib.rs`
- Clean up `enabled_features()` in `app/src/features.rs`
- Hardcode "always-on" flags (replace `if FeatureFlag::X.is_enabled()` with unconditional execution)
- Commit: multiple commits, one per batch of ~10-20 flags

### Issue 8: Remove Dead Crates + Patches
- Delete: `crates/cloud_objects/`, `crates/cloud_object_models/`, `crates/warp_graphql_schema/`
- Remove from `Cargo.toml` workspace deps
- Remove `tink-*` patch entries
- Remove `session-sharing-protocol` patch section
- Remove `cloud` feature from `app/Cargo.toml` default (FINALLY possible after Issues 4-7)
- Regenerate `Cargo.lock`
- Commit: `chore: remove dead crate dirs and patch entries`

---

## Key Constraints (Red Lines)

1. **DO NOT modify** `app/src/ai/agent_providers/chat_stream.rs` — BYOP core stream
2. **DO NOT delete** `crates/remote_server/` — SSH remote dev, not cloud
3. **KEEP** `CONTEXT.md` in root — used by AI skill system
4. **KEEP** `app/src/terminal/ref_tests/` — valid regression test data
5. **KEEP** user-toggleable FeatureFlags (ligatures, rect_selection, kitty_images, etc.)
6. **One commit per logical step** — easy to bisect
7. **Compile fails → stop and fix** before continuing

---

## Environment Notes

- Windows 11 + PowerShell 7 (but cmd.exe is the default shell in this environment)
- Files use **LF line endings** (not CRLF)
- `cargo check --bin warp-oss` takes ~3-4 minutes (incremental)
- `cargo test -p warp --lib` takes ~5-8 minutes (full rebuild after `cargo clean`)
- `cargo tree -p warp` for dep tree measurement (NOT `--bin warp-oss` — that flag doesn't exist in this cargo version)
- Baseline dep tree: **3852 lines** (captured before Issue 3)
- Post-Issue-3 dep tree: **3503 lines** (-9%)
- Target: **< 2500 lines** (achieved after Issue 8)

---

## Git State

```
Branch: marb
Last commit: be8ba1f3 "chore: purge Warp-internal specs, skills, and brand assets"
Uncommitted changes: 4 files modified (see "Files modified but NOT yet committed" above)
Untracked files: specs/a-handoff/ (this handoff doc), scripts/marb-slimming-review.md, temp txt files
```

---

## How to Continue

1. Read this file completely
2. Read `specs/a-handoff/HANDBOOK.md` for full project context
3. Read the TECH.md for Issue 3 (`specs/a-handoff/issue-3-flip-default-features/TECH.md`)
4. Fix the stale cargo cache (see "Stale Cache Issue" above)
5. Run `cargo test -p warp --lib` — confirm pass
6. Commit Issue 3
7. Proceed to Issue 4 (read its TECH.md first)
8. Continue through Issues 5-8 per the execution order
