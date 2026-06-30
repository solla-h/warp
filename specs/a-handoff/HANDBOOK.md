# Marb Slimming Execution Handbook

> Date: 2026-06-30 | Branch: marb (f5e8968b) | Repo: https://github.com/solla-h/warp.git

## Project Context

**Marb** is a hard fork of Warp Terminal. Goal: pure BYOP (Bring Your Own Provider) AI terminal.
62 commits of cloud code deletion done (Wave 0-4). Still has dead code, dead deps, hollow modules.

### Key Decisions

1. **Hard fork** - No upstream merge compatibility. Can physically delete anything.
2. **BYOP-only** - Only AI path is user-provided API key (Anthropic/OpenAI/Ollama).
3. **Single build config** - After flip, only one default build. No `--features cloud` option.

### Current Runtime State

- `oss.rs` binary uses `ChannelConfig::local_only()` forcing local mode
- All server URLs point to `localhost:0` (unreachable)
- `ServiceProvider::new_for_local_only()` constructs stub clients
- BYOP Agent chat, websearch, webfetch, todowrite tools all work

---

## Execution Order

```
Issue 1 (independent) ────────────────────────────────→
Issue 2 (independent) ────────────────────────────────→

Issue 3 ──→ Issue 4 ──→ Issue 7 ──→ Issue 8
              |
Issue 5 ─────┘ (depends on Issue 3)
Issue 6 ─────┘ (depends on Issue 3)
```

### Dependency Matrix

| Issue | Title | Blocked by | Est. |
|-------|-------|-----------|------|
| 1 | Physical cleanup (PDB, yarn, sqlite, docs) | None | 30m |
| 2 | Purge dead dirs (specs/, .agents/skills/, etc.) | None | 30m |
| 3 | **Flip default features to BYOP-only** | None | 2-4h |
| 4 | Delete hollow cloud module shells | #3 | 4-6h |
| 5 | Collapse telemetry/ to single-file no-op | #3 | 2-3h |
| 6 | Collapse auth/ to BYOP-only | #3 | 4-8h |
| 7 | Prune FeatureFlag enum (326->~50) | #4,#5,#6 | 1-2d |
| 8 | Remove dead crate deps and patches | #7 | 4-6h |

### Recommended Batches

**Batch A (parallel, no deps):** Issue 1 + Issue 2
**Batch B (keystone):** Issue 3
**Batch C (parallel, all depend on #3):** Issue 4 + Issue 5 + Issue 6
**Batch D (consolidation):** Issue 7
**Batch E (finalization):** Issue 8

---

## Critical Technical Knowledge

### Feature Flag Three-Layer Architecture

```
Layer 1: Cargo features (compile-time)
  app/Cargo.toml [features] section
  #[cfg(feature = "xxx")] decides if code compiles

Layer 2: enabled_features() (initialization)
  app/src/features.rs - enabled_features() function
  Each Cargo feature maps 1:1 to FeatureFlag enum variant
  Calls flag.set_enabled(true) on global AtomicBool

Layer 3: FeatureFlag::X.is_enabled() (runtime)
  crates/warp_features/src/lib.rs
  326 AtomicBools control UI/logic branches
  Priority: test override > user preference > global state > false
```

### Key File Paths

| Path | Role |
|------|------|
| `app/Cargo.toml` | Main app deps + features definition |
| `app/src/features.rs` | Cargo feature -> FeatureFlag bridge |
| `app/src/lib.rs` | Module declarations + init logic (cfg-gates here) |
| `app/src/bin/oss.rs` | OSS binary entry point |
| `crates/warp_features/src/lib.rs` | FeatureFlag enum (326 variants) |
| `Cargo.toml` (root) | Workspace def + [patch] entries |
| `app/src/infra/server_api.rs` | ServiceProvider + local-only stub |
| `app/src/ai/agent_providers/chat_stream.rs` | BYOP core stream (DO NOT TOUCH) |

### Verification Commands

Run after every change:

```powershell
# Quick compile check
cargo check --bin warp-oss

# Full build
cargo build --bin warp-oss

# Smoke test
cargo run --bin warp-oss -- --smoke-test

# Unit tests
cargo test -p warp --lib

# Dep tree size (track slimming progress)
(cargo tree --bin warp-oss | Measure-Object -Line).Lines
```

### Environment Notes

- Windows + PowerShell 7
- Files use LF line endings (not CRLF)
- Use [System.IO.File]::ReadAllText/WriteAllText for reliable file ops
- $content.Replace("multi\nline",...) may silently fail on LF files
- Release build: ~20-25 min. Incremental check: 30-60s. Plan accordingly.

---

## TDD Strategy for Deletion Work

Most issues are DELETION operations. TDD strategy differs from feature work:

### For deletion operations (Issues 1-4, 8):
1. **Compile gate** - `cargo check` must pass after deletion (this IS the test)
2. **Dep tree shrinkage** - `cargo tree` line count must decrease
3. **Smoke test** - `--smoke-test` verifies app can start
4. **Search assertion** - `rg "deleted_symbol"` returns 0 results

### For refactoring operations (Issues 5, 6, 7):
1. **Behavior invariance** - Same inputs produce same outputs
2. **Interface testing** - Test public behavior, not implementation
3. **Regression protection** - Existing tests still pass

### Universal Acceptance Checklist (every issue):
- [ ] `cargo check --bin warp-oss` passes (0 errors)
- [ ] `cargo test -p warp --lib` passes
- [ ] No new `#[allow(dead_code)]` added
- [ ] Git status clean (all changes committed)
- [ ] Each logical step is one commit (not one mega-commit)

---

## Risks and Guardrails

1. **DO NOT modify `app/src/ai/agent_providers/chat_stream.rs`** - BYOP core
2. **DO NOT delete `crates/remote_server/`** - SSH remote dev, not cloud
3. **KEEP `CONTEXT.md` in root** - Used by AI skill system as domain glossary
4. **KEEP `terminal/ref_tests/`** - 41 MB of valid regression test data
5. **KEEP user-toggleable FeatureFlags** - ligatures, rect_selection etc.
6. **`cargo check` after every batch of deletions** - Don't accumulate
7. **One commit per logical step** - Easy to bisect if something breaks

---

## Expected Outcomes (all issues complete)

| Metric | Before | After | Delta |
|--------|--------|-------|-------|
| Dep tree lines | ~4378 | ~2000 | -54% |
| Source files removed | - | ~700 | large |
| Lines of code removed | - | ~5000 | estimate |
| Incremental check time | 30-60s | ~15-20s | -50% |
| Feature flag variants | 326 | ~50 | -85% |
| #[allow(dead_code)] sites | 28 | ~5 | -82% |
| Git-tracked binary bloat | ~175 MB | ~60 MB | -66% |