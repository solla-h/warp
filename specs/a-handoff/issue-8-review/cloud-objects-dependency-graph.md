# cloud_objects — Complete Dependency & Reference Graph

## Scale

| Metric | Count |
|--------|-------|
| Direct `use cloud_objects::` imports (app/src/) | 30 lines across 19 files |
| Transitive consumers via `crate::cloud_object::*` | 416 import lines across 200 files |
| Cargo.toml declarations | 4 crates |
| Types defined in `cloud_objects` | ~50 (26 in mod.rs + sub-modules) |
| Types re-exported through glob | All of `cloud_objects::cloud_object::*` |

## Architecture Diagram

```
┌─────────────────────────────────────────────────────────────────────────┐
│                            warp_types                                     │
│  (UserUid, ServerId, SyncId, ClientId, ServerTimestamp, ObjectUid, ...)  │
└───────────────────────────────────┬─────────────────────────────────────┘
                                    │ re-exports & extends
                                    ▼
┌─────────────────────────────────────────────────────────────────────────┐
│                          cloud_objects                                    │
│                                                                          │
│  ┌──────────────────┐  ┌────────────────┐  ┌─────────────────────────┐ │
│  │ auth/            │  │ ids.rs         │  │ cloud_object/           │ │
│  │  UserUid (re)    │  │  FolderId      │  │  mod.rs (26 types)     │ │
│  │  TEST_USER_*     │  │  GenericStr..  │  │  creation.rs (6 types) │ │
│  │                  │  │  ServerIdExt   │  │  generic_cloud_obj (2) │ │
│  │                  │  │  SyncIdExt     │  │  generic_str_model (2) │ │
│  │                  │  │  ServerIdAnd.. │  │  server_object.rs (4)  │ │
│  └──────────────────┘  └────────────────┘  │  update.rs (3 types)   │ │
│                                            └─────────────────────────┘ │
│  ┌────────────────────────────────────────────────────────────────────┐ │
│  │ drive/                                                              │ │
│  │  mod.rs — CloudObjectTypeAndId                                     │ │
│  │  sharing.rs — SharingAccessLevel, Subject, UserKind, TeamKind, ... │ │
│  └────────────────────────────────────────────────────────────────────┘ │
└───────────────────────────────────┬─────────────────────────────────────┘
                                    │ depends on
                                    ▼
┌─────────────────────────────────────────────────────────────────────────┐
│                      cloud_object_models                                  │
│  (ObjectClient trait, AgentConfig, CloudFolder, all model structs)       │
└───────────────────────────────────┬─────────────────────────────────────┘
                                    │
                                    ▼
<!-- PLACEHOLDER_SECTION_2 -->
```

## The Glob Re-export Bottleneck

```rust
// app/src/cloud_object/mod.rs:44
pub use cloud_objects::cloud_object::*;
```

This single line makes every public type from `cloud_objects::cloud_object` available as
`crate::cloud_object::SomeType` throughout `app/src/`. **200 files** import through this path.

## Consumer Distribution by Subsystem

| Subsystem | Files | Key types used |
|-----------|-------|----------------|
| ai/ | 51 | CloudModel, StringModel, GenericStringObjectId, CloudObjectLookup, Owner |
| search/ | 28 | CloudModel, GenericStringObjectId, model::persistence types |
| terminal/ | 19 | CloudModel, CloudObjectLookup, SharingAccessLevel |
| notebooks/ | 15 | CloudModel, GenericStringObjectId, model::persistence |
| settings_view/ | 12 | GenericStringObjectId, model::persistence, StringModel |
| workflows/ | 11 | GenericStringObjectId, model::persistence |
| drive/ | 11 | CloudObjectTypeAndId, Owner, FolderId, ServerPermissions |
| integration_testing/ | 8 | CloudModel, various test fixtures |
| env_vars/ | 6 | GenericStringObjectId, model::persistence |
| cloud_object/ | 6 | All internal types (this IS the module) |
| settings/ | 5 | model::persistence |
| workspace/ | 4 | CloudModel, UpdateManager |
| pane_group/ | 4 | CloudModel, notebook-related types |
| persistence/ | 3 | ObjectType, SyncId |
| code/ | 3 | model::persistence (tests) |
| infra/ | 2 | SharingAccessLevel, FolderId, ServerId, SerializedModel |
| auth/ | 2 | CloudModel, UpdateManager |
| Others | 10 | Various |

## Direct Import Sites (19 files importing `use cloud_objects::`)

These bypass the glob and import directly from the crate:

