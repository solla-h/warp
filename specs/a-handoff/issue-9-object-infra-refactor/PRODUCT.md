# Issue 9: Object Infrastructure Refactor

## Problem Statement

After Issues 1-8, the Marb fork compiles and runs without cloud features, but the codebase retains two crates named `cloud_objects` (1772 LOC) and `cloud_object_models` (3212 LOC) plus an app module `app/src/cloud_object/` (6959 LOC). Despite the "cloud" naming, these provide the foundational object model infrastructure — identity types, permissions, persistence framework, and object lifecycle — used by **200 files across 416 import sites**.

The problems:
1. **Misleading naming** — Every new contributor opening this BYOP terminal sees "cloud_objects" and assumes it's dead cloud code that should be deleted. It isn't.
2. **Dead methods within live crates** — The hollow-shell approach from Issue 4 left ~50% of methods as no-ops. These still compile but confuse readers.
3. **Stale dependencies** — `cynic` is used only for its `Id` type (a String newtype) across 3 files; 3 feature flags in `app/Cargo.toml` reference nothing.
4. **Unnecessary coupling** — `cloud_objects` imports `session-sharing-protocol` just for a `From<Role>` impl in `sharing.rs` that could be an adapter in app code.

## Solution

A three-phase refactor that transforms the "cloud object" layer into a clean, correctly-named local object infrastructure without breaking the 200-file consumer surface:

- **Phase A** (immediate): Remove provably dead code — `cynic` dep, dead feature flags, dead methods
- **Phase B** (short-term): Rename crates and module to remove "cloud" branding
- **Phase C** (optional, deferred): Slim inter-crate coupling by moving adapter impls to app

Each phase ships independently, passes `cargo check --bin warp-oss`, and maintains full backward compatibility for consumer code within the same phase.

## User Stories

1. As a new contributor, I want crate names to reflect their purpose (object model types, not cloud sync), so that I don't waste time investigating whether they're dead code.
2. As a developer extending the AI agent system, I want the object framework free of no-op ghost methods, so that I know which APIs actually do something.
3. As a developer, I want zero dead dependencies in Cargo.toml, so that `cargo build` doesn't fetch and compile code that's never called.
4. As a CI maintainer, I want the dependency graph to be minimal, so that builds are fast and auditable.
5. As a developer reading `cloud_objects/drive/sharing.rs`, I want to see only the types I need (SharingAccessLevel, Subject), without the session-sharing-protocol coupling that's irrelevant to the BYOP use case.
6. As a developer running `cargo test -p cloud_objects`, I want the test build to succeed, so that I can iterate locally without `--no-run` hacks.
7. As a developer navigating `app/src/`, I want module names like `objects/` instead of `cloud_object/`, so that the directory tree makes sense for a BYOP product.
8. As a developer working on notebooks or workflows, I want `use crate::objects::model::persistence::CloudModel` to be renamed to something like `ObjectModel`, so that the type name matches the runtime behavior (local persistence, not cloud sync).
9. As a release engineer, I want the workspace Cargo.toml free of patch sections and dead workspace deps pointing to warpdotdev forks, so that builds are reproducible without access to Warp's private repos.
10. As a developer doing `grep -r "cloud"` to find remaining cloud code, I want zero false positives from the object infrastructure layer, so that cleanup audits are accurate.

## Implementation Decisions

### Phase A: Dead Code Removal (2-3 hours)

1. **Replace `cynic::Id` with `String`** in 3 files:
   - `app/src/ai/ambient_agents/mod.rs` — remove `From<AmbientAgentTaskId> for cynic::Id`
   - `app/src/infra/server_api/auth.rs` — change `cynic::Id` params to `String`
   - `app/src/settings_view/platform_page.rs` — change `uid: cynic::Id` field to `String`
   Then remove `cynic` from `app/Cargo.toml` and root workspace `Cargo.toml`.

2. **Delete 3 dead feature flags** from `app/Cargo.toml`:
   - `cloud_object_initial_load`
   - `enforce_revisions_to_cloud_objects`
   - `personal_cloud_objects`
   These have zero consumers in any `.rs` file.

3. **Remove `cynic` from `crates/cloud_objects/Cargo.toml`** — it's listed but unused within the crate.

4. **Fix `cloud_objects` test build** — the `From<i64> for ServerId` bound fails in test cfg. Either add the impl to `warp_types` or gate the test that needs it.

### Phase B: Rename (2-3 days)

1. **Rename crates:**
   - `crates/cloud_objects/` → `crates/object_types/`
   - `crates/cloud_object_models/` → `crates/object_models/`
   - Package names in their Cargo.toml files updated accordingly.

