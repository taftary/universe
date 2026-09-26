# gh-orchestrator

Owns every interaction with GitHub and every git write operation. Other agents never run these commands directly; they call the recipes below.

Repository: `taftary/universe`. All `gh` commands run from the repository root.

## Preflight

Run in order. Stop at the first failure and follow its instructions.

| Step | Command | On failure |
| --- | --- | --- |
| 1 | `gh --version` | `gh` not installed -> go to **Install** |
| 2 | `gh auth status` | not logged in -> run `gh auth login` interactively with the user |
| 3 | `gh repo view --json nameWithOwner -q .nameWithOwner` | must print `taftary/universe`; otherwise check `git remote -v` |

Return `gh-available` when all three pass.

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

### Branches and commits

- Branch: `<type>/<issue>-<slug>` e.g. `feature/12-atmosphere-profile`. Small doc-only changes may go directly on `main`.
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
- [ ] <subtask 1>
- [ ] <subtask 2>

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

### Branch, commit, PR

```sh
git checkout -b <type>/<nn>-<slug>
git add <files>
git commit -m "<type>(<area>): <summary> (#<nn>)"
git push -u origin <type>/<nn>-<slug>
gh pr create --fill --label "type:<t>,area:<a>" --milestone "<Mx name>"
gh pr merge --squash --delete-branch
```

### Close

```sh
gh issue edit <nn> --remove-label "status:in-progress" --add-label "status:done"
gh issue close <nn> --comment "Done in <commit-sha or PR>. Docs updated: <paths>."
```

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
