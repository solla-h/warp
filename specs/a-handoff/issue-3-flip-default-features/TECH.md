# Issue 3: Tech Spec - Flip Default Features

## Context

This is the keystone operation. After this, all `#[cfg(feature = "cloud")]` code stops compiling, enabling physical deletion in subsequent issues.

The feature system has three layers:
1. Cargo features in `app/Cargo.toml` (compile-time)
2. `enabled_features()` in `app/src/features.rs` (bridge)
3. `FeatureFlag::X.is_enabled()` runtime checks (326 AtomicBools)

Changing `default` affects Layer 1, which cascades to Layer 2 (flags not in default won't be in enabled_features set), which cascades to Layer 3 (is_enabled() returns false).

## Relevant code

- `app/Cargo.toml` lines 458-686 — [features] section with ~130 default flags
- `app/src/features.rs` lines 16-503 — `enabled_features()` function with ~170 cfg-gated inserts
- `app/src/lib.rs` lines 1242-1257 — `#[cfg(feature = "local-only")]` gate for ServiceProvider
- `app/src/bin/oss.rs` — Binary entry point using `ChannelConfig::local_only()`
- `crates/warp_features/src/lib.rs` — FeatureFlag enum definition (326 variants)

## Current state

The `default` feature list has ~130 entries including:
```toml
default = ["otel", "cloud", "aws-bedrock", "agent_mode", "cloud_ui",
    "viewing_shared_sessions", "session_sharing_acls", "usage_based_pricing",
    "loginless_conversion", "bundled_workflows", ...]
```

The `local-only` feature is defined as just `["skip_login"]` and is NOT in default.

## Proposed changes

### 1. Rewrite the default feature list

Replace the entire `default = [...]` with a minimal BYOP set (~50 flags). The exact list should be determined by:
1. Start with `local-only` (which includes `skip_login`)
2. Add all AI/Agent flags that have runtime `.is_enabled()` checks
3. Add all terminal enhancement flags
4. Add all editor/code-review flags
5. EXCLUDE all cloud/oz/billing/session/otel/aws flags

```toml
default = [
    "local-only",
    # AI Core
    "agent_mode", "agent_mode_primary_xml", "agent_mode_pre_plan_xml",
    "mcp_server", "grep_tool", "file_retrieval_tools",
    "web_search_ui", "web_fetch_ui", "image_as_context",
    "ai_rules", "am_workflows", "custom_inference_endpoints", "solo_user_byok",
    "render_agent_mode_output_markdown", "agent_decides_command_execution",
    "bundled_skills", "list_skills", "ask_user_question", "agent_view",
    # Terminal
    "ligatures", "rect_selection", "kitty_images", "kitty_keyboard_protocol",
    "minimalist_ui", "full_screen_zen_mode", "shell_selector",
    "settings_file", "global_search", "ui_zoom", "default_waterfall_mode",
    # Editor
    "code_find_replace", "tabbed_editor_view", "file_tree", "vim_code_editor",
    "revert_diff_hunk", "inline_code_review", "linked_code_blocks",
    "code_review_save_changes", "selection_as_context",
    # UI
    "richtext_multiselect", "command_palette_file_search",
    "undo_closed_panes", "bundled_workflows",
]
```

### 2. Delete dead feature flag DEFINITIONS

Remove from [features] section entirely (not just from default):
- `cloud = [...]`
- `cloud_ui = [...]`
- All `viewing_shared_sessions`, `session_sharing_*`, `shared_with_me` etc.
- All `hoa_*`, `oz_*`, `handoff_*` flags
- `usage_based_pricing`, `billing_*`, `loginless_conversion`
- `otel` (will break opentelemetry dep — that dep removal is Issue 8)

**Important:** Some removed flags may cause `#[cfg(feature = "xxx")]` in `features.rs` to reference undefined features. That is OK — the cfg simply won't match, and the FeatureFlag variant won't be inserted. As long as the FLAG_STATES array size stays consistent with the enum, it compiles.

### 3. Handle compile errors iteratively

After changing default, run `cargo check`. Expected errors:
- Modules gated by `#[cfg(not(feature = "local-only"))]` that reference removed features
- Code inside `#[cfg(feature = "cloud")]` blocks that uses types from cloud crates

For each error:
- If the code is inside a `#[cfg(feature = "cloud")]` gate — it won't compile anyway (feature removed). The gate protects it. No action needed.
- If the code is NOT gated — either add `#[cfg(feature = "cloud")]` gate, or if it's dead code that will be deleted in Issue 4, add a temporary `#[allow(dead_code)]` (document which ones are temporary).

### 4. Verify feature flag runtime behavior

After compile passes, verify that essential FeatureFlags are still enabled:
- `FeatureFlag::AgentMode.is_enabled()` should be true
- `FeatureFlag::McpServer.is_enabled()` should be true
- `FeatureFlag::Ligatures.is_enabled()` should be true
- `FeatureFlag::ViewingSharedSessions.is_enabled()` should be FALSE

## Testing and validation

### Compile verification
```powershell
cargo check --bin warp-oss  # Must pass with 0 errors
```

### Dep tree measurement
```powershell
$before = (cargo tree --bin warp-oss 2>$null | Measure-Object -Line).Lines
# After change:
$after = (cargo tree --bin warp-oss 2>$null | Measure-Object -Line).Lines
# Assert: $after < $before * 0.7 (at least 30% reduction)
```

### Smoke test
```powershell
cargo run --bin warp-oss -- --smoke-test
```

### Search for broken cfg references
```powershell
# Find any cfg references to features that no longer exist
rg '#\[cfg\(feature = "(cloud|otel|aws-bedrock|cloud_ui)"' app/src/
# These should be in dead code paths (inside other cfg gates) or need fixing
```

### Acceptance criteria

- [ ] `cargo check --bin warp-oss` passes with 0 errors
- [ ] `cargo run --bin warp-oss -- --smoke-test` passes
- [ ] dep tree line count reduced by >30%
- [ ] No `cloud`, `cloud_ui`, `otel`, `aws-bedrock` in default features
- [ ] `local-only` IS in default features
- [ ] All AI/terminal/editor features still compile and are runtime-enabled
- [ ] Commit: "feat: flip default features to BYOP-only minimum"

## Risks and mitigations

- **Risk:** Missing a feature flag that controls essential UI
  - **Mitigation:** If after flipping, a UI element is missing that should be there, add its flag back to default. The smoke test should catch obvious failures.

- **Risk:** `otel` removal causes compile error in dep resolution
  - **Mitigation:** If opentelemetry deps are non-optional in app/Cargo.toml, make them optional first: `opentelemetry = { ..., optional = true }`

- **Risk:** `cloud` feature removal causes transitive compile errors
  - **Mitigation:** `cloud_objects` is currently a HARD dep (not optional). If removing `cloud` feature causes cloud_objects to fail, you may need to first make it optional: `cloud_objects = { workspace = true, optional = true }` and gate its usage with `#[cfg(feature = "cloud")]`.