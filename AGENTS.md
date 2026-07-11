# surgeist-template Repository Guide

Use `$surgeist-agent` for every task in this repository.

## Authority Split

This file is the leaf repository's committed discovery entry point. It owns the
mapping from mutable leaf facts to authoritative sources, the intended crate and
architecture boundary, and the configured local command inventory. The sources
named below own their current values.

`$surgeist-agent` is the sole Surgeist workflow authority. It owns scope control,
planning, debugging and TDD, worker/reviewer gates, external-software permission,
the absolute unsafe prohibition, Git landing and publication, and cross-repository
handoffs. This file does not redefine those workflows or grant authority to
mutate, install, commit, or publish.

Resolve an apparent conflict by domain: use this file and the sources below for
mutable repository facts; use `$surgeist-agent` for workflow. Higher-priority user
and system instructions still apply. Do not import another general development
workflow.

## Repository Identity And Ownership

`surgeist-template` is an independent leaf repository. It owns its manifest,
domain implementation, public front door, focused tests and docs, commits, and
published `main` candidate.

The root `surgeist` repository owns the facade and public composition surface,
cross-crate adapters, root integration tests and tools, this leaf's gitlink, and
the API generator and generated audit artifacts. A parent workspace, Codex
project, task, branch, or worktree does not change repository ownership.

## Discover The Current Structure

Read these sources instead of relying on cached descriptions.

| Fact | Authoritative source |
| --- | --- |
| Package identity, edition, dependencies, features, and targets | `Cargo.toml` |
| Public front door | `src/lib.rs` and its reexports |
| Current behavior and crate boundary | `README.md` and `src/` |
| Focused verification | tracked `#[cfg(test)]` modules in `src/` and integration tests in `tests/` |
| Additional configured commands | Cargo targets and features in `Cargo.toml`, `README.md`, and tracked task-runner or CI configuration when present |
| Integration MSRV, authoritative URL, and compatible pin when root integration is in scope | root `Cargo.toml`, root `.gitmodules`, and the root committed gitlink |

When these sources disagree, report the exact paths and revisions. Do not guess,
silently update another document, or widen the task to reconcile them.

## Crate Boundary

`surgeist-template` owns strict, typed template and DSL authoring contracts,
including parsing, validation, and source generation. It excludes runtime
evaluation, host integration, and cross-crate lowering.

Surgeist-to-Surgeist lowering and adapters belong to root, and sibling internals
are not this repository's surface.

## API Artifacts

Source in this repository is authoritative. The root `surgeist` repository owns
the only API generator and all generated API audit artifacts; this leaf carries
no copies.

## Command Inventory

These commands describe local verification capability. `$surgeist-agent`
determines the exact gate, order, feature matrix, and whether already-present
tooling can run without unauthorized acquisition.

```sh
cargo check -p surgeist-template
cargo test -p surgeist-template
cargo clippy -p surgeist-template --all-targets -- -F unsafe-code -D warnings
cargo fmt --check
```

Discovery is complete when the owning repository, public front door, dependency
and feature facts, verification sources, API-artifact owner, and applicable
command inventory are identified from the sources above.
