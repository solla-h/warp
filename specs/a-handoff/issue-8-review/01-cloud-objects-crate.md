# Requirement 1: Delete `crates/cloud_objects/`

## Verdict: Cannot Remove

## Reason summary

The `cloud_objects` crate is the foundational substrate for Warp's cloud sync system. It defines core identity types (`UserUid`, `Owner`), object metadata structures, sync status enums, sharing/permissions models, drive primitives, and the generic cloud object framework used by every cloud-synced feature (notebooks, workflows, env vars, AI profiles, MCP servers). It is imported by 22+ files in `app/src/` and is a direct dependency of `cloud_object_models`. Most types originate here and are re-exported through `app/src/cloud_object/mod.rs` via a glob `pub use cloud_objects::cloud_object::*`.

## Types exported and their consumers

### UserUid
- Defined in: `warp_types::user_uid` (re-exported through `cloud_objects::auth::UserUid`)
- Used by:
  - `app/src/auth/mod.rs` — re-exports as `pub use cloud_objects::UserUid` (app-wide identity type)
  - `app/src/auth/auth_state.rs` — user authentication state management
  - `app/src/auth/user/mod.rs` — user model, also re-exports `TEST_USER_EMAIL`, `TEST_USER_UID`
  - `app/src/auth/user/persistence.rs` — persisting user identity
  - `app/src/workspaces/user_workspaces.rs` — workspace ownership checks
- Role: The canonical user identifier type throughout the app

### Owner
- Defined in: `crates/cloud_objects/src/cloud_object/mod.rs` (enum: `User { user_uid }` | `Team { team_uid }`)
- Used by:
  - `app/src/ai/cloud_environments/mod.rs` — cloud environment ownership
  - `app/src/drive/mod.rs` — drive ownership determination
  - `app/src/workspaces/user_workspaces.rs` — workspace owner resolution
  - `app/src/cloud_object/mod.rs` — glob re-export exposes to all cloud object consumers
- Role: Discriminates personal vs team ownership for any cloud object

### AccessLevel (SharingAccessLevel)
- Defined in: `crates/cloud_objects/src/drive/sharing.rs` (enum: `View` | `Edit` | `Full`)
- Used by:
  - `app/src/ai/execution_profiles/profiles_tests.rs` — test assertions on profile access
  - `app/src/terminal/view/shared_session/cloud_conversation_continuation_tests.rs` — test assertions
  - `app/src/terminal/shared_session/permissions_manager.rs` — session permission checks
  - `app/src/infra/server_api/object.rs` — server API calls for sharing
  - `crates/cloud_object_models/src/client_types.rs` — client-side permission traits
- Role: Permission level for shared objects and link sharing

### GenericStringModel / Serializer
- Defined in: `crates/cloud_objects/src/cloud_object/generic_string_model.rs`
- Used by:
  - `app/src/cloud_object/model/generic_string_model.rs` — re-exports and wraps for app use
- Role: Generic serialization framework for cloud objects stored as JSON/string blobs

### CloudObjectUpsertParams
- Defined in: `crates/cloud_objects/src/cloud_object/generic_cloud_object.rs`
- Used by:
  - `app/src/cloud_object/model/generic_string_model.rs` — creating upsert requests
- Role: Parameters needed to upsert a cloud object to the server

### GenericStringObjectId / FolderId
- Defined in: `crates/cloud_objects/src/ids.rs`
- Used by:
  - `app/src/ids.rs` — re-exports `GenericStringObjectId` and other ID types
  - `app/src/drive/folders/mod.rs` — re-exports `FolderId`
  - `app/src/infra/server_api/object.rs` — server API ID parameters
  - `app/src/cloud_object/model/generic_string_model.rs` — re-exports
- Role: Typed wrappers around `ServerId` for folder and generic-string objects

### CloudObjectTypeAndId
- Defined in: `crates/cloud_objects/src/drive/mod.rs`
- Used by:
  - `app/src/drive/items.rs` — drive item identification
  - `app/src/drive/mod.rs` — re-exports for drive module
