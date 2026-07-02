# Requirement 3: Remove workspace dependency declarations

## Verdict: Cannot Remove

## Reason summary

Both `cloud_objects` and `cloud_object_models` are **non-optional, unconditionally compiled**
dependencies of the `app` crate. They are imported without any feature gate in core modules
(`auth`, `cloud_object`, `drive`, `ids`, `ai/`). The spec (TECH.md) assumed Issues 3-7 would
have already stripped all consuming code, but that work has not been completed. Removing the
workspace dependency declarations today causes an immediate `cargo check` failure.

## Current state in root Cargo.toml

Lines 48-49:
```toml
cloud_objects = { path = "crates/cloud_objects" }
cloud_object_models = { path = "crates/cloud_object_models" }
```

These declare both crates as workspace-level dependencies so consumers can
use `{ workspace = true }` syntax.

## Consumer Cargo.toml files

### app/Cargo.toml

Lines 89-90 (dependencies section):
```toml
cloud_object_models = { workspace = true }
cloud_objects = { workspace = true }
```

- **Optional?** No. Neither has `optional = true`.
- **Feature-gated?** No. The `cloud_object` module in `app/src/lib.rs` (line 20) is
  declared unconditionally (`mod cloud_object;`), as is `mod auth;`.
- **Compiled in default build?** Yes, always.

Feature-flag references (informational only — these do NOT gate the dependency itself):
- Line 480: `cloud_object_initial_load = ["enforce_revisions_to_cloud_objects"]`
- Line 673: `enforce_revisions_to_cloud_objects = []`
- Line 704: `"cloud_object_models/agent_mode_evals"` (inside `agent_mode_evals` feature)
- Line 759: `personal_cloud_objects = []`

None of these features are in `default = [...]`, but the dependency itself is always linked.

**Why needed:** The `app` crate imports types from both crates in >170 locations across
`src/auth/`, `src/cloud_object/`, `src/drive/`, `src/ids.rs`, and `src/ai/`.
Key re-exports include `UserUid`, `CloudObjectTypeAndId`, `Owner`, `AccessLevel`,
`GenericStringModel`, `AgentConfig`, `HarnessConfig`, and many more.

### crates/cloud_object_models/Cargo.toml

Line 20 (regular dependency):
```toml
cloud_objects.workspace = true
```

Line 45 (dev-dependency):
```toml
cloud_objects = { workspace = true, features = ["test-util"] }
```

Line 11 (feature propagation):
```toml
test-util = ["cloud_objects/test-util"]
```

- **Optional?** No.
- **Why needed:** `cloud_object_models` is a higher-level crate that wraps
  `cloud_objects` types with domain model logic. It directly depends on
  `cloud_objects` for IDs, traits, and object primitives.

### No other workspace crates

No other crate under `crates/` (outside `cloud_objects` and `cloud_object_models`
themselves) references either dependency.

## Dependency graph

```
app
├── cloud_objects          (direct, non-optional)
├── cloud_object_models    (direct, non-optional)
│   └── cloud_objects      (direct, non-optional)
```

Transitive deps pulled in exclusively by these two crates:
- `cynic` (GraphQL client) — used only by `cloud_objects`
- `session-sharing-protocol` — used by `cloud_objects` and `cloud_object_models`
- `lasso` — used by `cloud_objects`

## What would need to happen to eventually remove them

1. **Complete Issues 3-7 first.** The spec's precondition is that all consuming code
   has been removed or replaced before Issue 8 runs. Currently ~115 `cloud_objects`
   references and ~58 `cloud_object_models` references exist in `app/src/`.

2. **Extract or inline essential types.** Some types like `UserUid` are used in core
   auth paths that remain necessary even in local-only mode. These must be moved to
   a surviving crate (e.g., `warp_types` or `warp_core`) or inlined.

3. **Remove `app/src/cloud_object/` module.** This is the largest consumer (~20 files)
   and provides the local persistence/sync layer for cloud objects.

4. **Remove `app/src/drive/` cloud-object imports.** Drive uses `CloudObjectTypeAndId`,
   `FolderId`, `ServerPermissions`, and `Owner`.

5. **Remove AI cloud config types.** `app/src/ai/` uses `CloudAgentConfig`,
   `HarnessConfig`, `AgentConfigSnapshot`, etc. from `cloud_object_models`. These
   would need local replacements or the AI modules refactored.

6. **Only then** can the workspace dependency declarations and crate directories
   be safely deleted as described in the TECH.md spec.
