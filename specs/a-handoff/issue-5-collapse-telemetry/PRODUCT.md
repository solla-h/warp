# Issue 5: Collapse telemetry/ to Single-File No-Op

## Summary

Replace the 12-file telemetry module with a single-file no-op implementation that preserves the same public interface but removes all infrastructure scaffolding.

## Problem

The telemetry module has 12 files maintaining a "hollowed-out" Rudderstack analytics pipeline. Every method already returns `Ok(())` or `0` — it sends nothing anywhere. But the 12-file structure:
- Makes it non-obvious that telemetry is disabled (you have to read multiple files to confirm)
- Maintains types and traits that callers must satisfy even though nothing happens
- The `otel` feature pulls in 5+ OpenTelemetry crates with ~20 transitive deps

## Goals

- Replace 12 files with 1 file (~20-30 lines)
- Preserve the public interface that callers depend on (same struct name, same method signatures)
- Remove `otel` from features and deps (compile time savings)
- Make the "telemetry does nothing" fact immediately obvious

## Non-goals

- Adding local file-based telemetry (future work)
- Changing how callers interact with TelemetryApi (interface stays the same)
- Removing telemetry event call sites (callers keep calling — calls just no-op)

## Behavior

1. All existing callers of `TelemetryApi` compile unchanged
2. `TelemetryApi::flush_events()` returns 0
3. `TelemetryApi::send_telemetry_event()` returns `Ok(())`
4. No data is persisted or transmitted anywhere
5. OpenTelemetry crates are no longer in the dep tree