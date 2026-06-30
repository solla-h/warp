# Issue 1: Physical Cleanup

## Summary

Remove large binary files, build logs, and stale documentation that are tracked by git but serve no purpose for the Marb BYOP terminal. This is zero-risk file deletion with no compile impact.

## Problem

The git working tree contains ~175 MB of tracked files that are dead weight:
- 73.5 MB of Windows PDB debug symbols (never needed at runtime)
- ~80 MB of Yarn package cache (TypeScript build artifacts in a Rust project)
- 14.3 MB of cloud integration test SQLite database
- Build logs and stale planning documents from prior development phases

These inflate clone time, bloat search results, and waste disk space.

## Goals

- Remove all tracked PDB debug symbol files
- Remove Yarn package cache from version control
- Remove cloud integration test data
- Remove build logs and superseded planning documents
- Add appropriate .gitignore entries to prevent re-addition

## Non-goals

- Modifying any Rust source code
- Changing build configuration
- Git history rewriting (that is a separate long-term task)
- Removing `terminal/ref_tests/` data (valid regression tests)
- Removing `CONTEXT.md` (used by AI skill system)

## Behavior

1. After this issue, `git ls-files "*.pdb"` returns 0 results
2. After this issue, `git ls-files "*/.yarn/cache/*"` returns 0 results
3. After this issue, `cargo check --bin warp-oss` still passes unchanged
4. `.gitignore` contains entries for `*.pdb` and `.yarn/cache/`

## Files to Remove

| File/Pattern | Size | Reason |
|---|---|---|
| `app/assets/windows/x64/OpenConsole.pdb` | 32.4 MB | Debug symbols |
| `app/assets/windows/arm64/OpenConsole.pdb` | 28.8 MB | Debug symbols |
| `app/assets/windows/x64/conpty.pdb` | 6.2 MB | Debug symbols |
| `app/assets/windows/arm64/conpty.pdb` | 6.1 MB | Debug symbols |
| `crates/command-signatures-v2/js/.yarn/cache/` | ~80 MB | Yarn cache |
| `crates/integration/tests/data/cloud_objects.sqlite` | 14.3 MB | Cloud test data |
| `build_log.txt` | 57.4 KB | Build log |
| `build_log2.txt` | 21.5 KB | Build log |
| `DEPENDENCY_GRAPH.md` | 20.0 KB | Outdated dep graph |
| `PLAN.md` | 7.4 KB | Superseded plan |
| `NEXT_STEPS.md` | 6.7 KB | Mostly completed |
| `CODE_REVIEW_ISSUES.md` | 6.1 KB | Stale review notes |
| `TODO_REMAINING.md` | 3.9 KB | Mostly completed |
| `WARP.md` | 12.7 KB | Warp original docs |
| `LOCAL_FIRST.md` | 15.4 KB | Superseded by PRD |