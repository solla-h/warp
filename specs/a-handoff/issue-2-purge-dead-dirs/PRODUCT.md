# Issue 2: Purge Dead Directories

## Summary

Remove entire directories that are vestigial from the original Warp codebase and serve no purpose for Marb BYOP terminal.

## Problem

Several directories contain hundreds of files from the original Warp team's internal processes:
- `specs/` (410 files) — Jira/GitHub issue specifications for Warp features
- `.agents/skills/` (15 skills) — Warp-internal AI development workflows
- `docker/agent-dev/` — Cloud agent development Docker environment
- `images/` — Warp/Oz brand images
- `archive/` — Superseded planning documents

These pollute search results, confuse AI code indexing, and add cognitive noise.

## Goals

- Remove all Warp-internal spec documents (410 files)
- Remove Warp-internal agent skills (15 skills)
- Remove cloud agent Docker config
- Remove Warp brand images
- Remove archive directory

## Non-goals

- Removing `.agents/` directory itself (keep structure for Marb's own skills)
- Removing `docker/linux-dev/` (still useful for Linux builds)
- Removing `specs/a-handoff/` (this is the current handoff — do NOT delete yourself!)

## Behavior

1. After this issue, `git ls-files specs/ | Measure-Object` returns only `a-handoff/` files
2. After this issue, `ls .agents/skills/` is empty or only contains Marb-specific skills
3. After this issue, no `images/` directory exists
4. `cargo check --bin warp-oss` passes unchanged (zero compile impact)

## Directories to Remove

| Directory | File Count | Reason |
|---|---|---|
| `specs/` (except `a-handoff/`) | ~410 files | Warp internal Jira specs |
| `.agents/skills/` (all 15 dirs) | ~45 files | Warp internal workflows |
| `docker/agent-dev/` | 1 file | Cloud agent dev env |
| `images/` | 2 files | Warp/Oz brand images |
| `archive/` | 2 files | Superseded plans |

## Important Warning

**Do NOT delete `specs/a-handoff/`** — that is the directory containing these very specs you are reading. Delete everything else under `specs/` but preserve `a-handoff/`.