- Role: Enum combining object type + ID for passing between drive actions

### UpdatedObjectInput / ObjectsToUpdate
- Defined in: `crates/cloud_objects/src/cloud_object/update.rs`
- Used by:
  - `app/src/cloud_object/mod.rs` — fetching updated objects from server
- Role: Input structs for server sync/polling operations

### SerializedModel
- Defined in: `crates/cloud_objects/src/cloud_object/mod.rs`
- Used by:
  - `app/src/cloud_object/model/json_model.rs` — JSON model serialization
  - `app/src/sync_queue.rs` — re-exports for sync queue operations
  - `app/src/infra/server_api/object.rs` — server API payloads
- Role: Newtype wrapping a serialized string representation of a model

### ServerPermissions / CloudObjectPermissions / CloudObjectMetadata
- Defined in: `crates/cloud_objects/src/cloud_object/mod.rs`
- Used by:
  - `app/src/drive/index_tests.rs` — test fixtures
  - `app/src/cloud_object/mod.rs` — glob re-export
- Role: Full permission and metadata state for synced cloud objects

### Other re-exported types (via glob `pub use cloud_objects::cloud_object::*`)
- `ObjectType`, `ObjectIdType`, `GenericStringObjectFormat`, `JsonObjectType`
- `GenericCloudObject<K, M>`, `GenericServerObject`
- `Revision`, `CloudObjectSyncStatus`, `CloudObjectStatuses`
- `ServerMetadata`, `ServerObjectContainer`, `ServerGuestSubject`
- `CreateObjectRequest`, `BulkCreateGenericStringObjectsRequest`
- `CloudObjectEventEntrypoint`, `RevisionAndLastEditor`
- All consumed indirectly through `app/src/cloud_object/mod.rs` line 44: `pub use cloud_objects::cloud_object::*`

## Complete file listing

| File path | Imports from `cloud_objects` | Role in app |
|-----------|------------------------------|-------------|
| `app/src/auth/mod.rs` | `UserUid` | Re-exports UserUid as the app-wide user identity type |
| `app/src/auth/auth_state.rs` | `UserUid` | Manages authenticated user state |
| `app/src/auth/user/mod.rs` | `UserUid`, `TEST_USER_EMAIL`, `TEST_USER_UID` | User model and test constants |
| `app/src/auth/user/persistence.rs` | `UserUid` | Persists user identity to storage |
| `app/src/cloud_object/mod.rs` | `UpdatedObjectInput`, `cloud_object::*` (glob) | Central cloud object module; re-exports everything |
| `app/src/cloud_object/model/generic_string_model.rs` | `CloudObjectUpsertParams`, `GenericStringModel`, `Serializer`, `GenericStringObjectId` | Generic string model implementation |
| `app/src/cloud_object/model/json_model.rs` | `GenericStringObjectFormat`, `SerializedModel` | JSON-based cloud object model |
| `app/src/ids.rs` | `GenericStringObjectId`, full ids re-export | App-level ID type definitions |
| `app/src/drive/mod.rs` | `CloudObjectTypeAndId`, `Owner` | Drive module root |
| `app/src/drive/items.rs` | `CloudObjectTypeAndId` | Drive item management |
| `app/src/drive/folders/mod.rs` | `FolderId` | Folder operations |
| `app/src/drive/index_tests.rs` | `ServerPermissions` | Drive index test fixtures |
| `app/src/sync_queue.rs` | `SerializedModel` | Cloud object sync queue |
| `app/src/infra/server_api/object.rs` | `SharingAccessLevel`, `FolderId`, `ServerId`, `SerializedModel` | Server API layer for objects |
| `app/src/ai/cloud_environments/mod.rs` | `Owner` | AI cloud environment ownership |
| `app/src/ai/execution_profiles/profiles_tests.rs` | `AccessLevel` | AI execution profile tests |
| `app/src/workspaces/user_workspaces.rs` | `Owner` | Workspace ownership |
| `app/src/terminal/shared_session/permissions_manager.rs` | `SharingAccessLevel` | Session permission management |
| `app/src/terminal/view/shared_session/cloud_conversation_continuation_tests.rs` | `AccessLevel` | Shared session tests |

