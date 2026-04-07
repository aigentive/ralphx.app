# Codex AGENTS.md Guide

Official docs: `https://developers.openai.com/codex/guides/agents-md`

Snapshot notes:

- Codex supports `AGENTS.md` as a project instruction source.
- Repo instructions are auto-included unless explicitly disabled.

RalphX notes:

- RalphX already depends heavily on `AGENTS.md`; Codex can consume this directly.
- Centralized prompt generation should preserve `AGENTS.md` as repo-level guidance and avoid duplicating those rules inside every spawn prompt.
