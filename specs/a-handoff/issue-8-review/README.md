# Issue 8 Review Report: Unmet Spec Requirements

## Context

Issue 8 spec (`specs/a-handoff/issue-8-remove-dead-crates/TECH.md`) assumed that Issues 3-7 would have removed all code referencing cloud crates. In reality, Issues 3-7 adopted a "hollow shell" strategy — keeping the crate directories intact as no-op stubs while removing `cfg(feature = "cloud")` gates. This means the crates still provide base types actively imported by 100+ source files.

This directory contains deep-dive analysis for each unmet requirement, documenting:
- Why the requirement cannot be fulfilled in the current state
- Every source file that depends on the crate/dependency
- The specific types, traits, and functions imported
- The role each dependency plays in the architecture

## Documents

| # | Requirement | Document | Verdict |
|---|---|---|---|
| 1 | `crates/cloud_objects/` deleted | [01-cloud-objects-crate.md](01-cloud-objects-crate.md) | Cannot remove — provides foundational identity/permission types |
| 2 | `crates/cloud_object_models/` deleted | [02-cloud-object-models-crate.md](02-cloud-object-models-crate.md) | Cannot remove — provides AI agent config types |
| 3 | Workspace deps for above cleaned | [03-workspace-deps.md](03-workspace-deps.md) | Blocked by #1 and #2 |
| 4 | `app/Cargo.toml` cleaned of dead deps | [04-app-cargo-deps.md](04-app-cargo-deps.md) | Blocked by #1, #2, #5, #6, #7 |
| 5 | `session-sharing-protocol` removed | [05-session-sharing-protocol.md](05-session-sharing-protocol.md) | Cannot remove — used by terminal session sharing |
| 6 | `cynic` workspace dep removed | [06-cynic.md](06-cynic.md) | Cannot remove — Id type used across AI and settings |
| 7 | `warp_multi_agent_api` removed | [07-warp-multi-agent-api.md](07-warp-multi-agent-api.md) | Cannot remove — core agent API protocol types |

## Commit Reference

- Issue 8 commit: `ff6cfa9e` (what WAS removed: warp_graphql_schema, tink patches, graphql-ws-client)
- Base: `833b8f8c` (issue 7 complete)

## Deep Dive

- [cloud-objects-dependency-graph.md](cloud-objects-dependency-graph.md) — Full reference graph with 200 transitive consumers, type layering analysis, and extraction strategy
