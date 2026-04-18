> **Maintainer note:** Keep this file compact. Prefer one-line rules, links to source docs, and explicit non-negotiables over prose.

---
paths:
  - "scripts/prompts/**"
  - "scripts/generate-release-notes.sh"
  - "agents/**"
  - "docs/ai-docs/openai/**"
  - "src-tauri/src/infrastructure/agents/**"
  - "src-tauri/src/application/chat_service/**"
---

# OpenAI GPT-5.4 Prompting Rules

Primary reference:
- `docs/ai-docs/openai/gpt-5.4-prompting.md`

| Rule | Detail |
|---|---|
| Use the right source | GPT-5.4 safety/system behavior comes from the GPT-5.4 system card; XML-like prompt-structure guidance comes from OpenAI’s GPT-5 coding guidance, not the system card. |
| Prefer structured prompt files | For long-lived Codex/GPT-5.4 prompt contracts, prefer XML-like sections over flat prose blobs. |
| Keep instruction layers separate | Reusable prompt contract → `model_instructions_file`; lightweight operational guardrails → `developer_instructions`; task facts → user payload. |
| Precision beats intensity | Avoid vague, conflicting, or overly forceful wording; tighten scope instead of telling GPT-5.4 to be “more thorough”. |
| Bound agent eagerness explicitly | State the primary source of truth, secondary context, and any scope limits instead of relying on generic “use judgment” wording. |
| Don’t let prompts sprawl | If prompt quality degrades, remove stale or duplicate instructions before adding new ones. |
