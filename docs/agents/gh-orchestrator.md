# gh-orchestrator

Only gh-orchestrator runs gh write operations and git branch, commit, push, and tag. Profile tasks never run gh or git writes; they hand off to the recipes below.

Repository: `taftary/universe`. All `gh` commands run from the repository root.

## Preflight

Run in order. Stop at the first failure and follow its instructions.

| Step | Command | On failure |
| --- | --- | --- |
| 1 | `gh --version` | `gh` not installed -> go to **Install** |
| 2 | `gh auth status` | not logged in -> run `gh auth login` interactively with the user |
| 3 | `gh repo view --json nameWithOwner -q .nameWithOwner` | must print `taftary/universe`; otherwise check `git remote -v` |
| 4 | `gh auth status` must list the `project` scope | missing scope -> run `gh auth refresh -s project` |
| 5 | `gh project list --owner @me --format json` contains title `universe` | not found -> go to **Bootstrap project** |

Return `gh-available` when all five pass.

### Install

Tell the user `gh` (GitHub CLI) is required for issue tracking and offer the command for their platform:

- Windows: `winget install --id GitHub.cli` (or `choco install gh`, `scoop install gh`)
- macOS: `brew install gh`
- Debian/Ubuntu: `sudo apt install gh` (see https://github.com/cli/cli/blob/trunk/docs/install_linux.md)

Then ask: "Install now, or continue without issue tracking?"

- User installs -> return `gh-pending`; help with any error; re-run Preflight when told to (a new shell may be needed for PATH).
- User refuses -> return `gh-declined`. The orchestrator ends and no git or gh write operation is performed for this request.

## Conventions

### Labels

GitHub defaults (`bug`, `documentation`, `enhancement`, ...) are kept. Project labels use prefixed groups. Every issue has exactly one `type:`, one `status:`, at least one `area:` and at least one `agent:`.

| Group | Values | Color |
| --- | --- | --- |
| `type:` | feature, fix, change, plan, task, audit, report, idea | `1D76DB` |
| `status:` | draft, planned, in-progress, review, blocked, done | `FBCA04` |
| `area:` | scale, survival, planet-gen, machines, resources, docs, process | `0E8A16` |
| `agent:` | po, ppo, architect, techlead, dev, tester, ux, ui, analyst, scrum-master, physicist, astronomer, mathematician, scientist | `5319E7` |

### Milestones

`M0 Process and foundations`, then `M1`..`M8` matching the roadmap stages in [../../README.md](../../README.md#roadmap-shape-high-level-not-yet-scheduled):

`M1 Orbit to surface`, `M2 Body model`, `M3 Planet generation`, `M4 Ships and transit`, `M5 Ground and air vehicles`, `M6 Subterranean`, `M7 Colonies and robots`, `M8 Creatures and riding`.

### Project

Single user project `universe` (milestone `M0`, repo `taftary/universe`). It mirrors the `status:` labels.

- Status field options: `Draft`, `Planned`, `In progress`, `Review`, `Blocked`, `Done` (matching `status:draft`, `status:planned`, `status:in-progress`, `status:review`, `status:blocked`, `status:done`).
- Views: Board grouped by Status; Roadmap table grouped by Milestone.
- Rule: never change a `status:` label without setting the matching Project Status, and never change the Project Status without changing the matching `status:` label. Both flips happen together (see **Add to project / set status**).
- The token currently lacks the `project` scope, so ids below are placeholders. Fill the Project IDs table after bootstrap (see **Bootstrap project**).

| Key | Placeholder | Filled value |
| --- | --- | --- |
| Project number | `<P>` | 1 |
| Project node id | `<PROJECT_ID>` | PVT_kwHOCpw4Tc4BkyPg |
| Status field id | `<STATUS_FIELD_ID>` | PVTSSF_lAHOCpw4Tc4BkyPgzhjhj7Q |
| Option Draft | `<OPTION_ID_DRAFT>` | 416b1a11 |
| Option Planned | `<OPTION_ID_PLANNED>` | f5f8ddc9 |
| Option In progress | `<OPTION_ID_IN_PROGRESS>` | e7851b1d |
| Option Review | `<OPTION_ID_REVIEW>` | ee7126f1 |
| Option Blocked | `<OPTION_ID_BLOCKED>` | e13e43ca |
| Option Done | `<OPTION_ID_DONE>` | 464eb338 |

### Branches and commits

- Branch: `<type>/<issue>-<slug>` e.g. `feature/12-atmosphere-profile`.
- Commit message: `<type>(<area>): <summary> (#<issue>)` e.g. `feature(survival): add ppCO2 model (#12)`. Use `Closes #nn` in the body when the commit completes the issue.
- Tag: `vMAJOR.MINOR.PATCH`; one tag per release, referenced from the milestone doc.

## Recipes

### Bootstrap (run once)

```sh
# labels
for v in feature fix change plan task audit report idea; do gh label create "type:$v" --color 1D76DB --force; done
for v in draft planned in-progress review blocked done; do gh label create "status:$v" --color FBCA04 --force; done
for v in scale survival planet-gen machines resources docs process; do gh label create "area:$v" --color 0E8A16 --force; done
for v in po ppo architect techlead dev tester ux ui analyst scrum-master physicist astronomer mathematician scientist; do gh label create "agent:$v" --color 5319E7 --force; done

# milestones
gh api repos/taftary/universe/milestones -f title="M0 Process and foundations"
gh api repos/taftary/universe/milestones -f title="M1 Orbit to surface"
gh api repos/taftary/universe/milestones -f title="M2 Body model"
gh api repos/taftary/universe/milestones -f title="M3 Planet generation"
gh api repos/taftary/universe/milestones -f title="M4 Ships and transit"
gh api repos/taftary/universe/milestones -f title="M5 Ground and air vehicles"
gh api repos/taftary/universe/milestones -f title="M6 Subterranean"
gh api repos/taftary/universe/milestones -f title="M7 Colonies and robots"
gh api repos/taftary/universe/milestones -f title="M8 Creatures and riding"
```

PowerShell equivalent of the label loop: `foreach ($v in "feature","fix") { gh label create "type:$v" --color 1D76DB --force }`.

### Bootstrap project (run once)

Run once after Preflight Step 5 reports no `universe` project. Requires the `project` scope (see Preflight Step 4).

```sh
# create the single user project
gh project create --owner @me --title "universe"

# resolve ids and fill the Project IDs table in Conventions > Project
gh project view <P> --owner @me --format json -q .id
gh project field-list <P> --owner @me --format json -q '.fields[] | select(.name=="Status")'
```

Replace the built-in Status single-select options with the six project options (Draft, Planned, In progress, Review, Blocked, Done), for example via GraphQL:

```sh
gh api graphql \
  -f query='mutation($projectId: ID!, $fieldId: ID!, $options: [ProjectV2SingleSelectFieldOptionInput!]!) { updateProjectV2Field(input: {projectId: $projectId, fieldId: $fieldId, singleSelectOptions: $options}) { projectV2Field { ... on ProjectV2SingleSelectField { options { id name } } } } }' \
  -f projectId="<PROJECT_ID>" \
  -f fieldId="<STATUS_FIELD_ID>" \
  -f options='[{"name":"Draft"},{"name":"Planned"},{"name":"In progress"},{"name":"Review"},{"name":"Blocked"},{"name":"Done"}]'
```

Record the returned option ids as `<OPTION_ID_DRAFT>` through `<OPTION_ID_DONE>` in the Project IDs table.

Fallback: if the built-in Status field options cannot be replaced, create a custom single-select field `Stage` with the same six options and use its field id wherever `<STATUS_FIELD_ID>` is referenced. Note the fallback in the closing comment.

### Read before working

```sh
gh issue list --state open --label "area:<area>" --limit 50
gh issue view <nn> --comments
gh issue list --milestone "M1 Orbit to surface" --state all
```

### Create an issue

```sh
gh issue create \
  --title "<short imperative title>" \
  --label "type:<t>,status:draft,area:<a>,agent:<p>" \
  --milestone "<Mx name>" \
  --body-file <path-to-body.md>
```

Body template:

```md
## Goal
<one paragraph>

## Subtasks
- [ ] Step 1 (owner:<profile>, files:<paths>, after:-)
- [ ] Step 2 (owner:<profile>, files:<paths>, after:Step 1)

## Acceptance criteria
- [ ] AC1: <criterion>

One line per criterion; tick only via the Update acceptance criteria recipe by the reviewer.

## Decisions
- Q: <open question> -> A: <decision> (by:<profile|user>)

Plan resolves all open Q before status:in-progress; any unresolved Q moves the issue to status:blocked.

## References
- docs: <paths>
- related: #<nn>
```

### Sub-issues

```sh
# database id of the child
gh api repos/taftary/universe/issues/<child> -q .id
# attach child to parent
gh api -X POST repos/taftary/universe/issues/<parent>/sub_issues -F sub_issue_id=<child-id>
```

### Update status

```sh
gh issue edit <nn> --remove-label "status:draft" --add-label "status:in-progress"
gh issue comment <nn> --body "<what was done, files touched, decisions>"
gh issue edit <nn> --add-assignee @me
```

### Add to project / set status

Add the issue to the project once at creation, then keep the Project Status in sync with every `status:` label flip.

```sh
# once per issue, at creation
gh project item-add <P> --owner @me --url https://github.com/taftary/universe/issues/<nn>

# on every status flip, together with the matching gh issue edit label change
gh project item-edit --id <ITEM_ID> --field-id <STATUS_FIELD_ID> --project-id <PROJECT_ID> --single-select-option-id <OPTION_ID_IN_PROGRESS>
```

Option mapping: `status:draft` -> `<OPTION_ID_DRAFT>`, `status:planned` -> `<OPTION_ID_PLANNED>`, `status:in-progress` -> `<OPTION_ID_IN_PROGRESS>`, `status:review` -> `<OPTION_ID_REVIEW>`, `status:blocked` -> `<OPTION_ID_BLOCKED>`, `status:done` -> `<OPTION_ID_DONE>`. Resolve `<ITEM_ID>` with `gh project item-list <P> --owner @me --format json` filtered by issue url.

### Update plan subtasks

```sh
gh issue view <nn> --json body -q .body > body.md
gh issue edit <nn> --body-file body.md
gh issue comment <nn> --body "Step <N> done: <what was done, files touched>. Next: Step <M> (owner:<profile>)."
```

Run this after EVERY completed Step before spawning the next profile; do not batch multiple Steps into one update.

Check off the completed Step in `body.md` by changing `- [ ]` to `- [x]`. Keep the `owner:`, `files:`, and `after:` fields unchanged. Update the body file once per completed Step, then comment. The step-done comment is not the handoff block; keep the `files:` and `after:` fields in the issue body. The step-done comment is a pointer only; Subtasks, Acceptance criteria, and Decisions live in the issue body.

### Update acceptance criteria

```sh
gh issue view <nn> --json body -q .body > body.md
gh issue edit <nn> --body-file body.md
gh issue comment <nn> --body "Review: AC1 PASS (<evidence>), AC2 FAIL (<gap>). Next: Step <N+1> (owner:<lead>)."
```

Tick `- [ ] ACn` to `- [x]` in `body.md` only from the reviewer's per-AC PASS list in the review comment; never from the author claim. For each FAIL, append `- [ ] Step N+1 (owner:<lead>, files:<paths>, after:Step N)` under `## Subtasks`, flip the label back with `gh issue edit <nn> --remove-label "status:review" --add-label "status:in-progress"`, and re-review after the fix.

### Develop branch

```sh
gh issue develop <nn> --name <type>/<nn>-<slug> --base main --checkout
```

Branch name format is `<type>/<nn>-<slug>` e.g. `feature/12-atmosphere-profile`. Never develop directly on `main` for tracked work.

### Commit and draft PR

```sh
git add <files>
git commit -m "<type>(<area>): <summary> (#<nn>)"
git push -u origin <type>/<nn>-<slug>
gh pr create --draft --title "<type>(<area>): <summary> (#<nn>)" --body "Closes #<nn>

<what changed, tests>" --label "type:<t>,area:<a>" --milestone "<Mx name>"
```

`Closes #<nn>` as the first line of the PR body is mandatory. It links the PR to the issue and closes it on squash merge.

### CI watch and fix loop

CI jobs are defined in `.github/workflows/ci.yml` (`gates`, `android`, `ios`, `msrv`); the gate order and policy are defined in `docs/tech/quality.md`. Any failure blocks merge. Max 2 fix attempts per PR.

```sh
# watch until green or first failure
gh pr checks <pr> --watch --fail-fast

# list failing jobs
gh pr checks <pr> --json name,state,link -q '.[] | select(.state=="FAILURE")'

# inspect logs (pick the failing run id from gh run list)
gh run list --commit <sha> --status failure
gh run view <run-id> --log-failed
```

Loop rules:

- On failure, append `- [ ] Step N+1 (owner:dev, files:<paths>, after:Step N)` under `## Subtasks`, comment `CI FAIL attempt k/2: <failing job, log excerpt, fix plan>` on the issue, and flip the label back to `status:in-progress` with the matching Project Status `In progress` (see **Add to project / set status**).
- Fix, push, and re-run `gh pr checks <pr> --watch --fail-fast`.
- After 2 failed attempts, set `status:blocked` with the matching Project Status `Blocked`, record the blocker log excerpt in a comment, and return to the user.
- If the failure appears right after a fresh push (checks not yet reported), wait 15 s and retry `gh pr checks <pr> --watch --fail-fast` once before treating it as a real failure.

### Close

```sh
gh pr ready <pr>
gh pr checks <pr> --watch --fail-fast
gh pr merge <pr> --squash --delete-branch
git checkout main
git pull --prune
```

The squash merge carries the `Closes #<nn>` trailer from the draft PR body, which closes the linked issue. Then finish tracking:

```sh
gh issue edit <nn> --remove-label "status:review" --add-label "status:done"
gh project item-edit --id <ITEM_ID> --field-id <STATUS_FIELD_ID> --project-id <PROJECT_ID> --single-select-option-id <OPTION_ID_DONE>
gh issue close <nn> --comment "Done in <commit-sha or PR>. Docs updated: <paths>."
```

The closing comment must post the filled Close checklist: all Acceptance criteria ticked by a non-author reviewer; no open Decisions; one step-done comment per Step; commit sha and docs touched listed.

### Tag and release

```sh
git tag -a v<x.y.z> -m "<release title>"
git push origin v<x.y.z>
gh release create v<x.y.z> --title "<release title>" --generate-notes
gh api -X PATCH repos/taftary/universe/milestones/<number> -f state=closed
```

## Constraints

- Never `git push --force`, never rewrite published history, never delete issues.
- Never commit secrets or tokens.
- One issue per request; use sub-issues for decomposition.
- Never merge with failing checks; `gh pr checks <pr> --watch --fail-fast` must be green first.
- Never push directly to `main` for tracked work; always use the **Develop branch** and **Commit and draft PR** recipes.
