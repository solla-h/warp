# Requirement 6: Remove `cynic` workspace dependency

## Verdict: Cannot Remove (yet)

## Reason summary

`cynic` is still actively used in 3 source files across the `app` crate and is listed
as a dependency in `crates/cloud_objects/Cargo.toml` (though unused there). All usage is
exclusively for the `cynic::Id` type -- a trivial newtype over `String`. The full
GraphQL client machinery (derive macros, query builders, HTTP transport) is not used.

## What cynic provides to this codebase

Only `cynic::Id` -- a `pub struct Id(String)` with `::new(String)` constructor and
`.into_inner() -> String` accessor. No GraphQL queries, mutations, subscriptions, or
derive macros are present anywhere outside the deleted `warp_graphql_schema` crate.

## Source files using cynic

### app/src/ai/ambient_agents/mod.rs (line 51)
- What it imports: `cynic::Id` (path-qualified, no `use` statement)
- How it's used: `impl From<AmbientAgentTaskId> for cynic::Id` -- converts a task
  UUID to a `cynic::Id` via `Self::new(id.to_string())`. This impl exists to pass
  task IDs into the `create_api_key` API which takes `Option<cynic::Id>` parameters.

### app/src/infra/server_api/auth.rs (lines 135-136, 184-185, 248-249)
- What it imports: `cynic::Id` (path-qualified, no `use` statement)
- How it's used: The `AuthClient` trait method `create_api_key` takes
  `team_id: Option<cynic::Id>` and `agent_uid: Option<cynic::Id>` parameters.
  This appears in:
  1. The trait definition (lines 135-136)
  2. The `mockall::mock!` block for testing (lines 184-185)
  3. A stub implementation that returns an error (lines 248-249)

### app/src/settings_view/platform_page.rs (line 58)
- What it imports: `cynic::Id` (path-qualified, no `use` statement)
- How it's used: `GqlApiKeyProperties` struct has field `pub uid: cynic::Id`.
  The `From<&GqlApiKeyProperties> for APIKeyProperties` impl calls
  `gql_key.uid.clone().into_inner()` to extract the inner `String`.

## Cargo.toml references

| File | Line | Entry |
|------|------|-------|
| `Cargo.toml` (workspace root) | 144 | `cynic = { version = "3" }` |
| `app/Cargo.toml` | 99 | `cynic.workspace = true` |
| `crates/cloud_objects/Cargo.toml` | 15 | `cynic.workspace = true` |

Note: `crates/cloud_objects` declares the dep but has ZERO actual `cynic::` references
in its source files -- this is a dead dependency that can be removed immediately.

## Analysis: Could cynic::Id be replaced?

Yes, trivially. `cynic::Id` is semantically identical to:

```rust
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct GraphQlId(String);

impl GraphQlId {
    pub fn new(s: String) -> Self { Self(s) }
    pub fn into_inner(self) -> String { self.0 }
    pub fn inner(&self) -> &str { &self.0 }
}
```

All 3 usage sites only call `::new(String)` and `.into_inner()`. No serialization,
deserialization, or other trait impls from cynic are exercised.

## What would need to happen to eventually remove it

1. **Remove dead dep from `crates/cloud_objects/Cargo.toml`** -- zero code changes needed.
2. **Define a local `GraphQlId` type** (or just use `String` directly) in a shared
   location, e.g. `app/src/infra/server_api/types.rs` or inline in `auth.rs`.
3. **Replace `cynic::Id`** in the 3 files above:
   - `auth.rs`: change `Option<cynic::Id>` to `Option<String>` (simplest) or the local type.
   - `platform_page.rs`: change `pub uid: cynic::Id` to `pub uid: String` and remove
     the `.clone().into_inner()` call (just `.clone()`).
   - `ambient_agents/mod.rs`: remove the `From<AmbientAgentTaskId> for cynic::Id` impl;
     replace with `From<AmbientAgentTaskId> for String` or produce `String` directly.
4. **Remove `cynic.workspace = true`** from `app/Cargo.toml`.
5. **Remove `cynic = { version = "3" }`** from workspace root `Cargo.toml`.

Estimated effort: ~30 minutes. No behavioral change, pure type substitution.
