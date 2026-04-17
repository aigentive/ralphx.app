# Codex Models

Official docs: `https://developers.openai.com/codex/models`

Snapshot notes:

- Official docs recommend `gpt-5.4` for most Codex tasks.
- Official docs recommend `gpt-5.4-mini` for faster, lower-cost tasks and subagents.
- The docs state that Codex CLI and SDK use the same model configuration surface.

RalphX notes:

- RalphX should default Codex to `gpt-5.4`.
- Initial preferred verifier / specialist subagent default should be `gpt-5.4-mini` unless audit evidence says a stronger model is required.
- Codex reasoning effort should be configured separately from the model name.
