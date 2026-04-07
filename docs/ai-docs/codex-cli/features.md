# Codex CLI Features

Official docs: `https://developers.openai.com/codex/cli/features`

Snapshot notes:

- Official docs include sections for interactive mode, local code review, models and reasoning, image inputs, web search, shell completions, approval modes, and Codex Cloud.
- Approval modes can be switched from inside the CLI with `/permissions`.
- The docs explicitly position `gpt-5.4-mini` as a lighter option for subagents.

RalphX notes:

- Codex already has a vendor concept for subagents; this should be used for Codex ideation/verifier parity instead of trying to emulate Claude `Task(...)` literally.
- Codex Cloud features are out of scope for initial RalphX core parity.
- Local code review features may later inform reviewer parity, but initial target remains ideation and verification.
