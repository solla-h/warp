# Issue 8: Remove Dead Crate Dependencies and Patches

## Summary

Remove crate directories, workspace dependency declarations, and [patch] entries that are no longer reachable from the BYOP default build.

## Problem

After Issues 3-7, several crates are completely unreferenced:
- `crates/cloud_objects/` — Cloud object substrate
- `crates/cloud_object_models/` — Concrete cloud object models
- `crates/warp_graphql_schema/` — 2-line cynic schema marker
- Workspace [patch.crates-io] entries for tink-core, tink-proto, tink-hybrid
- Workspace [patch] section for session-sharing-protocol
- Workspace dependency declarations for deleted crates

These add noise to Cargo.toml, slow resolution, and maintain dependencies on warpdotdev forks.

## Goals

- Delete dead crate directories
- Remove dead [patch.crates-io] entries
- Remove dead workspace dependency declarations
- Clean Cargo.lock (regenerate after removals)
- Verify dep tree shrinks further

## Non-goals

- Removing `remote_server` crate (it is for SSH remote dev, not cloud)
- Removing `lib/rust-genai` (it is the BYOP backbone)
- Removing `crates/mcp/` or `crates/mcp_types/` (needed for Agent tools)

## Behavior

1. `cargo check --bin warp-oss` passes
2. `cargo tree --bin warp-oss` shows no tink, session-sharing-protocol, or cynic
3. No `crates/cloud_objects/`, `crates/cloud_object_models/`, or `crates/warp_graphql_schema/` directory exists
4. Cargo.lock is clean (no stale entries for removed crates)