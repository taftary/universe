# GLOBAL AGENT

This file is the single entry point for every AI agent, model or IDE working on this repository.
All IDE-specific files (`CLAUDE.md`, `GEMINI.md`, `.cursor/`, `.github/copilot-instructions.md`, ...) only point here.

## Project

**universe** — a seamless, real-scale, physics-first survival game. Mobile-first, solo-buildable.
Read [README.md](README.md) for the vision and [docs/README.md](docs/README.md) for the documentation index.

## Routing rule

1. If the request is a **change, fix, feature, plan, task, audit, report or idea** — anything that creates or modifies project artefacts — **stop and follow [docs/agents/orchestrator.md](docs/agents/orchestrator.md)** before doing any work.
2. If the request is a **question or read-only exploration**, answer directly. Do not open issues, do not commit.

## Ground rules

- GitHub Issues are the single source of truth for status and history. Read the relevant issue(s) before working; update them when done.
- Never commit, tag, branch or release outside the [gh-orchestrator](docs/agents/gh-orchestrator.md) recipes.
- Documents under `docs/` are references; keep them in sync with closed issues.
- Plain Markdown, English, no emojis. Keep files small and specific.
- Agents may update their own definition files when they detect a gap, following the self-update rule in [docs/agents/profiles/_template.md](docs/agents/profiles/_template.md).
