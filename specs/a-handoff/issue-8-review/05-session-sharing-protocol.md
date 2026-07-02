# Requirement 5: Remove `session-sharing-protocol`

## Verdict: Cannot Remove

## Reason summary

`session-sharing-protocol` is deeply embedded across the entire terminal session
sharing subsystem, the agent SDK terminal driver, the AI blocklist controller, the
pane group module, telemetry, and three downstream crates (`cloud_objects`,
`cloud_object_models`, `warp_terminal`). It is referenced from **68 source files**
with **148 individual import lines**. The crate provides the canonical protocol types
for real-time terminal session sharing between multiple participants.

## What this crate provides

`session-sharing-protocol` is an external Git dependency
(`github.com/warpdotdev/session-sharing-protocol.git`, pinned to rev `b30fdd06`)
that defines the wire protocol and domain types for Warp's real-time terminal
session sharing feature. It exposes three top-level modules:

- `common` — shared types used by both sharers and viewers (SessionId,
  ParticipantId, Role, Selection, GridType, Point, WindowSize, etc.)
- `sharer` — types specific to the session host (InitPayload, SessionSourceType,
  SessionRetentionReason, SessionEndedReason, lifetime/quota types)
- `viewer` — types specific to session viewers (FailedToJoinReason,
  RoleUpdatedReason, UpstreamMessage)

## Types used and their consumers

### SessionId
- Source: `session_sharing_protocol::common::SessionId`
- Occurrence count: 24 (most-used type from this crate)
- Used by:
  - `app/src/ai/agent_sdk/driver/terminal.rs` — TerminalDriverEvent::EstablishedSharedSession
  - `app/src/ai/agent_conversations_model/entry.rs` — conversation entry metadata
  - `app/src/ai/ambient_agents/spawn.rs`, `task.rs` — ambient agent task association
  - `app/src/ai/blocklist/history_model.rs` — history event session tracking
  - `app/src/ai/blocklist/local_agent_task_sync_model.rs` — task sync
  - `app/src/infra/server_api/ai.rs` — server API request parameters
  - `app/src/pane_group/mod.rs`, `ambient_pane_restoration.rs`, `child_agent/restoration.rs`
  - `app/src/root_view.rs` — workspace-level session references
  - `app/src/terminal/shared_session/mod.rs`, `manager.rs`
  - `app/src/telemetry/events.rs` (as `SharedSessionId`)

### ParticipantId
- Source: `session_sharing_protocol::common::ParticipantId`
- Used by:
  - `app/src/ai/agent/mod.rs` — agent identity
  - `app/src/ai/blocklist/block/model.rs`, `model_impl.rs` — block attribution
  - `app/src/ai/blocklist/controller.rs`, `controller/shared_session.rs`
  - `app/src/pane_group/mod.rs`
  - `app/src/telemetry/events.rs`
  - `app/src/terminal/block_list_element.rs`, `input.rs`
  - `app/src/terminal/shared_session/participant_avatar_view.rs`,
    `permissions_manager.rs`, `presence_manager.rs`, `role_change_modal/`

### Role
- Source: `session_sharing_protocol::common::Role`
- Used by:
  - `app/src/ai/agent_sdk/driver/terminal.rs` — session role for agent
  - `app/src/pane_group/mod.rs` — pane role tracking
  - `app/src/telemetry/events.rs`
  - `app/src/terminal/input_tests.rs`
  - `app/src/terminal/shared_session/mod.rs`, `participant_avatar_view.rs`,
    `permissions_manager.rs`, `role_change_modal/`
  - `crates/cloud_objects/src/drive/sharing.rs` — drive sharing role mapping

### SessionRetentionReason
- Source: `session_sharing_protocol::sharer::SessionRetentionReason`
- Used by:
  - `app/src/ai/agent_sdk/driver.rs` — decides when to keep session alive
  - `app/src/ai/agent_sdk/driver/terminal.rs` — terminal driver lifecycle
  - `app/src/ai/agent_sdk/driver/terminal_tests.rs`

### ServerConversationToken
- Source: `session_sharing_protocol::common::ServerConversationToken`
- Used by:
  - `app/src/ai/agent/api.rs` — TryFrom conversion to protocol type
  - `app/src/ai/blocklist/controller/shared_session.rs`
  - `app/src/terminal/input.rs`

### Other heavily-used types from `common`
| Type | Usage summary |
|------|---------------|
| `Selection` | 12 occurrences — cursor/selection state in shared sessions |
| `GridType` | 11 occurrences — terminal grid type conversions |
| `BlockId` / `BufferId` | 5+3 — block ID conversions in `warp_terminal` crate |
| `Point` | 4 — cursor point conversions in `warp_terminal` crate |
| `ActivePrompt` | 2 — prompt state broadcast in `terminal_manager.rs` |
| `InputMode` / `InputType` | 1 — input classifier in `blocklist/input_model.rs` |
| `WindowSize` | 1 — terminal resize events |
| `ProfileData` | 1 — user profile conversion in `cloud_object_models` |
| `AgentAttachment` | used in `controller/shared_session.rs` and `terminal/input.rs` |
| `CLIAgentSessionState` | 1 — CLI agent session state in `terminal_manager.rs` |

