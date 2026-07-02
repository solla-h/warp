# Requirement 4: Clean app/Cargo.toml of dead deps

## Verdict: Cannot Remove (most deps still active)

## Reason summary

Both `cloud_objects` and `cloud_object_models` are **heavily used** across 37 source
files each in `app/src/`. They provide core types for auth (UserUid), drive (folders,
items, sharing), cloud_object model infrastructure, AI agent configs, MCP configs,
env vars, external secrets, notebooks, workflows, settings sync, and more.

The three feature flags (`cloud_object_initial_load`, `enforce_revisions_to_cloud_objects`,
`personal_cloud_objects`) are **dead** -- declared only in `app/Cargo.toml` and never
referenced in any `.rs` file anywhere in the repo.

## Dependencies still in app/Cargo.toml

### cloud_objects
- Line in Cargo.toml: 90
- Source files that import from it: 37 files
- Key modules: `app/src/auth/` (UserUid), `app/src/cloud_object/`, `app/src/drive/`,
  `app/src/ids.rs`, `app/src/sync_queue.rs`, `app/src/infra/server_api/object.rs`,
  `app/src/terminal/shared_session/`, `app/src/workspaces/`
- Required? **Yes** -- removing it breaks compilation immediately

### cloud_object_models
- Line in Cargo.toml: 89
- Source files that import from it: 37 files
- Key modules: `app/src/ai/` (agent configs, mcp, environments, execution profiles,
  facts, ambient agents), `app/src/cloud_object/`, `app/src/drive/folders/`,
  `app/src/env_vars/`, `app/src/external_secrets/`, `app/src/notebooks/`,
  `app/src/settings/cloud_preferences*`, `app/src/workflows/`
- Required? **Yes** -- removing it breaks compilation immediately
- Note: Line 704 also references `cloud_object_models/agent_mode_evals` as a
  sub-feature in the `agent_mode_evals` feature compound.

## Feature flags still in app/Cargo.toml

### cloud_object_initial_load
- Definition: Line 480 -- `cloud_object_initial_load = ["enforce_revisions_to_cloud_objects"]`
- What enables it: Nothing in current code (likely toggled by server-side feature flags
  or build profiles that are no longer wired up)
- What it gates: Zero `.rs` files contain `cfg(feature = "cloud_object_initial_load")`
- Is it dead? **Yes -- safe to remove**

### enforce_revisions_to_cloud_objects
- Definition: Line 673 -- `enforce_revisions_to_cloud_objects = []`
- What enables it: Only `cloud_object_initial_load` (which is itself dead)
- What it gates: Zero `.rs` files use this feature
- Is it dead? **Yes -- safe to remove**
### personal_cloud_objects
- Definition: Line 759 -- `personal_cloud_objects = []`
- What enables it: Nothing in current code
- What it gates: Zero `.rs` files use this feature
- Is it dead? **Yes -- safe to remove**

## What could potentially be cleaned now vs what must stay

### Can remove now (zero risk)
1. Feature flag `cloud_object_initial_load` (line 480)
2. Feature flag `enforce_revisions_to_cloud_objects` (line 673)
3. Feature flag `personal_cloud_objects` (line 759)

These are pure dead declarations. Removing them cannot break compilation because
no source code references them.

### Must stay (actively used)
1. `cloud_objects` dependency (line 90) -- 37 source files import from it
2. `cloud_object_models` dependency (line 89) -- 37 source files import from it
3. `cloud_object_models/agent_mode_evals` sub-feature (line 704) -- gates code in
   `crates/cloud_object_models/src/ai_execution_profile.rs:424`

### Also already gone
- `warp_graphql_schema` -- not in app/Cargo.toml, crate directory does not exist

## Steps to eventually remove

The spec (TECH.md) assumed Issues 3-7 would have removed all source-level usage
of these crates first. That has not happened. To complete the original plan:

1. **Inline or relocate types** from `cloud_object_models` that the app still
   needs (AgentConfig, UserUid, CloudFolder, EnvVar types, etc.) into app-local
   modules or a new lightweight crate.
2. **Decouple `cloud_objects`** by moving `UserUid` and `ids` types into a
   standalone `warp_ids` crate; move `drive` types into `app/src/drive/`.
3. **Remove the three dead feature flags** immediately (trivial, no code impact).
4. After the source migrations, remove the dependency lines and delete the crate
   directories.
5. Regenerate `Cargo.lock` and verify with `cargo check --bin warp-oss`.

