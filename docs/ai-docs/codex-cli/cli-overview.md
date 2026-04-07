# Codex CLI Overview

Official docs: `https://developers.openai.com/codex/cli`

Snapshot notes:

- Codex CLI is the terminal interface for local interactive Codex sessions.
- Current official docs position it alongside CLI features, command-line options, and slash commands.
- RalphX impact: this is the interactive harness candidate for ideation / verification parity, but not a drop-in replacement for Claude team mode.

RalphX notes:

- Codex must be treated as a distinct harness, not a Claude flag variant.
- Official docs expose a richer CLI than the locally installed binary in this environment.
- RalphX should capability-detect the Codex binary version before enabling advanced paths.
