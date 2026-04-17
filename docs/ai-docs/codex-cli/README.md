# Codex CLI Integration Notes

RalphX does not integrate with Codex CLI yet. This directory is the local snapshot for the Codex CLI surfaces that matter to RalphX parity planning.

Scope:

- Official Codex docs remain the source of truth when vendor behavior matters.
- Local files here are condensed topic notes so agents can grep-discover Codex capabilities from inside the repo.
- RalphX-specific compatibility notes live alongside the official links.

Current status:

- Snapshot started on 2026-04-07.
- Official docs and the locally installed `codex` binary are already divergent.
- The installed binary here is `codex 0.1.2505172129` via Homebrew cask `0.116.0`; its `--help` output is much smaller than the current official docs tree.
- RalphX should treat Codex capability detection as version-sensitive instead of assuming one stable CLI contract.

Primary official docs:

- Codex docs root: `https://developers.openai.com/codex`
- CLI overview: `https://developers.openai.com/codex/cli`
- Config basics: `https://developers.openai.com/codex/config-basic`
- Config reference: `https://developers.openai.com/codex/config-reference`

Local files in this directory:

- `cli-overview.md`, `cli-reference.md`, `features.md`, `slash-commands.md`
- `config-basics.md`, `config-advanced.md`, `config-reference.md`, `config-sample.md`
- `authentication.md`, `approvals-security.md`, `sandboxing.md`
- `agents-md.md`, `rules.md`, `hooks.md`, `mcp.md`, `skills.md`, `subagents.md`
- `models.md`, `noninteractive.md`
- `index.txt`
