# process-orchestrator

Acts as the manager: selects the right profile(s) for a brief received from [orchestrator.md](orchestrator.md) and runs the standard sequence. Profiles live in [profiles/](profiles/).

## Input

The brief from the orchestrator (`type`, `area`, `tracking`, `request`, `context`).

## Routing table

Pick the **lead** profile from the topic; add **support** profiles as listed. When several rows match, the first match is lead.

| Topic detected in request | Lead | Support |
| --- | --- | --- |
| vision, scope, priorities, what to build next | [po](profiles/po.md) | ppo, analyst |
| roadmap, milestone content, sequencing | [ppo](profiles/ppo.md) | po, scrum-master |
| orbital mechanics, celestial bodies, star systems, time scales | [astronomer](profiles/astronomer.md) | physicist, architect |
| thermodynamics, atmospheres, reentry, physiology numbers, radiation | [physicist](profiles/physicist.md) | scientist, mathematician |
| numerical methods, integrators, precision, floating origin | [mathematician](profiles/mathematician.md) | physicist, architect |
| chemistry, geology, biology, ISRU processes | [scientist](profiles/scientist.md) | physicist |
| system design, module boundaries, data flow, tech decisions | [architect](profiles/architect.md) | techlead |
| tech stack docs, coding standards, stack decisions | [architect](profiles/architect.md) | techlead |
| code standards, tooling, performance budget, review | [techlead](profiles/techlead.md) | architect, dev |
| implementation of a specified change | [dev](profiles/dev.md) | techlead, tester |
| tests, verification, acceptance criteria, reference values | [tester](profiles/tester.md) | physicist, dev |
| player experience, legibility, flow, controls, warnings | [ux](profiles/ux.md) | ui, po |
| screens, HUD, instruments layout, mobile ergonomics | [ui](profiles/ui.md) | ux |
| gap analysis, audit, report, metrics | [analyst](profiles/analyst.md) | po, techlead |
| process, agents, issues hygiene, ceremonies | [scrum-master](profiles/scrum-master.md) | analyst |

If nothing matches, lead is `analyst` to shape the request first.

## Standard sequence

Every profile follows this order. Skip a step only if the brief's `type` makes it meaningless (e.g. `type:idea` stops after **plan**).

| Step | Who | Output | Issue status | Project | Development |
| --- | --- | --- | --- | --- | --- |
| 1. Draft | lead | one-paragraph goal + open questions, in the issue body | `status:draft` | Draft | — |
| 2. Plan | lead + support | subtask checklist, acceptance criteria, Decisions, docs to update — all in issue body | `status:planned` | Planned | — |
| 3. Implement | lead (or dev when code) | artefacts changed | `status:in-progress` | In progress | branch linked via `gh issue develop` + draft PR with `Closes #nn` |
| 4. Review / test | tester or techlead (never the author) | review comment with one PASS/FAIL line per AC, checks pass | `status:review` | Review | CI watch loop (`gh pr checks --watch --fail-fast`, max 2 fix attempts) |
| 5. Docs | lead | documents mapped to `area:` updated | `status:review` | Review | docs commits on same branch |
| 6. Close | orchestrator via gh-orchestrator | closing comment, commit references `#nn` | `status:done` | Done | merge when checks green (`gh pr ready`, squash merge) |

With `tracking: off` the same steps run but outputs are reported to the user instead of written to an issue, and no git write operations occur.

## Progress cadence

Each step boundary follows this order: profile returns -> gh-orchestrator ticks the Step, posts the step-done comment, flips the status label if it changes -> then the next profile is spawned. Batching step updates is forbidden.

- Step 2 (Plan) writes acceptance criteria into the body `## Acceptance criteria` and resolves open questions into `## Decisions`. Any question still open means `status:blocked` (no `status:in-progress` with open questions).
- Step 4 (Review) output is one PASS/FAIL line per AC; the reviewer ticks passing ones via the Update acceptance criteria recipe; any FAIL adds a follow-up Step and returns to `status:in-progress`.
- Label + Project Status flip together: every `status:` label change and its matching Project Status change run in the same gh-orchestrator step; never one without the other.
- The plan comment is a pointer only; the body is single source of truth.

## Depth-1 execution

The top orchestrator is the sequencer; process-orchestrator executes its sequence. Only the top orchestrator spawns subagents. Subagents never spawn subagents. Execution depth is 1.

## Task DAG

Steps form a DAG via `after:` and `files:`. Related means same issue chain (linked by `after:`) or overlapping `files:`; the later step waits for the earlier step to finish. This matches the `same issue or overlapping files` rule in orchestrator.md. Steps with disjoint `files:` and no `after:` link may run in parallel.

## Files vs gh split

Profile tasks edit files only. All issue, label, branch, commit, and push operations go through gh-orchestrator. Profiles never perform gh or git writes; they return file changes and request tracking updates through the handoff brief.

## Plan location

The plan must live in issue subtasks. Step 2 of the standard sequence writes one checklist line per step in the tracking issue:

`- [ ] Step N (owner:<profile>, files:<paths>, after:<prior>)`

Use `after:-` when there is no prior step.

`## Acceptance criteria` (`- [ ] ACn ...`) and `## Decisions` (`- Q: ... -> A: ... (by:...)`) live alongside `## Subtasks` in the body.

## Handoff brief

When passing work between profiles, use this block so context is never lost:

```
from: <profile>
to: <profile>
issue: #<nn>
ask: <one sentence>
inputs: <files, numbers, constraints>
files: <paths this step edits>
after: <prior Step N or - if none>
done when: <acceptance criterion>
```

Handoff note: implementation briefs cite `docs/tech/standards.md` for layout, lints, and performance rules. Implementation briefs for CI failures include the failed log excerpt.

## Blocked

If a profile cannot proceed (missing decision, missing data, conflicting docs, open Decision at end of Plan): set `status:blocked`, comment the blocker on the issue, and return to the orchestrator with the question for the user.

CI failures that exhaust the 2-attempt cap set `status:blocked` with matching Project Status `Blocked`, record the failed log excerpt in a comment, and return to the user.

## Retroactivity

Process changes apply forward only:

- Closed issues stay as-is; their history is not rewritten (e.g. #1-#7 closed before profile adapters existed).
- Reopening a closed issue re-routes it with the current routing table above.
- Work done before a process change is documented once, in the next issue comment or changelog, not re-applied retroactively.

## Self-update

If a request keeps landing on the wrong profile, or a needed profile does not exist, update the routing table above and, if needed, create the profile from [profiles/_template.md](profiles/_template.md). Record the change in the Changelog below.

## Changelog

- 2026-09-26: created.
- 2026-09-26: add tech-stack routing row and standards.md handoff note.
- 2026-09-26: all 14 profiles invocable via adapters in .opencode/agent/ and .claude/agents/; lifecycle table added to README; retroactivity rule added (#9).
- 2026-09-26: define depth-1 execution, DAG wait rule, files-vs-gh split, and issue subtask plan format (#10); fix sequencer wording per #10 audit.
- 2026-09-26: enforce per-step gh-orchestrator cadence, body AC+Decisions sections, per-AC review verdicts with FAIL->follow-up Step (#15).
- 2026-09-26: Project mirror + Development linkage + CI watch loop (max 2) in sequence (#16).