| File | Import | Purpose |
|------|--------|---------|
| `auth/mod.rs` | `pub use cloud_objects::UserUid` | App-wide identity type |
| `auth/auth_state.rs` | `use cloud_objects::UserUid` | Auth state |
| `auth/user/mod.rs` | `UserUid`, `TEST_USER_EMAIL`, `TEST_USER_UID` | User model |
| `auth/user/persistence.rs` | `UserUid` | User persistence |
| `cloud_object/mod.rs` | `UpdatedObjectInput`, `cloud_object::*` (glob) | Central re-export |
| `cloud_object/model/generic_string_model.rs` | `CloudObjectUpsertParams`, `GenericStringModel`, `Serializer`, `GenericStringObjectId` | String model layer |
| `cloud_object/model/json_model.rs` | `GenericStringObjectFormat`, `SerializedModel` | JSON model layer |
| `ids.rs` | `GenericStringObjectId`, full ids re-export | App ID types |
| `drive/mod.rs` | `CloudObjectTypeAndId`, `Owner` | Drive root |
| `drive/items.rs` | `CloudObjectTypeAndId` | Drive items |
| `drive/folders/mod.rs` | `FolderId` | Folder operations |
| `drive/index_tests.rs` | `ServerPermissions` | Tests |
| `sync_queue.rs` | `SerializedModel` | Sync queue |
| `infra/server_api/object.rs` | `SharingAccessLevel`, `FolderId`, `ServerId`, `SerializedModel` | Server API |
| `ai/cloud_environments/mod.rs` | `Owner` | AI env ownership |
| `ai/execution_profiles/profiles_tests.rs` | `AccessLevel` | Tests |
| `workspaces/user_workspaces.rs` | `Owner` | Workspace ownership |
| `terminal/shared_session/permissions_manager.rs` | `SharingAccessLevel` | Permissions |
| `terminal/view/.../cloud_conversation_continuation_tests.rs` | `AccessLevel` | Tests |

## Type Dependency Layers

The types in `cloud_objects` form a layered hierarchy:

### Layer 0: Pure Identity (could live in warp_types)
- `UserUid` — already defined in warp_types, merely re-exported
- `TEST_USER_EMAIL`, `TEST_USER_UID` — test constants
- `FolderId`, `GenericStringObjectId` — thin ServerId wrappers

### Layer 1: Object Schema (depends on warp_types)
- `ObjectType` — enum of all cloud object kinds
- `ObjectIdType` — mirrors ObjectType for ID routing
- `GenericStringObjectFormat` — string object subtypes
- `JsonObjectType` — JSON object subtypes
- `Revision` — ServerTimestamp wrapper for versioning

### Layer 2: Ownership & Permissions (depends on Layer 1)
- `Owner` — User | Team discriminator
- `SharingAccessLevel` — View | Edit | Full
- `Subject`, `UserKind`, `TeamKind` — sharing permission targets
- `CloudObjectPermissions` — full permission state
- `ServerPermissions`, `ServerMetadata` — server-side state

### Layer 3: Object Framework (depends on Layers 0-2)
- `GenericCloudObject<K, M>` — generic cloud-synced object
- `CloudObjectUpsertParams<M>` — upsert parameters
- `GenericStringModel<M, S>` — string-serialized model
- `Serializer<M>` trait — custom serialization
- `ServerObjectModel` / `ServerObject` traits
- `GenericServerObject<K, M>` — server-side representation

### Layer 4: Lifecycle (depends on Layer 3)
- `CreateObjectRequest`, `BulkCreateGenericStringObjectsRequest`
- `UpdatedObjectInput`, `ObjectsToUpdate`
- `CloudObjectSyncStatus`, `CloudObjectStatuses`
- `CloudObjectMetadata` — full mutable metadata state
- `SerializedModel` — serialized string blob
- `CloudObjectEventEntrypoint` — telemetry routing

### Layer 5: Drive (depends on Layers 1-2)
- `CloudObjectTypeAndId` — type+id for drive actions
- `sharing.rs` types — full sharing model with session-sharing-protocol interop

## Cargo.toml References

| File | Declaration | Notes |
|------|------------|-------|
| `Cargo.toml` (root) | `cloud_objects = { path = "crates/cloud_objects" }` | Workspace dep |
| `app/Cargo.toml` | `cloud_objects = { workspace = true }` | Non-optional |
| `crates/cloud_object_models/Cargo.toml` | `cloud_objects.workspace = true` | Required dep |
| `crates/cloud_object_models/Cargo.toml` | `cloud_objects = { .., features = ["test-util"] }` | Dev dep |

## Why Removal is Blocked

1. **The glob re-export** (`pub use cloud_objects::cloud_object::*`) pipes ~50 types into 200 consumer files
2. **UserUid** is the universal user identifier — used in auth, persistence, telemetry
3. **Owner** is needed anywhere ownership matters — drive, workspaces, environments
4. **SharingAccessLevel** is needed by session sharing and server API
5. **The generic framework** (GenericCloudObject, Serializer, etc.) underpins notebooks, workflows, env vars, and all "string model" objects
6. **cloud_object_models** depends on it — removing cloud_objects means also removing cloud_object_models (40+ additional files)

## Potential Extraction Strategy

If the goal is to eventually break the dependency on the `warpdotdev` fork:

```
Step 1: Extract Layer 0 types to warp_types (trivial — already defined there)
Step 2: Create a new crate `marb_object_types` with Layers 1-2 
Step 3: Move Layer 3 framework into the new crate
Step 4: Repoint app/src/cloud_object/mod.rs glob to new crate
Step 5: Delete crates/cloud_objects/ once no consumer remains
```

Estimated effort: 3-5 days. Risk: medium (many import paths change, compile-time validation catches all breakage).

