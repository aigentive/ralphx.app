# Memory Management Framework

> Reference document for /rule-manager and /knowledge-capture skills.
> Based on Claude Code official documentation: https://code.claude.com/docs/en/memory

## How Claude Code Memory Works

### Auto-Discovery (.claude/rules/)
- All .md files in `.claude/rules/` are recursively auto-loaded at session start
- Same priority as `.claude/CLAUDE.md`
- No @ references needed between rule files — auto-discovery handles loading
- @ references between rule files are REDUNDANT and waste tokens via double-loading

### Path-Specific Scoping (CRITICAL for optimization)
YAML frontmatter limits when a rule loads:
```yaml
---
paths:
  - "src/api/**/*.ts"
  - "src-tauri/src/http_server/**"
---
```
- Without paths: -> loads globally (every session, every task)
- With paths: -> loads only when Claude works with matching files
- Supported patterns: `**` (any dirs), `*` (any chars), `{a,b}` (alternatives)

### @ Import Syntax
- `@path/to/file` imports file content into context
- Relative paths resolve relative to the FILE containing the import (not cwd)
- Max depth: 5 recursive hops
- First-time approval dialog per project
- NOT evaluated inside code blocks or backticks

### Auto-Memory
- Location: `~/.claude/projects/<project>/memory/MEMORY.md`
- First 200 lines loaded at startup (keep concise!)
- Topic files (debugging.md, etc.) loaded on demand
- Format: `## Heading` + bullet points

### CLAUDE.md Hierarchy (precedence order)
1. Managed policy (org-wide, highest priority)
2. Project memory (`./CLAUDE.md` or `./.claude/CLAUDE.md`)
3. Project rules (`.claude/rules/*.md`)
4. User memory (`~/.claude/CLAUDE.md`)
5. Project local (`./CLAUDE.local.md`, gitignored, overrides project)

### Loading Behavior
- Parent directories: loaded at launch (recursive upward)
- Child directories: loaded on demand when Claude reads files there
- More specific instructions take precedence over broader ones

## Self-Management Principles

### 1. Specialize and Scope
- Global rules: ONLY for truly universal standards (aim for <6 global files)
- Domain rules: MUST use paths: frontmatter
- Split large rules (>400 LOC) into focused sub-rules with tight scoping
- One topic per file, descriptive kebab-case filenames

### 2. No @ References Between Rules
- Auto-discovery loads everything in .claude/rules/ — @ is redundant
- @ between rules creates wasteful double-loading
- Use plain text: "See filename.md" for cross-references
- ONLY use @ in CLAUDE.md for files OUTSIDE .claude/rules/

### 3. Deprecate What's Unused
- Rules referencing files/patterns that no longer exist -> archive or delete
- Rules that haven't been relevant in multiple sessions -> evaluate removal
- Stale rules waste context tokens on every session

### 4. Capture Knowledge from Sessions
- Complex discoveries -> new scoped rules (not just auto-memory)
- Format: ## Heading + Problem/Fix/File/Pattern bullets
- Always include paths: frontmatter based on which files the knowledge relates to
- Append to existing domain rules when possible, create new only for new domains

## Optimization Priority List
1. Remove @ references between rule files (instant win, no risk)
2. Add paths: scoping to domain-specific rules (biggest token savings)
3. Split rules >400 LOC into focused sub-rules with paths:
4. Deprecate stale rules (reference files that don't exist)
5. Create new rules from session learnings (knowledge capture)
6. Tighten existing paths: patterns (reduce false-positive loading)

## Knowledge Entry Format (matches auto-memory style)
```markdown
## [Descriptive Title]
- **Problem**: What went wrong or was complex
- **Fix**: How it was resolved
- **File**: Key file paths involved
- **Pattern**: Reusable pattern or principle (if applicable)
```

## Rule Health Metrics
| Metric | Healthy | Action |
|--------|---------|--------|
| Total rule tokens | <20k | Monitor |
| Total rule tokens | >25k | Run /rule-manager |
| Single rule LOC | <400 | OK |
| Single rule LOC | >400 | Split |
| Rules without paths: | <6 | OK (truly global) |
| Rules without paths: | >6 | Evaluate scoping |
| @ refs between rules | 0 | OK |
| @ refs between rules | >0 | Remove (priority 1) |

## New Rule Checklist
- [ ] Has paths: frontmatter (unless truly global)
- [ ] Under 400 lines
- [ ] No @ references to other rule files
- [ ] One focused topic per file
- [ ] Descriptive kebab-case filename
- [ ] Knowledge entries use ## Heading + bullet format
