# Issue 3: Flip Default Features to BYOP-Only

## Summary

Rewrite the `default` feature list in `app/Cargo.toml` from ~130 cloud-inclusive flags to ~50 BYOP-essential flags. This is the keystone operation that makes all subsequent slimming possible.

## Problem

The current `default` feature list includes `cloud`, `cloud_ui`, `otel`, `aws-bedrock`, `viewing_shared_sessions`, `usage_based_pricing`, and ~100 other flags. Every `cargo build` compiles the entire cloud dependency subtree (~2000 extra dep lines), even though:
- The runtime is already forced to local-only mode via `ChannelConfig::local_only()`
- All cloud API methods are no-op stubs
- No user can access any cloud feature

This wastes compilation time, inflates binary size, and maintains the fiction of a "dual-mode" build that doesn't exist.

## Goals

- Redefine `default` to contain only flags needed for BYOP AI terminal functionality
- Remove dead feature flag DEFINITIONS (not just remove from default — delete the declaration entirely)
- Achieve `cargo check` passing with the new minimal default
- Reduce dep tree size by ~30-40%

## Non-goals

- Physical deletion of the code behind removed features (that is Issues 4-6)
- Pruning the FeatureFlag enum itself (that is Issue 7)
- Removing crate dependencies from Cargo.toml (that is Issue 8)

## Behavior

1. `cargo build --bin warp-oss` compiles successfully with new default
2. `cargo run --bin warp-oss -- --smoke-test` passes
3. Agent chat with BYOP provider works (websearch, webfetch, todowrite tools)
4. Terminal features work: ligatures, kitty images, rectangular selection, zen mode
5. Editor features work: file tree, find/replace, vim mode, code review
6. Cloud UI elements (Share button, Drive panel, Upgrade prompts) do NOT appear
7. dep tree line count decreases by >30%

## Feature Flags to KEEP in new default

### AI Core (must have for BYOP)
- agent_mode, agent_mode_primary_xml, agent_mode_pre_plan_xml
- mcp_server, grep_tool, file_retrieval_tools
- web_search_ui, web_fetch_ui, image_as_context
- ai_rules, am_workflows, custom_inference_endpoints, solo_user_byok
- render_agent_mode_output_markdown, agent_decides_command_execution
- bundled_skills, list_skills, ask_user_question, agent_view

### Terminal Enhancement (pure local)
- ligatures, rect_selection, kitty_images, kitty_keyboard_protocol
- minimalist_ui, full_screen_zen_mode, shell_selector
- settings_file, global_search, ui_zoom, default_waterfall_mode

### Editor / Code Review (pure local)
- code_find_replace, tabbed_editor_view, file_tree, vim_code_editor
- revert_diff_hunk, inline_code_review, linked_code_blocks
- code_review_save_changes, selection_as_context

### UI Quality (pure local)
- richtext_multiselect, command_palette_file_search
- undo_closed_panes, bundled_workflows

## Feature Flags to REMOVE from default (and DELETE definition)

### Cloud infrastructure
- cloud, cloud_ui (and all 12 sub-flags)
- otel, aws-bedrock

### Session sharing
- viewing_shared_sessions, creating_shared_sessions
- session_sharing, session_sharing_acls
- shared_with_me, agent_shared_sessions

### Billing / Analytics
- usage_based_pricing, billing_and_usage_page_v2
- loginless_conversion, global_ai_analytics_collection

### Hosted Agent (Oz)
- All hoa_* flags, orchestration_viewer_streamer
- oz_platform_skills, oz_identity_federation, oz_launch_modal
- oz_handoff, handoff_local_cloud, handoff_cloud_cloud

### Cloud Mode
- cloud_mode, cloud_mode_from_local_session
- cloud_mode_image_context, cloud_mode_setup_v2, cloud_mode_input_v2