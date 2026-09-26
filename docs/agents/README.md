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
