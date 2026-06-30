# Issue 2: Tech Spec - Purge Dead Directories

## Context

These are pure directory deletions with no compile impact. The directories contain documentation, configuration, and images from the original Warp team that have no relevance to Marb.

## Relevant files

- `specs/` — 410 files across ~200 subdirectories (APP-*, REMOTE-*, QUALITY-*, GH*, etc.)
- `.agents/skills/` — 15 Warp-internal skills: add-feature-flag, add-telemetry, changelog-draft, classify-changelog-pr, create-launch-modal, dedupe-issue-local, onboarding-verification-skill, promote-feature, remove-feature-flag, reproduce-bug-report-local, review-pr-local, rust-unit-tests, triage-issue-local, warp-integration-test, warp-ui-guidelines
- `docker/agent-dev/Dockerfile` — Cloud agent Docker setup
- `images/Built-With-Warp-Export@2x.png` — Warp brand image
- `images/Powered-By-Oz-Export@2x.png` — Oz brand image
- `archive/MASTER_TODO.md`, `archive/SLIM_PLAN_V2.md` — Superseded plans

## Proposed changes

### 1. Move specs/a-handoff/ to temporary safe location

```powershell
Move-Item specs/a-handoff $env:TEMP/a-handoff-backup
```

### 2. Delete specs/ entirely

```powershell
git rm -r specs/
```

### 3. Recreate specs/a-handoff/ from backup

```powershell
New-Item -ItemType Directory specs/a-handoff -Force
Move-Item $env:TEMP/a-handoff-backup/* specs/a-handoff/
git add specs/a-handoff/
```

### 4. Delete .agents/skills/

```powershell
git rm -r .agents/skills/
```

Note: Keep `.agents/` directory and any non-skills config (like `.agents/triage-issue-local/` at root level if it exists separately).

### 5. Delete docker/agent-dev/

```powershell
git rm -r docker/agent-dev/
```

Keep `docker/linux-dev/` — it is still useful for Linux development builds.

### 6. Delete images/ and archive/

```powershell
git rm -r images/
git rm -r archive/
```

## Testing and validation

### Verification steps

1. `cargo check --bin warp-oss` passes (no compile change)
2. `git ls-files specs/ | Select-String -NotMatch "a-handoff"` returns 0 results
3. `git ls-files .agents/skills/` returns 0 results
4. `git ls-files images/` returns 0 results
5. `git ls-files archive/` returns 0 results
6. `git ls-files docker/agent-dev/` returns 0 results
7. `specs/a-handoff/HANDBOOK.md` still exists and is tracked

### Acceptance criteria

- [ ] All specs/ subdirectories removed EXCEPT a-handoff/
- [ ] All 15 .agents/skills/ directories removed
- [ ] docker/agent-dev/ removed, docker/linux-dev/ intact
- [ ] images/ directory removed
- [ ] archive/ directory removed
- [ ] `cargo check --bin warp-oss` passes
- [ ] Commit: "chore: purge Warp-internal specs, skills, and brand assets"

## Risks

- **Risk:** Accidentally deleting `specs/a-handoff/` (this handoff directory)
  - **Mitigation:** Move it to temp before bulk delete, restore after
- **Risk:** Some `.agents/` config might be needed by current triage workflow
  - **Mitigation:** Only delete `.agents/skills/`, keep `.agents/` root files