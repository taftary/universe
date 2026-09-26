# Documentation Index

Single source of truth for **status and history** is GitHub Issues (see [agents/gh-orchestrator.md](agents/gh-orchestrator.md)).
This folder holds the **reference documents** those issues point to.

| Path | Purpose | Updated when |
| --- | --- | --- |
| [../README.md](../README.md) | Standing project overview, pillars, roadmap shape | Vision or roadmap changes |
| [specs.md](specs.md) | Implementation-level design spec | A `type:feature` / `type:change` issue is closed |
| [tech.md](tech.md) | Tech stack decision record and performance budget | A tech decision is taken |
| [topics/](topics/README.md) | One file per domain topic (scale, survival, planet-gen, ...) | A topic gains a dedicated doc |
| [milestones/](milestones/README.md) | One file per milestone, mirrors GitHub milestones | A milestone is opened or released |
| [agents/](agents/README.md) | Agent system: orchestrators and profiles | An agent self-updates or a new profile is added |

## Issue to document mapping

- Every issue carries one `area:*` label. The area tells which document(s) must be updated before the issue is closed:
  - `area:docs` -> this folder or `README.md`
  - `area:process` -> `agents/`
  - `area:scale`, `area:survival`, `area:planet-gen`, `area:machines`, `area:resources` -> `specs.md` and, when it exists, `topics/<area>.md`
- Every issue belongs to one milestone `M0..M8`; the matching `milestones/Mx-*.md` (when created) lists the issues it contains.
- Every release tag is referenced from the milestone file it closes.
