# Git Workflow Rules

**Required Context:** @.claude/rules/code-quality-standards.md | @.claude/rules/commit-lock.md

> Shared git rules for all streams. Stream-specific ownership rules are defined in each stream file.

## Critical Rules

1. **NEVER use git stash** — parallel agents run simultaneously; stashing causes conflicts and lost work
2. **Use commit lock protocol** — see @.claude/rules/commit-lock.md for full details
3. **Only recover YOUR work** — uncommitted files belong to the stream whose backlog/PRD mentions them

## Commit Lock Summary

Before any commit:
```bash
PROJECT_ROOT="$(git rev-parse --show-toplevel)"
# Check/acquire lock at $PROJECT_ROOT/.commit-lock
# See commit-lock.md for full protocol
```

After commit (success or failure):
```bash
rm -f "$PROJECT_ROOT/.commit-lock"
```

## Recovery Check (Code-Writing Streams Only)

**Applies to:** features, refactor, polish streams
**Does NOT apply to:** verify, hygiene streams (they don't write code)

Before starting normal workflow, check for incomplete work from a previous iteration:

```
1. Run: git status --porcelain
   → No uncommitted changes? → Skip recovery, proceed to normal workflow

2. Read your stream's backlog (and PRD if applicable)
   → See stream-specific ownership rules below

3. For each uncommitted file, check ownership:
   → Matches your backlog/PRD? → YOURS — complete and commit
   → No match? → NOT yours — leave alone

4. After handling matched files (or if none matched):
   → Proceed to normal workflow
```

## Stream-Specific Ownership Rules

Each stream defines what "matches" means for step 3:

### Features Stream
```
Ownership sources:
- streams/features/backlog.md (P0 items)
- Active PRD task files
- Files in streams/features/ or specs/phases/

Match if: File path appears in P0 item OR active PRD task OR is a features stream file
```

### Refactor Stream
```
Ownership source:
- streams/refactor/backlog.md (P1 items)

Match if: File path contains a module/path mentioned in any P1 backlog item
Example: Backlog has "http_server" item → http_server/handlers/foo.rs is YOURS
```

### Polish Stream
```
Ownership source:
- streams/polish/backlog.md (P2/P3 items)

Match if: File path matches a backlog item's file:line reference
```

## Commit Message Conventions

| Stream | Prefix | Example |
|--------|--------|---------|
| features | `feat:` `fix:` `docs:` | `feat: add task filtering` |
| refactor | `refactor(scope):` | `refactor(http_server): extract handlers` |
| polish | `refactor(scope):` | `refactor(api): fix type safety` |
| verify | `chore(verify):` | `chore(verify): add P0 items from phase 12` |
| hygiene | `chore(hygiene):` | `chore(hygiene): backlog maintenance` |

## Reference

- Full commit lock protocol: @.claude/rules/commit-lock.md
- Code quality standards: @.claude/rules/code-quality-standards.md
