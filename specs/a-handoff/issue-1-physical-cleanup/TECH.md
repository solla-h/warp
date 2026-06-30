# Issue 1: Tech Spec - Physical Cleanup

## Context

This is a pure file-deletion task. No Rust code changes, no Cargo.toml edits, no compile risk.

## Relevant code

- `app/assets/windows/x64/OpenConsole.pdb` - ConPTY debug symbols (32.4 MB)
- `app/assets/windows/arm64/OpenConsole.pdb` - ConPTY debug symbols (28.8 MB)
- `app/assets/windows/x64/conpty.pdb` - ConPTY debug symbols (6.2 MB)
- `app/assets/windows/arm64/conpty.pdb` - ConPTY debug symbols (6.1 MB)
- `crates/command-signatures-v2/js/.yarn/cache/` - TypeScript compiler cache (~80 MB)
- `crates/integration/tests/data/cloud_objects.sqlite` - Cloud test DB (14.3 MB)
- `.gitignore` - Needs PDB and yarn cache entries

## Proposed changes

### 1. Remove PDB files + add .gitignore entry

```powershell
git rm app/assets/windows/x64/OpenConsole.pdb
git rm app/assets/windows/arm64/OpenConsole.pdb
git rm app/assets/windows/x64/conpty.pdb
git rm app/assets/windows/arm64/conpty.pdb
```

Add to `.gitignore`:
```
# Debug symbols
*.pdb
```

### 2. Remove Yarn cache + add .gitignore entry

```powershell
git rm -r crates/command-signatures-v2/js/.yarn/cache/
```

Add to `.gitignore`:
```
# Yarn cache
.yarn/cache/
```

### 3. Remove cloud test database

```powershell
git rm crates/integration/tests/data/cloud_objects.sqlite
```

### 4. Remove stale root documents

```powershell
git rm build_log.txt build_log2.txt
git rm DEPENDENCY_GRAPH.md PLAN.md NEXT_STEPS.md
git rm CODE_REVIEW_ISSUES.md TODO_REMAINING.md
git rm WARP.md LOCAL_FIRST.md
```

Note: KEEP `PRD.md` (still relevant) and `CONTEXT.md` (used by AI skills).

### 5. Remove .git-rewrite/ residue (local only, not git-tracked)

```powershell
Remove-Item -Recurse -Force .git-rewrite/
```

## Testing and validation

### Verification steps

1. `cargo check --bin warp-oss` passes (no compile change)
2. `git ls-files "*.pdb" | Measure-Object` returns 0
3. `git ls-files "*/.yarn/cache/*" | Measure-Object` returns 0
4. `git ls-files "cloud_objects.sqlite" | Measure-Object` returns 0
5. `git status` shows only the planned deletions + .gitignore change

### Acceptance criteria

- [ ] All 4 PDB files removed from git tracking
- [ ] .yarn/cache/ removed from git tracking
- [ ] cloud_objects.sqlite removed from git tracking
- [ ] 7 stale root .md files removed
- [ ] .gitignore updated with `*.pdb` and `.yarn/cache/`
- [ ] `cargo check --bin warp-oss` passes
- [ ] Single commit with message: "chore: remove binary bloat and stale docs"

## Risks

None. These files are not referenced by any build system or source code.