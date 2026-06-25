# Wave 0: Smoke Test Infrastructure
# Issue: https://github.com/solla-h/warp/issues/2
# Copy this entire file content as your prompt in a new session with /implement

/implement https://github.com/solla-h/warp/issues/2

## Context

This is a Rust project — a hard fork of Warp Terminal called "Marb". It's a BYOP (Bring Your Own Provider) terminal that uses a user-configured LLM endpoint instead of Warp's cloud.

Key files to read FIRST:
- `CONTEXT.md` — project glossary, architecture, current state
- `PLAN.md` — execution plan (you are implementing Phase C)
- `app/src/lib.rs` — main entry point, `LaunchMode` enum, `run()` and `run_internal()` functions
- `app/src/bin/oss.rs` — binary entry point
- `crates/warp_cli/src/lib.rs` — CLI argument parsing (clap-based `Args` struct)

## What to build

Add `--smoke-test` CLI flag to `warp-oss` that:
1. Starts the app in headless mode (full singleton initialization, no GPU window)
2. Programmatically injects one user message ("ping") into the BYOP conversation path
3. Waits for the BYOP stream to complete
4. Checks log for `[byop] stream stats: start=1 chunks=N` (N>0)
5. Exits with code 0 (success) or 1 (failure/timeout)

## Architecture guidance

- Add `LaunchMode::SmokeTest` to the existing enum in `app/src/lib.rs`
- `is_headless()` must return `true` for SmokeTest (uses `AppBuilder::new_headless()`)
- The message injection path: look at how `BlocklistAIController.send_user_query_in_conversation()` works
- The BYOP endpoint is configured in user settings — the app reads it from `CustomEndpoint` storage
- On Windows, attach console via `warp_util::windows::attach_to_parent_console()` for log visibility
- Timeout: 30 seconds from message send to stream completion

## BYOP endpoint (for testing)
- URL: `https://ds-api.xnurta.com/`
- API Type: Anthropic
- Model: claude-opus-4-6

## Wrapper script

Create `scripts/smoke-test.ps1` that:
1. Runs `cargo build --release -p warp`
2. Launches `target/release/warp-oss.exe --smoke-test`
3. Reports pass/fail based on exit code

## Verification

- `cargo check -p warp` — 0 errors
- `warp-oss --smoke-test` — exits 0 with a configured endpoint
- No existing tests broken

## Constraints

- Do NOT modify the BYOP data flow (agent_providers/chat_stream.rs)
- Do NOT add external dependencies
- Do NOT create mock servers — use the real endpoint
- Keep the smoke test code minimal — it's infrastructure, not a feature