## Dependency chain

| Cargo.toml | Reference | Purpose |
|------------|-----------|---------|
| `Cargo.toml` (root) | `cloud_objects = { path = "crates/cloud_objects" }` | Workspace dependency declaration |
| `app/Cargo.toml` | `cloud_objects = { workspace = true }` | App depends on it directly |
| `app/Cargo.toml` | feature `enforce_revisions_to_cloud_objects = []` | Feature flag for revision enforcement |
| `app/Cargo.toml` | feature `cloud_object_initial_load = ["enforce_revisions_to_cloud_objects"]` | Feature composition |
| `app/Cargo.toml` | feature `personal_cloud_objects = []` | Feature flag for personal objects |
| `crates/cloud_object_models/Cargo.toml` | `cloud_objects.workspace = true` | Models crate depends on it |
| `crates/cloud_object_models/Cargo.toml` | feature `test-util = ["cloud_objects/test-util"]` | Test utility feature forwarding |
| `crates/cloud_object_models/Cargo.toml` | dev-dep `cloud_objects = { workspace = true, features = ["test-util"] }` | Test mocks |

Note: `cloud_objects` itself depends on `warp_types`, `warp_core`, `warpui_core`, `pathfinder_geometry`, `chrono`, `serde`, `derivative`, and `session_sharing_protocol`.

## What would need to happen to eventually remove it

1. **Extract `UserUid` re-export path**: `UserUid` already lives in `warp_types`. All consumers could import directly from `warp_types` (or through `app/src/auth/mod.rs`). Same for `TEST_USER_EMAIL` and `TEST_USER_UID`.

2. **Move identity/ownership types to `warp_types`**: `Owner`, `ServerObjectContainer`, and related enums are pure data types that could live in `warp_types` alongside `UserUid` and `ServerId`.

3. **Move ID wrapper types**: `FolderId`, `GenericStringObjectId`, `ServerIdExt`, `SyncIdExt`, and `ServerIdAndType` are thin wrappers over `warp_types` primitives. These could move to `warp_types` or a new `cloud_ids` micro-crate.

4. **Move cloud object metadata/sync types**: `CloudObjectMetadata`, `CloudObjectPermissions`, `CloudObjectSyncStatus`, `Revision`, `SerializedModel`, and the full permissions hierarchy are tightly coupled to the UI (they import `warpui_core`, `warp_core::ui`, etc.). These would need a new crate like `cloud_object_core` that depends on the UI framework, or the UI rendering logic (`render_icon`) would need to be extracted out.

5. **Move the generic object framework**: `GenericCloudObject<K,M>`, `CloudObjectUpsertParams`, `GenericStringModel`, `Serializer` trait, creation/update types form the backbone of the sync system. They'd need their own crate or could merge into `cloud_object_models`.

6. **Move drive primitives**: `CloudObjectTypeAndId` and `sharing` module (`SharingAccessLevel`, `Subject`, etc.) could be split into a `cloud_drive_types` crate.

7. **Update all import paths**: After extraction, update 22+ files in `app/src/` and `crates/cloud_object_models/` to point to new locations.

8. **Remove the glob re-export**: `app/src/cloud_object/mod.rs` line 44 (`pub use cloud_objects::cloud_object::*`) means any consumer of `crate::cloud_object::*` gets everything. A removal would need to audit all transitive users.

**Estimated effort**: Large (2-3 days). The crate is not dead — it is deeply woven into the cloud sync, drive, auth, and AI subsystems. The TECH.md spec assumed Issues 3-7 would have already removed these usages, but they have not been completed.