### Types from `sharer` module
| Type | Usage summary |
|------|---------------|
| `SessionSourceType` | 7 — distinguishes user vs. ambient-agent sessions |
| `SessionRetentionReason` | 3 — agent SDK session lifecycle |
| `SessionEndedReason` | used in telemetry events |
| `RoleUpdateReason` | 2 — role change UI in shared sessions |
| `InitPayload` | 1 — session initialization |
| `AddGuestsResponse`, `RemoveGuestResponse`, `Lifetime`, `QuotaType` | `terminal_manager.rs` |
| `FailedToInitializeSessionReason` | `terminal_manager.rs` |
| `LinkAccessLevelUpdateResponse`, `TeamAccessLevelUpdateResponse` | `terminal_manager.rs` |
| `UpdatePendingUserRoleResponse` | `terminal_manager.rs` |

### Types from `viewer` module
| Type | Usage summary |
|------|---------------|
| `FailedToJoinReason` | 8 — viewer connection error handling |
| `SessionEndedReason` (viewer) | 1 — viewer-side session end |
| `RoleUpdatedReason` | 1 — viewer role update notification |
| `UpstreamMessage` | 1 — viewer network message parsing |

## Complete file listing

68 source files total. Grouped by domain:

**Terminal shared session subsystem (17 files):**
- `app/src/terminal/shared_session/mod.rs`
- `app/src/terminal/shared_session/mod_tests.rs`
- `app/src/terminal/shared_session/manager.rs`
- `app/src/terminal/shared_session/participant_avatar_view.rs`
- `app/src/terminal/shared_session/permissions_manager.rs`
- `app/src/terminal/shared_session/presence_manager.rs`
- `app/src/terminal/shared_session/presence_manager_tests.rs`
- `app/src/terminal/shared_session/role_change_modal/mod.rs`
- `app/src/terminal/shared_session/role_change_modal/sharer_response_body.rs`
- `app/src/terminal/shared_session/role_change_modal/viewer_request_body.rs`
- `app/src/terminal/shared_session/selections.rs`
- `app/src/terminal/shared_session/shared_handlers.rs`
- `app/src/terminal/shared_session/sharer/network.rs`
- `app/src/terminal/shared_session/sharer/network_tests.rs`
- `app/src/terminal/shared_session/viewer/event_loop.rs`
- `app/src/terminal/shared_session/viewer/event_loop_tests.rs`
- `app/src/terminal/shared_session/viewer/network.rs`
- `app/src/terminal/shared_session/viewer/network_tests.rs`
- `app/src/terminal/shared_session/viewer/orchestration_viewer_model.rs`
- `app/src/terminal/shared_session/viewer/terminal_manager.rs`

**Terminal view/model (8 files):**
- `app/src/terminal/view.rs`
- `app/src/terminal/view/action.rs`
- `app/src/terminal/view/ambient_agent/model.rs`
- `app/src/terminal/view/shared_session/adapter.rs`
- `app/src/terminal/view/shared_session/test_utils.rs`
- `app/src/terminal/view/shared_session/view_impl.rs`
- `app/src/terminal/view/shared_session/view_impl_tests.rs`
- `app/src/terminal/view/shared_session/viewer.rs`
- `app/src/terminal/view_tests.rs`

**Terminal core (5 files):**
- `app/src/terminal/alt_screen/alt_screen_element.rs`
- `app/src/terminal/block_list_element.rs`
- `app/src/terminal/input.rs`
- `app/src/terminal/input_tests.rs`
- `app/src/terminal/local_tty/terminal_manager.rs`
- `app/src/terminal/model/terminal_model.rs`

**AI agent SDK (4 files):**
- `app/src/ai/agent_sdk/driver.rs`
- `app/src/ai/agent_sdk/driver/terminal.rs`
- `app/src/ai/agent_sdk/driver/terminal_tests.rs`
- `app/src/ai/agent/api.rs`
- `app/src/ai/agent/mod.rs`

**AI blocklist (7 files):**
- `app/src/ai/blocklist/block/model.rs`
- `app/src/ai/blocklist/block/model/model_impl.rs`
- `app/src/ai/blocklist/controller.rs`
- `app/src/ai/blocklist/controller/shared_session.rs`
- `app/src/ai/blocklist/history_model.rs`
- `app/src/ai/blocklist/input_model.rs`
- `app/src/ai/blocklist/local_agent_task_sync_model.rs`
- `app/src/ai/blocklist/local_agent_task_sync_model_tests.rs`

