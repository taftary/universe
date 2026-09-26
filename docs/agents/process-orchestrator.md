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

| Step | Who | Output | Issue status |
| --- | --- | --- | --- |
| 1. Draft | lead | one-paragraph goal + open questions, in the issue body | `status:draft` |
| 2. Plan | lead + support | subtask checklist, acceptance criteria, docs to update | `status:planned` |
| 3. Implement | lead (or dev when code) | artefacts changed | `status:in-progress` |
| 4. Review / test | tester or techlead (never the author) | review comment on the issue, checks pass | `status:review` |
| 5. Docs | lead | documents mapped to `area:` updated | `status:review` |
| 6. Close | orchestrator via gh-orchestrator | closing comment, commit references `#nn` | `status:done` |

With `tracking: off` the same steps run but outputs are reported to the user instead of written to an issue, and no git write operations occur.

## Handoff brief

When passing work between profiles, use this block so context is never lost:

```
from: <profile>
to: <profile>
issue: #<nn>
ask: <one sentence>
inputs: <files, numbers, constraints>
done when: <acceptance criterion>
```

## Blocked

If a profile cannot proceed (missing decision, missing data, conflicting docs): set `status:blocked`, comment the blocker on the issue, and return to the orchestrator with the question for the user.

## Self-update

If a request keeps landing on the wrong profile, or a needed profile does not exist, update the routing table above and, if needed, create the profile from [profiles/_template.md](profiles/_template.md). Record the change in the Changelog below.

## Changelog

- 2026-09-26: created.
