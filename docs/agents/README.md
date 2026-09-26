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
orchestrator.md
  |-- 1. classify request (type, area)
  |-- 2. gh-orchestrator.md : preflight (gh installed + authenticated?)
  |        gh-available -> issue tracking ON
  |        gh-declined  -> orchestrator ends; work continues untracked
  |-- 3. process-orchestrator.md : pick profile(s), run the sequence
  |        draft -> plan (issue + subtasks) -> implement -> review/test -> docs -> close
  '-- 4. close : issue updated, docs synced, commit references #nn
```

## Lifecycle

Single state machine for every tracked request. Recipes are in [gh-orchestrator.md](gh-orchestrator.md). The handoff block from [process-orchestrator.md](process-orchestrator.md#handoff-brief) is mandatory between profiles.

| Step | Profile | gh recipe | Status in | Status out | Agent label add |
| --- | --- | --- | --- | --- | --- |
| 1. Draft | lead | Create an issue | none | `status:draft` | `agent:<lead>` at create |
| 2. Plan | lead + support | Update status (plan comment) | `status:draft` | `status:planned` | `agent:<support>` when different from lead |
| 3. Implement | lead (or dev when code) | Branch, commit, PR | `status:planned` | `status:in-progress` | none |
| 4. Review / test | tester or techlead, never the author | Update status (review comment) | `status:in-progress` | `status:review` | `agent:<reviewer>` before the status change |
| 5. Docs | lead | Update status (docs comment) | `status:review` | `status:review` | none |
| 6. Close | orchestrator via gh-orchestrator | Close | `status:review` | `status:done` | none |

Rules: reviewer is never the author; the reviewer `agent:` label is added before `status:review`; `status:blocked` may interrupt any step with a blocker comment.

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
- Never run git write operations (`commit`, `tag`, `push`, `branch`) or `gh` write operations outside the recipes in `gh-orchestrator.md`.
- When an agent finds its own instructions incomplete or misaligned with reality, it updates its own file (self-update rule in `profiles/_template.md`).
- Output is plain Markdown, English, no emojis.
