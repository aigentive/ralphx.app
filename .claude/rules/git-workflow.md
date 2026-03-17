> **Maintainer note:** This file optimizes for LLM context efficiency. Rules: (1) Tables > prose (2) One example max per concept (3) No redundant explanations (4) Use symbols: → = leads to, | = or, ❌/✅ = wrong/right (5) Before adding content, ask: "Can this be a single line?" If yes, make it one line.

# Git Workflow Rules

**Required Context:** code-quality-standards.md | commit-lock.md

## Critical Rules

1. **NEVER use git stash** — parallel agents run simultaneously; stashing causes conflicts and lost work
2. **Use commit lock protocol** — see commit-lock.md for full details

## Commit Message Conventions

| Prefix | Use Case | Example |
|--------|----------|---------|
| `feat:` | New feature | `feat: add task filtering` |
| `fix:` | Bug fix | `fix: prevent stale question UI` |
| `refactor(scope):` | Restructure / extract / rename | `refactor(http_server): extract handlers` |
| `docs:` | Documentation only | `docs: update API reference` |
| `chore:` | Maintenance / tooling | `chore: backlog maintenance` |
| `test:` | Test-only changes | `test: add merger integration tests` |

## Universal Branch/Merge Rules

| Rule | Detail |
|------|--------|
| Atomic commits | New files + deletions in same commit |
| No partial commits | Code must compile after each commit |
| Commit lock | Acquire `.commit-lock` before `git add`, release after commit — see commit-lock.md |

## Reference

- Full commit lock protocol: commit-lock.md
- Code quality standards: code-quality-standards.md
