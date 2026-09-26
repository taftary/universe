# M0 Process and Foundations

GitHub milestone 1. Closes with tag v0.1.0.

## Scope

Process, agent system, tech decisions, and build scaffold. No gameplay.

## Issues

16 closed, 1 open:

- #1 Define tech stack and engineering standards (closed)
- #2 Tech stack decisions, architecture and simulation docs (closed)
- #3 Coding standards, quality gates and mobile docs (closed)
- #4 Wire tech docs into agent system and indexes (closed)
- #5 Fix tech-docs review findings from M0 stack drop (closed)
- #6 Record Bielefeld URL and deduplicate unsafe rule sentence (closed)
- #7 Scaffold Rust workspace with CI gates (implemented by PR #8, closed)
- #9 Make profiles invocable and define process lifecycle (closed)
- #10 change(process): enforce depth-1 safe lifecycle (implemented by PR #11, closed)
- #12 Standardize Self-update rule and expand stub agent profiles (implemented by PR #13, closed)
- #15 Enforce per-step progress updates and acceptance-criteria validation in lifecycle (closed)
- #16 Use Project and Development in process lifecycle with PR checks loop (implemented by PR #17, closed)
- #18 Close M0: milestone docs and release v0.1.0 (this issue — closed by this close-out)

PRs #8, #11, #13, #17 count toward the 4 closed PRs.

## Decisions

D-001..D-008 in [../tech.md](../tech.md). Agent system in [../agents/](../agents/). CI gates single source of truth in [../tech/quality.md](../tech/quality.md).

## Scaffold

Flat Rust workspace: crates engine, game, debug, tools, plus tests/smoke.rs. Edition 2024, rust-version 1.95 floor.

## Definition of Done

- Entry: issues listed above assigned to M0.
- Exit: all M0 issues closed, CI gates green, docs/tech.md and docs/agents/ consistent.
- Release: v0.1.0 on commit 9e3c421 (main), workspace version 0.1.0.

Related: [../../README.md](../../README.md), [../specs.md](../specs.md), [../agents/gh-orchestrator.md](../agents/gh-orchestrator.md).