**Ambient agents (3 files):**
- `app/src/ai/ambient_agents/spawn.rs`
- `app/src/ai/ambient_agents/spawn_tests.rs`
- `app/src/ai/ambient_agents/task.rs`

**Pane group / workspace (7 files):**
- `app/src/pane_group/mod.rs`
- `app/src/pane_group/mod_tests.rs`
- `app/src/pane_group/ambient_pane_restoration.rs`
- `app/src/pane_group/child_agent/restoration.rs`
- `app/src/pane_group/pane/terminal_pane.rs`
- `app/src/workspace/action.rs`
- `app/src/workspace/view.rs`
- `app/src/workspace/view_tests.rs`

**Other app-level (4 files):**
- `app/src/ai/agent_conversations_model/entry.rs`
- `app/src/infra/server_api/ai.rs`
- `app/src/root_view.rs`
- `app/src/telemetry/events.rs`
- `app/src/uri/mod.rs`

**Downstream crates (4 files):**
- `crates/cloud_objects/src/drive/sharing.rs`
- `crates/cloud_object_models/src/user_profile.rs`
- `crates/warp_terminal/src/model/block_id.rs`
- `crates/warp_terminal/src/shared_session.rs`

## Cargo.toml references

| File | Declaration |
|------|-------------|
| `Cargo.toml` (root workspace) | `session-sharing-protocol = { git = "https://github.com/warpdotdev/session-sharing-protocol.git", rev = "b30fdd06..." }` |
| `app/Cargo.toml` | `session-sharing-protocol.workspace = true` |
| `crates/cloud_objects/Cargo.toml` | `session-sharing-protocol.workspace = true` |
| `crates/cloud_object_models/Cargo.toml` | `session-sharing-protocol.workspace = true` |
| `crates/warp_terminal/Cargo.toml` | `session-sharing-protocol.workspace = true` |

## Why the spec assumption was wrong

The spec (line 42) stated:

> Also remove any git dependencies that are ONLY reachable through these
> [cloud_objects, cloud_object_models, warp_graphql_schema]:
> - `session-sharing-protocol` (check: `rg "session.sharing.protocol" **/Cargo.toml`)

The assumption was that `session-sharing-protocol` was only pulled in as a
transitive dependency of `cloud_objects` and `cloud_object_models`. This is
incorrect for three reasons:

1. **Direct dependency from `app/Cargo.toml`**: The main application crate
   declares `session-sharing-protocol.workspace = true` independently. Even if
   `cloud_objects` and `cloud_object_models` were deleted, `app/` would still
   need it.

2. **Direct dependency from `crates/warp_terminal/Cargo.toml`**: The
   `warp_terminal` crate (which is NOT a cloud crate and cannot be removed)
   depends on it for `BlockId`, `BufferId`, and `Point` conversions.

3. **Pervasive use in non-cloud subsystems**: The terminal session sharing module
   (`app/src/terminal/shared_session/`), the agent SDK terminal driver, the AI
   blocklist, ambient agents, pane groups, and telemetry all import types directly
   from this crate. None of these are cloud-specific — they implement the
   real-time collaborative terminal feature and AI agent terminal interaction.

The spec's risk section (line 130) correctly identified this possibility:

> **Risk:** `session-sharing-protocol` is still used by terminal/shared_session/ code
> **Mitigation:** If that module was cfg-gated or deleted in Issues 3-4, the dep
> is safe to remove. If NOT, keep it until the shared_session module is fully deleted.

Since the shared_session module was NOT deleted or cfg-gated in Issues 3-7, the
risk materialized and the dependency must be retained.

## What would need to happen to eventually remove it

1. **Delete the entire `app/src/terminal/shared_session/` subtree** (20+ files)
   — this is the session sharing feature itself.
2. **Remove session sharing from terminal_manager.rs** — strip all sharer/viewer
   initialization, prompt broadcasting, guest management, and session lifecycle
   code.
3. **Remove session sharing from the AI agent SDK** — the `TerminalDriver` uses
   `SessionId` and `SessionRetentionReason` to manage agent sessions.
4. **Remove `ParticipantId` from the blocklist controller** — this ties shared
   session participants to AI conversation blocks.
5. **Remove from telemetry** — event types reference session sharing enums.
6. **Remove from pane_group** — role tracking, session restoration.
7. **Remove from `crates/warp_terminal/`** — `BlockId`/`Point` conversions (or
   inline the types).
8. **Remove from `crates/cloud_objects/` and `crates/cloud_object_models/`** —
   `ProfileData` and `Role` conversions (blocked by those crates also not being
   removable yet).

This is effectively a removal of the entire "Warp Drive" session sharing feature,
which is a major product decision, not a dead-code cleanup task.

