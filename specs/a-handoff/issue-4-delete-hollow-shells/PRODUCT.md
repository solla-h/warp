# Issue 4: Delete Hollow Cloud Module Shells

## Summary

Physically delete the "hollowed-out" cloud modules that were kept as type stubs during Wave 1-4 but now serve no purpose since cloud features no longer compile.

## Problem

Wave 1-4 removed cloud logic but preserved type shells (empty structs + no-op methods) to avoid compilation errors. After Issue 3 flips default features, these shells are dead code that never compiles. They occupy cognitive space, trigger compiler warnings, and make the codebase appear larger than it functionally is.

## Goals

- Delete all hollow cloud module directories and files
- Fix any caller references that break (remove dead call sites)
- Eliminate all `#[allow(dead_code)]` annotations that were suppressing warnings for these shells
- Net reduction of ~2000+ lines of code

## Non-goals

- Deleting `telemetry/` (that is Issue 5)
- Deleting `auth/` (that is Issue 6)
- Deleting crate directories (that is Issue 8)
- Touching anything in `ai/agent_providers/` (BYOP core — do not modify)

## Modules to Delete

| Module | Files | Lines | Content |
|--------|-------|-------|---------|
| `app/src/cloud_object/` | 4 files | 700+ | UpdateManager with 50 no-op methods |
| `app/src/sync_queue.rs` | 1 file | 35 | Empty SyncQueue struct |
| `app/src/drive/` | 12 files | ~500 | DrivePanel stubs |
| `app/src/workspaces/` | 8 files | ~400 | Team workspace + GQL convert stubs |
| `app/src/billing/` | 3 files | ~50 | Creation-denied modal stubs |
| `app/src/server_experiments/` | 3 files | ~200 | Server A/B experiment enum |
| `app/src/pricing/full_impl.rs` | 1 file | ~100 | Stripe pricing stubs |

## Behavior

1. After deletion, `cargo check --bin warp-oss` passes
2. No `cloud_object`, `sync_queue`, `drive`, `workspaces`, `billing`, or `server_experiments` module exists in app/src/
3. All `#[allow(dead_code)]` annotations related to these modules are removed
4. Callers that referenced these modules have their dead branches removed