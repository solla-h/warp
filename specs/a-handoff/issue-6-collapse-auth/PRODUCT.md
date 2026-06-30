# Issue 6: Collapse auth/ to BYOP-Only

## Summary

Reduce the 18-file authentication module to a minimal BYOP provider-check module. The current auth module manages login ceremonies, SSO flows, and cloud state teardown — none of which exist in a BYOP terminal.

## Problem

The auth module (18 files, ~230 lines in mod.rs alone) manages:
- Login slides and onboarding
- SSO (Single Sign-On) flow
- Credential storage (Firebase tokens)
- Cloud state teardown on logout (WorkflowManager, TeamUpdateManager, NotebookManager cleanup)
- oauth2 dependency chain (~15 transitive crates)

For BYOP, the only relevant question is: "Has the user configured an API key?" This is already answered in `app/src/settings/ai.rs`. The auth module is a huge interface for a trivial check.

## Goals

- Reduce auth from 18 files to 2-3 files maximum
- Remove SSO, login slides, credential storage, cloud teardown code
- Preserve the `AuthState` / `AuthManager` interface that other modules reference (with simplified implementation)
- Remove oauth2 dependency from default build

## Non-goals

- Changing how BYOP API keys are stored (stays in settings/ai.rs)
- Adding new auth mechanisms
- Modifying the settings UI for API key configuration

## Behavior

1. App starts without any login prompt or auth ceremony
2. `AuthState` reports `is_logged_in() == true` always (skip_login is unconditional)
3. `User::test()` is the default user (as currently implemented by skip_login)
4. No oauth2 crate in dep tree
5. All modules that reference AuthState/AuthManager still compile