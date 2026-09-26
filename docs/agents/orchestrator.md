# Orchestrator

Top-level coordinator. Invoked by the GLOBAL AGENT ([../../AGENTS.md](../../AGENTS.md)) whenever a request creates or modifies project artefacts.

## Inputs

- The user request, verbatim.
- Repository state (`git status`, current branch).

## Procedure

### 1. Classify

Determine exactly one `type:*` and at least one `area:*` (label list in [gh-orchestrator.md](gh-orchestrator.md#labels)).

| Request looks like | type |
| --- | --- |
| new capability, new system, new document | `type:feature` |
| something is wrong and must be corrected | `type:fix` |
| existing behaviour or document modified | `type:change` |
| roadmap, sequencing, decision to be made | `type:plan` |
| small bounded piece of work | `type:task` |
| verification of state, gap analysis | `type:audit` |
| status or findings write-up | `type:report` |
| unshaped proposal to capture | `type:idea` |

If the classification is ambiguous, ask the user one short question before continuing.

### 2. GitHub preflight

Run the **Preflight** section of [gh-orchestrator.md](gh-orchestrator.md). It returns one of:

- `gh-available` — continue to step 3 with issue tracking enabled.
- `gh-declined` — the user refused to install or authenticate `gh`. **End this orchestrator.** Tell the user the work will proceed without issue tracking, then hand the request directly to [process-orchestrator.md](process-orchestrator.md) with `tracking: off`. No git write operations are performed in this mode.
- `gh-pending` — the user is installing; wait and re-run preflight. Do not start work.

### 3. Delegate

Act as the sequencer. Hand off to [process-orchestrator.md](process-orchestrator.md) with this brief:

```
type: <type:*>
area: <area:*, ...>
tracking: on | off
request: <verbatim user request>
context: <relevant issues #nn, relevant docs paths>
```

Sequencer rules:

- Spawn only depth-1 subagents. Pass `files:` and `after:` in each handoff. Never delegate sequencing.
- Require a subtask plan where each line uses the format `- [ ] Step N (owner:<profile>, files:<paths>, after:<prior>)`.
- Before each spawn, check the DAG: related means same issue or overlapping files. If related, wait for the prior step to finish.
- Files versus gh split: profiles edit files only; [gh-orchestrator.md](gh-orchestrator.md) owns all `gh` and `git` write operations.

### 4. Close

When process-orchestrator reports done, verify:

- [ ] Issue exists, has `type:`, `status:`, `area:`, `agent:` labels and a milestone (tracking on).
- [ ] Subtasks in the issue body are checked or moved to a follow-up issue.
- [ ] Subtasks use the format `- [ ] Step N (owner:<profile>, files:<paths>, after:<prior>)` and `agent:` labels match the owners.
- [ ] Documents mapped to the `area:` (see [../README.md](../README.md#issue-to-document-mapping)) are updated.
- [ ] Commit(s) reference `#nn` and follow the commit recipe.
- [ ] Issue status label is `status:done` and the issue is closed with a closing comment summarising what changed and where.

Report to the user: issue number, files touched, next suggested step.

## Constraints

- Never skip step 2. Never call `git commit`, `git tag`, `git push` or `gh` write commands directly; only via gh-orchestrator recipes. Profiles edit files only; gh-orchestrator owns `gh` and `git` writes.
- One request, one issue. Split into sub-issues rather than one oversized issue.
- The orchestrator is the sequencer. Spawn only depth-1 subagents with explicit `files:` and `after:`; never delegate sequencing. Before each spawn check the DAG (related is same issue or overlapping files) and wait if related; do not parallelise writes to the same file.
