# Issue 8: Tech Spec - Remove Dead Crate Dependencies

## Context

This is the final cleanup issue. After Issues 3-7 have removed all code that references cloud crates, the crate directories and dependency declarations become dead weight. This issue removes them and regenerates Cargo.lock.

## Relevant code

- `Cargo.toml` (root) — workspace members, [workspace.dependencies], [patch.crates-io]
- `crates/cloud_objects/` — entire crate directory
- `crates/cloud_object_models/` — entire crate directory
- `crates/warp_graphql_schema/` — entire crate directory (2-line lib.rs)
- `app/Cargo.toml` — dependency declarations for above crates

## Proposed changes

### 1. Remove crate directories

```powershell
Remove-Item -Recurse crates/cloud_objects/
Remove-Item -Recurse crates/cloud_object_models/
Remove-Item -Recurse crates/warp_graphql_schema/
```

### 2. Remove from workspace members

In root `Cargo.toml`:
- Remove from `exclude = [...]` if listed there
- The `members = ["crates/*"]` glob will auto-exclude deleted dirs

### 3. Remove [workspace.dependencies] entries

In root `Cargo.toml`, remove workspace dependency declarations:
```toml
# DELETE these:
cloud_objects = { path = "crates/cloud_objects" }
cloud_object_models = { path = "crates/cloud_object_models" }
warp_graphql_schema = { path = "crates/warp_graphql_schema" }
```

Also remove any git dependencies that are ONLY reachable through these:
- `session-sharing-protocol` (check: `rg "session.sharing.protocol" **/Cargo.toml`)
- `warp_multi_agent_api` (check: is it used elsewhere?)
- `cynic`, `graphql-ws-client` (check: any other consumer?)

### 4. Remove [patch.crates-io] entries

```toml
# DELETE these 3 lines:
tink-core = { git = "..warpdotdev/tink-rust..", rev = "..." }
tink-proto = { git = "..warpdotdev/tink-rust..", rev = "..." }
tink-hybrid = { git = "..warpdotdev/tink-rust..", rev = "..." }
```

Also delete the entire `[patch."https://github.com/warpdotdev/session-sharing-protocol.git"]` section if it exists.

### 5. Remove from app/Cargo.toml

```toml
# DELETE these dependency lines:
cloud_objects = { workspace = true, optional = true }
cloud_object_models = { workspace = true, optional = true }
warp_graphql_schema = { workspace = true }
```

Remove any other deps that are now orphaned (use `cargo check` to identify).

### 6. Regenerate Cargo.lock

```powershell
Remove-Item Cargo.lock
cargo generate-lockfile
```

Or just run `cargo check` which will update it automatically.

### 7. Verify dep tree shrinkage

```powershell
$lines = (cargo tree --bin warp-oss 2>$null | Measure-Object -Line).Lines
Write-Host "Dep tree: $lines lines"
# Compare against baseline captured at start of project (~4378)
```

## Testing and validation

### Verification

```powershell
# Must pass:
cargo check --bin warp-oss
cargo test -p warp --lib

# Must show no cloud deps:
cargo tree --bin warp-oss | Select-String "tink"              # 0 results
cargo tree --bin warp-oss | Select-String "session-sharing"   # 0 results
cargo tree --bin warp-oss | Select-String "cynic"             # 0 results
cargo tree --bin warp-oss | Select-String "cloud_object"      # 0 results

# Must not reference deleted crates:
rg "cloud_objects" Cargo.toml app/Cargo.toml     # 0 results
rg "cloud_object_models" Cargo.toml app/Cargo.toml  # 0 results
rg "warp_graphql_schema" Cargo.toml app/Cargo.toml  # 0 results
rg "tink" Cargo.toml                              # 0 results
```

### Acceptance criteria

- [ ] `crates/cloud_objects/` deleted
- [ ] `crates/cloud_object_models/` deleted
- [ ] `crates/warp_graphql_schema/` deleted
- [ ] tink-* entries removed from [patch.crates-io]
- [ ] session-sharing-protocol patch section removed
- [ ] Workspace deps cleaned of dead entries
- [ ] app/Cargo.toml cleaned of dead deps
- [ ] `cargo check --bin warp-oss` passes
- [ ] `cargo test -p warp --lib` passes
- [ ] No tink, session-sharing, cynic, or cloud_object in dep tree
- [ ] Cargo.lock regenerated cleanly
- [ ] Commit: "chore: remove dead crate dirs and patch entries"

## Risks

- **Risk:** A crate that appears dead is actually transitively required
  - **Mitigation:** `cargo check` will immediately fail if something is still needed. Re-add if necessary.

- **Risk:** Removing a workspace dep that another remaining crate uses
  - **Mitigation:** Before removing each dep declaration, grep ALL Cargo.toml files for references to it.

- **Risk:** `session-sharing-protocol` is still used by terminal/shared_session/ code
  - **Mitigation:** If that module was cfg-gated or deleted in Issues 3-4, the dep is safe to remove. If NOT, keep it until the shared_session module is fully deleted.