2. **Rename app module:**
   - `app/src/cloud_object/` → `app/src/objects/`
   - Update `app/src/lib.rs` module declaration.

3. **Update all import paths** — this is mechanical:
   - `use cloud_objects::` → `use object_types::`
   - `use cloud_object_models::` → `use object_models::`
   - `crate::cloud_object::` → `crate::objects::`
   - Root `Cargo.toml` workspace dep entries.
   - `app/Cargo.toml` dependency entries.

4. **Verification method:** After each rename step, `cargo check --bin warp-oss` must pass. The compiler will catch every missed path.

5. **Do NOT rename types** in this phase — `CloudModel`, `CloudObjectMetadata`, etc. keep their names. Type renames are a separate decision that affects serialized data (SQLite columns, serde fields) and should be scoped independently.

### Phase C: Decouple (optional, 1-2 days)

1. **Move `From<Role> for SharingAccessLevel`** impl from `cloud_objects/drive/sharing.rs` to `app/src/terminal/shared_session/` where it's actually used. This removes the `session-sharing-protocol` dependency from `object_types`.

2. **Remove `session-sharing-protocol` from `object_types/Cargo.toml`** once the adapter is moved.

3. **Audit remaining no-op methods** in `object_types` and `object_models` — methods that return `Default::default()`, `vec![]`, `None`, or empty strings with no real logic. Delete those with zero callers. For those with callers, annotate with `#[allow(unused)]` or restructure the call site.

### Module dependency after all phases:

```
warp_types (pure data: ServerId, SyncId, UserUid, ServerTimestamp)
     ↓
object_types (schema: ObjectType, Owner, Permissions, Framework)
     ↓
object_models (concrete: AgentConfig, Workflow, EnvVarCollection, ObjectClient)
     ↓
app/src/objects/ (app layer: persistence, views, sync queue)
     ↓
app/src/{ai, drive, notebooks, workflows, ...} (consumers)
```

## Testing Decisions

1. **Primary test seam:** `cargo check --bin warp-oss` — the type system is the test. Every import path, every type reference, every method call is verified at compile time. A passing check means zero regressions.

2. **Secondary test seam:** `cargo test -p warp --lib` — runs the app-level unit tests that exercise the object model layer (33 tests in `app/src/cloud_object/`).

3. **What makes a good test here:** Since this is a rename/delete refactor with zero behavioral change, tests should verify that the build succeeds and existing tests still pass. No new tests are needed — the refactor is semantically neutral.

4. **Pre-existing test failures:** `cargo test -p cloud_objects` currently fails to compile in test cfg due to a `From<i64> for ServerId` bound issue. Phase A should fix this as a prerequisite so Phase B can use it as a regression gate.

5. **Prior art:** The prior issues (4-8) all used `cargo check -p warp` as their primary verification, with `cargo test -p warp_features` as secondary. Same pattern applies here.

## Out of Scope

- **Type renames** (e.g., `CloudModel` → `ObjectModel`, `CloudObjectMetadata` → `ObjectMetadata`) — these affect serialized data in SQLite and would require a data migration. Separate issue.
- **Behavioral changes** — The hollow-shell no-op methods remain no-ops. This refactor doesn't add real implementations.
- **Splitting into micro-crates** — Creating separate `auth_types`, `drive_types`, `object_framework` crates is premature optimization. The current 2-crate structure is adequate.
- **Removing `session-sharing-protocol`** entirely — It's used by 68 files for terminal session sharing, which is a working feature.
- **Removing `warp_multi_agent_api`** — It's the agent protocol backbone (91 files, SQLite persistence format).
- **Removing `cloud_object_models`** — It provides 40+ actively-used type definitions.

## Further Notes

- The glob re-export at `app/src/cloud_object/mod.rs:44` (`pub use cloud_objects::cloud_object::*`) is the single line responsible for 200 files having transitive access. After rename, it becomes `pub use object_types::cloud_object::*` (the internal module name within the crate can be renamed separately later).
- Phase B (rename) produces a large diff (200+ files) but is entirely mechanical and zero-risk since the compiler catches all breakage. It should be done in a single commit for clean git blame.
- The `session-sharing-protocol` git dependency points to `github.com/warpdotdev/session-sharing-protocol`. Phase C eliminates this from `object_types` but it remains in `app/Cargo.toml` and `crates/warp_terminal/Cargo.toml`. Full removal of this warpdotdev fork dependency requires forking or vendoring the protocol crate — a separate decision.
