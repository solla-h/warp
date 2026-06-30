# Issue 5: Tech Spec - Collapse telemetry/

## Context

The telemetry module at `app/src/telemetry/` currently has 12 files. All API methods return no-op values. The module exists purely to satisfy caller references. This issue collapses it to a single file while preserving the public API surface.

## Relevant code

- `app/src/telemetry/mod.rs` — TelemetryApi struct, method stubs
- `app/src/telemetry/collector.rs` — Event collector (no-op)
- `app/src/telemetry/events.rs` — Event type definitions
- `app/src/telemetry/context.rs` — Telemetry context
- `app/src/telemetry/macros.rs` — Telemetry macros
- Plus ~7 more files in the module
- `app/Cargo.toml` — `otel` feature with opentelemetry deps

## Proposed changes

### 1. Identify the public API surface

Before deleting, scan all callers:
```powershell
rg "use crate::telemetry" app/src/ --type rust
rg "TelemetryApi" app/src/ --type rust
```

Document every method/struct/type that callers reference.

### 2. Write single-file replacement

Create `app/src/telemetry.rs` (single file, not directory):

```rust
pub struct TelemetryApi;

impl TelemetryApi {
    pub fn new() -> Self { Self }
    
    pub fn flush_events(&self) -> usize { 0 }
    
    pub fn send_telemetry_event(&self, _event: impl std::any::Any) -> anyhow::Result<()> {
        Ok(())
    }
    
    pub fn flush_persisted_events_to_rudder(&self) -> anyhow::Result<()> {
        Ok(())
    }
    
    pub fn flush_and_persist_events(&self) -> anyhow::Result<()> {
        Ok(())
    }
}
```

Note: The EXACT interface depends on what callers actually use. Step 1 determines the precise set of methods needed.

### 3. Delete the module directory

```powershell
Remove-Item -Recurse app/src/telemetry/
# The new app/src/telemetry.rs replaces it
```

In `app/src/lib.rs`, change:
```rust
// FROM: pub mod telemetry;  (which resolves to telemetry/mod.rs)
// TO:   pub mod telemetry;  (which now resolves to telemetry.rs)
```

No change needed in lib.rs — Rust resolves `mod telemetry` to either `telemetry.rs` OR `telemetry/mod.rs`.

### 4. Remove otel feature and deps

In `app/Cargo.toml`:
- Remove `otel` from features definition
- Remove or make optional: `opentelemetry`, `opentelemetry-http`, `opentelemetry-otlp`, `opentelemetry_sdk`, `tracing-opentelemetry`

### 5. Fix compile errors

Run `cargo check`. Callers that referenced specific types from the old telemetry module (e.g., event enums, collector types) will fail. For each:
- If the caller was passing a specific event type → change to pass a generic or remove the call
- If the caller was importing a trait → remove the import (no-op doesn't need traits)

## Testing and validation

### TDD approach: behavior invariance

The "test" for this refactoring is:
1. Before: all telemetry methods return no-op values
2. After: all telemetry methods still return no-op values
3. No caller behavior changes

### Verification steps

```powershell
# Compile
cargo check --bin warp-oss

# Verify no opentelemetry in dep tree (after removing otel feature)
cargo tree --bin warp-oss | Select-String "opentelemetry"
# Should return 0 results

# Verify callers still work
cargo test -p warp --lib
```

### Acceptance criteria

- [ ] `app/src/telemetry/` directory deleted
- [ ] `app/src/telemetry.rs` exists (single file, <50 lines)
- [ ] All callers compile unchanged
- [ ] `otel` feature removed from app/Cargo.toml
- [ ] opentelemetry not in dep tree output
- [ ] `cargo check --bin warp-oss` passes
- [ ] `cargo test -p warp --lib` passes
- [ ] Commit: "refactor: collapse telemetry to single-file no-op"

## Risks

- **Risk:** Some caller imports a specific type from deep within the telemetry module
  - **Mitigation:** If the type is needed by signature (e.g., an enum for event categories), add a minimal type alias or empty enum to the new file

- **Risk:** Macro definitions in telemetry/macros.rs are used across codebase
  - **Mitigation:** Keep the macros as no-op macros in the new single file if widely used