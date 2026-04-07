# Codex CLI Slash Commands

Official docs: `https://developers.openai.com/codex/cli/slash-commands`

Snapshot notes:

- Codex CLI has an in-session slash-command surface distinct from process flags.
- `/permissions` is explicitly documented in the features page as the in-session approval switch.
- `/model` is documented from the models page as the in-session model switch.

RalphX notes:

- RalphX should prefer spawn-time configuration for determinism, not interactive slash commands.
- Slash commands still matter for understanding what Codex sessions may mutate internally when a human attaches to a live process.
