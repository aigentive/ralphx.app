# Codex Config Advanced

Official docs: `https://developers.openai.com/codex/config-advanced`

Snapshot notes:

- Advanced config expands the same `config.toml` surface for more specialized control.
- Official docs split basic vs advanced vs full reference, which matters for layered RalphX config generation.

RalphX notes:

- The multi-harness layer should map RalphX settings onto generated Codex config fragments rather than leaking raw vendor keys through the entire app.
- Advanced config is the likely place to mirror per-agent or per-profile Codex overrides later.
