# Agent System

All agents are plain Markdown so any model or IDE can read and execute them. IDE-specific adapters (`.opencode/agent/`, `.claude/agents/`) only point to the files here.

## Flow

```
Request
  |
  v
GLOBAL AGENT (AGENTS.md) -- question? --> answer directly, stop
  | change / fix / feature / plan / task / audit / report / idea
  v
[orchestrator.md](orchestrator.md) (sequencer)
  |-- 1. classify request (type, area)
  |-- 2. [gh-orchestrator.md](gh-orchestrator.md) : preflight (gh installed + authenticated?)
  |        gh-available -> issue tracking ON
  |        gh-declined  -> orchestrator ends; work continues untracked
  |-- 3. [process-orchestrator.md](process-orchestrator.md) : pick profile(s), run the sequence
  |        draft -> plan (issue + subtasks) -> implement -> review/test -> docs -> close
  |        |-- profile tasks (depth-1, files only, with `files:` + `after:`)
  |        '-- gh-orchestrator tasks (`gh` and `git` writes only)
  |        DAG: related (same issue or overlapping files) waits; disjoint may run in parallel
  '-- 4. close : issue updated, docs synced, commit references #nn
```

## Lifecycle

Single state machine for every tracked request. The sequencer is [orchestrator.md](orchestrator.md); recipes are in [gh-orchestrator.md](gh-orchestrator.md). The handoff block from [process-orchestrator.md](process-orchestrator.md#handoff-brief) is mandatory between profiles.

Plan and DAG: the plan lives in issue subtasks, one line per step as `- [ ] Step N (owner:<profile>, files:<paths>, after:<prior>)`. Steps carry `files:` and `after:`; related steps (same issue or overlapping files) run sequentially and the later step waits, disjoint steps may run in parallel. Files versus gh split: profile tasks edit files only; all `gh` and `git` writes go through [gh-orchestrator.md](gh-orchestrator.md). ## Acceptance criteria (`- [ ] ACn`) and ## Decisions (`- Q: ... -> A: ... (by:...)`) live in the body alongside subtasks.

| Step | Profile | gh recipe | Status in | Status out | Agent label add | Body section written |
| --- | --- | --- | --- | --- | --- | --- |
| 1. Draft | lead | Create an issue | none | `status:draft` | `agent:<lead>` at create | ## Goal |
| 2. Plan | lead + support | Update status (plan comment) | `status:draft` | `status:planned` | `agent:<support>` when different from lead | ## Subtasks + ## Acceptance criteria + ## Decisions |
| 3. Implement | lead (or dev when code) | Branch, commit, PR | `status:planned` | `status:in-progress` | none | — (steps ticked per-step) |
| 4. Review / test | tester or techlead, never the author | Update status (review comment) | `status:in-progress` | `status:review` | `agent:<reviewer>` before the status change | ## Acceptance criteria ticked (reviewer-only) |
| 5. Docs | lead | Update status (docs comment) | `status:review` | `status:review` | none | — (area docs updated) |
| 6. Close | orchestrator via gh-orchestrator | Close | `status:review` | `status:done` | none | closing checklist posted |

Rules: reviewer is never the author; the reviewer `agent:` label is added before `status:review`; `status:blocked` may interrupt any step with a blocker comment. Cadence: gh-orchestrator records after EVERY Step before the next spawn (no batching); only the reviewer ticks AC; plan comment is a pointer, body is source of truth.

## Adapters

Every profile in [profiles/](profiles/) is invocable through a one-file adapter in both `.opencode/agent/<name>.md` and `.claude/agents/<name>.md`. Each adapter only points to its profile file, which stays the source of truth. The three orchestrators (`orchestrator`, `gh-orchestrator`, `process-orchestrator`) have adapters in the same two directories.

## Files

| File | Role |
| --- | --- |
| [orchestrator.md](orchestrator.md) | Top-level coordinator invoked by the GLOBAL AGENT |
| [gh-orchestrator.md](gh-orchestrator.md) | GitHub CLI preflight and recipes: issues, labels, milestones, branches, commits, tags, releases |
| [process-orchestrator.md](process-orchestrator.md) | Routes work to the right profile(s) and runs the standard sequence |
| [profiles/](profiles/) | One file per role (PO, architect, dev, tester, physicist, ...) built from [profiles/_template.md](profiles/_template.md) |

## Rules shared by every agent

- Read the linked GitHub issue(s) before acting; comment on them when done.
- Depth-1 only: only [orchestrator.md](orchestrator.md) spawns subagents; subagents never spawn subagents and sequencing is never delegated.
- Never run git write operations (`commit`, `tag`, `push`, `branch`) or `gh` write operations outside the recipes in [gh-orchestrator.md](gh-orchestrator.md). Profiles edit files only.
- When an agent finds its own instructions incomplete or misaligned with reality, it updates its own file (self-update rule in `profiles/_template.md`).
- Output is plain Markdown, English, no emojis.
