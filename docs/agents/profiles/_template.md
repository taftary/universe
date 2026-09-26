# <Profile name>

Label: `agent:<name>`

## Role

One paragraph: what this profile is accountable for in the project, and what it is explicitly not.

## Responsibilities

- ...

## Inputs

What this profile needs before starting (issues, docs, numbers, decisions).

## Outputs

Artefacts this profile produces (spec sections, code, tests, reports) and where they go.

## Skills and references

- Domain knowledge the profile must apply.
- Project docs it must read: [../../../README.md](../../../README.md), [../../specs.md](../../specs.md), [../../tech.md](../../tech.md).
- External references (standards, textbooks, datasets) when relevant.

## Working rules

- Read the issue and its comments before acting; write findings back as comments.
- Numbers must be physically derived and cite their source or formula.
- Prefer one model that produces many outcomes over many tuned rules.
- Never perform git or gh write operations; hand off to gh-orchestrator.

## Definition of done

- [ ] Acceptance criteria in the issue are met.
- [ ] Documents mapped to the `area:` are updated.
- [ ] A reviewer profile (not the author) has commented on the issue.

## Self-update rule

When this profile detects that its instructions are missing, wrong or misaligned with the repository (a reference moved, a rule contradicts a decision, a needed skill is absent):

1. Edit this file to fix the gap. Keep the change minimal and specific.
2. Append a dated line to the Changelog below stating what changed and why.
3. Mention the change in the current issue comment, or open a `type:change` + `area:process` issue if it is significant.

## Changelog

- YYYY-MM-DD: created.
