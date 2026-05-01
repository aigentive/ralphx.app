> **Maintainer note:** Keep this file compact. PR bodies are for reviewers and users; CI is for command logs.

# PR Description Rules

## Required Shape

| Section | Detail |
|---|---|
| Summary | 2-4 bullets: what changed + why it matters |
| User Impact | Visible behavior, product workflow, or failure mode addressed |
| Technical Context | Root cause, important architecture decisions, intentional scope limits |
| Risks / Follow-Ups | Compatibility, migrations, rollout risk, known gaps |

## Rules

| Rule | Detail |
|---|---|
| Impact first | Lead with context, user-facing changes, and why the change matters |
| Validation secondary | Do not dump command transcripts; CI is the source of truth for routine validation |
| Manual evidence only | Mention manual/visual validation only when it adds review value beyond CI |
| No agent diary | Omit implementation chronology, "I ran...", and raw local terminal output |
| Explicit scope | State meaningful non-goals or deferred work when reviewers might expect them |
