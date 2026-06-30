# Issue 7: Prune FeatureFlag Enum (326 -> ~50)

## Summary

Reduce the FeatureFlag enum from 326 variants to ~50, keeping only flags that represent user-toggleable preferences or genuine runtime feature gates for BYOP functionality.

## Problem

The FeatureFlag enum in `crates/warp_features/src/lib.rs` has 326 variants. After Issues 3-6:
- ~80 variants correspond to deleted cloud features (dead — no code references them)
- ~150 variants are "always on" for BYOP (no reason to check them at runtime)
- ~50 variants represent genuinely variable behavior (user can toggle in settings)

The oversized enum wastes memory (326 AtomicBools), makes the feature system untrustworthy (most flags are meaningless), and clutters `enabled_features()` with dead cfg-gates.

## Goals

- Delete all enum variants that are dead (no code references them after Issues 3-6)
- Convert "always on" variants to unconditional code (remove the if-check)
- Keep ~50 variants that represent real user preferences or genuine toggles
- Simplify `enabled_features()` function from 500 lines to ~50

## Non-goals

- Changing user-visible behavior (if a feature was on, it stays on — just without the flag check)
- Adding new features
- Modifying the settings UI

## Behavior

1. All previously-enabled features continue to work (they are now unconditional)
2. User can still toggle preferences in settings (ligatures, rect_selection, etc.)
3. `FeatureFlag` enum has <= 60 variants
4. `enabled_features()` function is < 100 lines
5. `cargo check --bin warp-oss` passes

## Categories of Flags

### DELETE (dead — no code after Issues 3-6)
~80 variants: all cloud, oz, session_sharing, billing flags

### HARDCODE TRUE (remove if-check from callers)
~150 variants: agent_mode, mcp_server, grep_tool, file_tree, etc.
For these, find every `if FeatureFlag::X.is_enabled()` and replace with unconditional execution.

### KEEP (genuine user toggles)
~50 variants: ligatures, rect_selection, kitty_images, zen_mode, etc.
These remain in the enum because users can opt-in/out via settings.