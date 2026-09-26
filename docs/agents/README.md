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
  |-- 2. [gh-orchestrator.md](gh-orchestrator.md) : preflight (gh installed + authenticated + project scope + project exists?)
  |        gh-available -> issue tracking ON
  |        gh-declined  -> orchestrator ends; work continues untracked
  |-- 3. [process-orchestrator.md](process-orchestrator.md) : pick profile(s), run the sequence
  |        draft -> plan (issue + subtasks) -> implement (develop branch + draft PR) -> review/test (CI watch loop) -> docs -> close (squash merge)
  |        |-- profile tasks (depth-1, files only, with `files:` + `after:`)
  |        '-- gh-orchestrator tasks (`gh` and `git` writes only)
  |        DAG: related (same issue or overlapping files) waits; disjoint may run in parallel
  '-- 4. close : issue updated, docs synced, commit references #nn
```

## Lifecycle

Single state machine for every tracked request. The sequencer is [orchestrator.md](orchestrator.md); recipes are in [gh-orchestrator.md](gh-orchestrator.md). The handoff block from [process-orchestrator.md](process-orchestrator.md#handoff-brief) is mandatory between profiles.

Plan and DAG: the plan lives in issue subtasks, one line per step as `- [ ] Step N (owner:<profile>, files:<paths>, after:<prior>)`. Steps carry `files:` and `after:`; related steps (same issue or overlapping files) run sequentially and the later step waits, disjoint steps may run in parallel. Files versus gh split: profile tasks edit files only; all `gh` and `git` writes go through [gh-orchestrator.md](gh-orchestrator.md). ## Acceptance criteria (`- [ ] ACn`) and ## Decisions (`- Q: ... -> A: ... (by:...)`) live in the body alongside subtasks.

| Step | Status label | Project Status | Development | gh recipe |
| --- | --- | --- | --- | --- |
| 1. Draft | `status:draft` | Draft | — | Create + Add to project |
| 2. Plan | `status:planned` | Planned | — | Update status |
| 3. Implement | `status:in-progress` | In progress | branch linked, draft PR Closes | Develop + draft PR |
| 4. Review / test | `status:review` | Review | PR checks green, fix loop max 2 | Update AC + CI watch |
| 5. Docs | `status:review` | Review | docs commits, same branch | Update status |
| 6. Close | `status:done` | Done | PR ready, checks, squash merge, delete | Close |
| any (blocked) | `status:blocked` | Blocked | — | Update status |

Rules: reviewer is never the author; the reviewer `agent:` label is added before `status:review`; `status:blocked` may interrupt any step with a blocker comment. Cadence: gh-orchestrator records after EVERY Step before the next spawn (no batching): Step output comment first, then the step-done pointer; only the reviewer ticks AC; step-done comment is a pointer, Step output comment is the record, body is source of truth.

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
