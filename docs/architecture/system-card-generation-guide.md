# System Card Generation Guide

> Instructions for Claude to analyze conversation transcripts and produce a system card documenting orchestration patterns. Invoke with: "Generate a system card for conversation(s) [session ID or commit range]"

---

## 1. Trigger Phrases

Any of these should activate this guide:
- "generate a system card for..."
- "analyze this conversation and create a system card"
- "document the orchestration pattern from..."
- "create a system card for commits [range]"

---

## 2. Data Sources

### Conversation Logs (JSONL)

| Source | Path |
|--------|------|
| Project logs | `~/.claude/projects/-Users-example-Code-ralphx/*.jsonl` |
| Plan files | `~/.claude/plans/*.md` |
| Session metadata | `~/.claude/projects/-Users-example-Code-ralphx/<uuid>/subagents/` |

**684 JSONL files** (as of 2026-02-13), UUID-named, 0.9 KB to 20.8 MB each.

### Git History

```bash
git log --format="%H %ai %s" --shortstat <start_sha>^..<end_sha>
```

### Identifying the Right JSONL Files

If the user provides **commit SHAs**, find the JSONL files by matching timestamps:
```python
# Get commit timestamps first
git log --format="%ai %s" <sha>

# Then search JSONL files by date
python3 -c "
import json, os, glob
target_date = '2026-02-13'  # from commit timestamps
for f in sorted(glob.glob('/Users/example/.claude/projects/-Users-example-Code-ralphx/*.jsonl')):
    try:
        with open(f) as fh:
            first = json.loads(fh.readline())
            ts = first.get('timestamp', '')
            if target_date in ts:
                size = os.path.getsize(f) / 1024
                print(f'{size:.0f}KB {os.path.basename(f)} started={ts[:19]}')
    except: pass
"
```

If the user provides a **session ID** (UUID), the file is directly `<uuid>.jsonl`.

---

## 3. Extraction Strategy — Parallel Agents

**MANDATORY:** Launch 4+ agents in parallel to mine data. Never do sequential exploration.

### Agent 1: JSONL Session Analyzer (per session)

**Type:** `general-purpose` | **Background:** `true`

**Prompt template:**
```
Read the JSONL transcript at [PATH].

Extract ALL structured data:
1. Exact timestamps for each lifecycle phase (discovery, plan, approval, execution, verification)
2. Every Task tool call (agent spawns) — description, subagent_type, background, prompt length, prompt preview (500 chars)
3. Every EnterPlanMode/ExitPlanMode event with timestamps
4. All human messages (type=user where content is a string, not tool_result)
5. All tool_result rejections (content containing "user doesn't want to proceed")
6. Context continuation events ("continued from a previous conversation")
7. TaskCreate/TaskUpdate sequences with timestamps
8. Error patterns (type errors, test failures, recovery actions)

Use this Python extraction pattern:
```python
import json
with open('[PATH]') as f:
    lines = f.readlines()
for i, line in enumerate(lines):
    obj = json.loads(line)
    # Filter by obj['type'] and obj['message']['content']
```

Return everything as structured tables.
```

### Agent 2: Agent Prompt Extractor (per session)

**Type:** `general-purpose` | **Background:** `true`

**Prompt template:**
```
Read the JSONL transcript at [PATH].

Find and extract every agent dispatch prompt. Search for tool_use with name='Task':
```python
import json
with open('[PATH]') as f:
    for i, line in enumerate(f):
        obj = json.loads(line)
        if obj.get('type') != 'assistant': continue
        content = obj.get('message', {}).get('content', [])
        if not isinstance(content, list): continue
        for item in content:
            if isinstance(item, dict) and item.get('type') == 'tool_use' and item.get('name') == 'Task':
                inp = item.get('input', {})
                ts = obj.get('timestamp', '')[:19]
                print(f"=== {ts} | {inp.get('description','')} | {inp.get('subagent_type','')} ===")
                print(f"Background: {inp.get('run_in_background', False)}")
                print(f"Prompt ({len(inp.get('prompt',''))} chars):")
                print(inp.get('prompt', '')[:2000])
                print()
```

Also extract:
- Agent completion notifications (search for "task-notification" in user messages)
- Usage stats (search for "<usage>" tags with tool_uses, tokens, duration)
- The STRICT SCOPE pattern if present in any prompt

Return the complete agent prompt template and all dispatch examples.
```

### Agent 3: Git History Analyzer

**Type:** `Bash` | **Background:** `true`

**Prompt template:**
```
Extract real git data for commits [START_SHA] through [END_SHA]:

1. Full commit messages with bodies:
   git log --format="%H%n%ai%n%s%n%b%n---" [START]^..[END]

2. Per-commit file stats:
   git log --format="%h %s" --shortstat [START]^..[END]

3. Aggregate diff:
   git diff --stat [START]^..[END]
   git diff --shortstat [START]^..[END]

4. Per-commit shortstat:
   git show --shortstat --format="%h %ai %s" [SHA] (for each commit)

5. All commits with timestamps as table:
   git log --format="| %h | %ai | %s |" [START]^..[END]

Return all raw git data.
```

### Agent 4: Plan File Analyzer

**Type:** `general-purpose` | **Background:** `true`

**Prompt template:**
```
Find and read the plan files used in these sessions.

1. Check ~/.claude/plans/*.md — read the first 3 lines of each to find plans matching the topic
2. Read the full content of matching plan files
3. Extract:
   - Plan structure (sections, phases/tiers)
   - Agent assignment tables (Create/Modify/Delete/Must NOT touch columns)
   - Conflict prevention rules
   - Agent prompt template
   - Dependency graph
   - Verification commands

Also check for plan content embedded in JSONL ExitPlanMode events:
```python
import json
with open('[PATH]') as f:
    for i, line in enumerate(f):
        obj = json.loads(line)
        if obj.get('type') != 'assistant': continue
        content = obj.get('message', {}).get('content', [])
        if isinstance(content, list):
            for item in content:
                if isinstance(item, dict) and item.get('name') == 'ExitPlanMode':
                    plan = item.get('input', {}).get('plan', '')
                    print(f"Line {i}: Plan ({len(plan)} chars)")
                    print(plan[:3000])
```

Return the complete plan text and structural analysis.
```

---

## 4. JSONL Record Types Reference

Key record structures to parse:

| `type` | `message.content` | What It Contains |
|--------|-------------------|-----------------|
| `assistant` | `[{type: "tool_use", name: "Task", input: {prompt, subagent_type, ...}}]` | Agent dispatch |
| `assistant` | `[{type: "tool_use", name: "ExitPlanMode", input: {plan: "..."}}]` | Plan submission |
| `user` | `[{type: "tool_result", content: "The user doesn't want to proceed..."}]` | Human rejection |
| `user` | `string` (not array) | Direct human message |
| `user` | `[{type: "tool_result", content: "agentId: ... <usage>...</usage>"}]` | Agent completion |
| `user` | `string` containing "continued from a previous conversation" | Context overflow recovery |
| `assistant` | `[{type: "tool_use", name: "TaskCreate"}]` | Task registration |
| `assistant` | `[{type: "tool_use", name: "Bash", input: {command: "git commit..."}}]` | Commit event |

---

## 5. System Card Template

After all agents return, compose the document using this structure. Every section must use **real data from the agents** — never fabricate metrics.

```markdown
# System Card: [Title — Pattern Name]

> Derived from [N] sessions on [date]. Grounded in JSONL logs [session IDs],
> plan files [names], and git history [commit range].

## 1. System Overview
- 1 paragraph + architecture diagram (ASCII)
- Key finding about coordinator vs. subagent division of labor

## 2. Lifecycle Phases
- Table: Phase | Name | Observed Duration | Key Mechanics
- Note on plan rejection patterns

## 3. Agent Taxonomy
- Table: Type | Tools | Scope | Observed Usage
- Subagent performance table: Agent | tool_uses | tokens | duration

## 4. Parallel Execution Model
- Wave table (if phase-driven): Wave | Agents | Files | Gate
- Independent model (if tier-driven): Agent | Tier | File Scope
- Conflict prevention rules (numbered, from plan)

## 5. Plan Anatomy
- Agent prompt template (verbatim from plan/JSONL)
- Plan archetypes table

## 6. Human Steering Model
- Per-session table: # | Timestamp | Intervention | Effect
- Note: count mid-execution interventions (usually zero)

## 7. TDD Integration
- Table: Pattern | Session | Flow | Observed test counts at checkpoints

## 8. Commit Strategy
- Per-session commit table: Hash | Timestamp | Message | +/-

## 9. Tool Usage Patterns
- Tool distribution table: Tool | Count | % | Primary Phase
- Typical coordinator sequence (pipeline diagram)

## 10. Metrics & Benchmarks
- Comparison table: Metric | Session 1 | Session 2
- Include: duration, commits, agents, files, lines, tests, interventions, errors

## 11. Anti-Patterns & Failure Modes
- Table: Anti-Pattern | Observed Risk | Mitigation
- Ground each in actual recovery events from the sessions

## 12. Reproducible Process — Checklist
- 7-9 numbered steps, each grounded in observed data
```

---

## 6. Composition Rules

1. **Tables > prose** — every concept that can be a table row, make it a table row
2. **No section > 30 lines** — if a section grows too large, compress to table format
3. **Real data only** — every number must come from git, JSONL, or plan files. Never estimate.
4. **One example max per concept** — cite the best observed instance, not all instances
5. **Symbols:** `→` = leads to, `|` = or, `+/-` = lines added/deleted
6. **Cross-reference sections** with `(§N)` notation
7. **Include exact commit hashes** — short form (8 chars) in tables, full in commit detail sections
8. **Include exact UTC timestamps** — from JSONL `timestamp` fields
9. **Include agent IDs** — from `agentId` in completion notifications (for reproducibility)
10. **Document deviations** — when execution differs from plan (e.g., coordinator absorbs planned agent work)

---

## 7. Verification Checklist

Run after document is written:

```bash
# All 12 sections present
grep -c '^## ' [file]  # must be 12

# No section exceeds 30 lines
awk '/^## /{if(name)print count, name; name=$0; count=0; next} {count++} END{print count, name}' [file]

# Tables outnumber prose (table lines vs total lines)
grep -c '|' [file]

# Total length reasonable (200-300 lines)
wc -l [file]
```

Additionally verify:
- [ ] Every commit hash in the doc exists in `git log`
- [ ] Every timestamp can be traced to a JSONL line
- [ ] Agent counts match actual Task tool_use calls in JSONL
- [ ] Test counts match actual test run output in Bash tool results
- [ ] Lines added/deleted match `git diff --shortstat`

---

## 8. Multi-Session Handling

When analyzing sessions that span multiple JSONL files (e.g., context overflow → new session):

1. **Find all related files** — match by date + topic (grep first lines for plan names or commit references)
2. **Launch separate agents per JSONL file** — don't ask one agent to read multiple large files
3. **Identify continuation points** — search for "continued from a previous conversation" strings
4. **Merge timelines** — combine timestamps from all sessions into one chronological view
5. **Track what carried over** — files on disk persist; context/memory does not

---

## 9. Scaling to N Sessions

For analyzing more than 2 sessions:

| Sessions | Agent Strategy |
|----------|---------------|
| 1 | 2 JSONL agents + 1 git agent + 1 plan agent = 4 parallel |
| 2 | 2 JSONL agents per session + 1 git + 1 plan = 6 parallel |
| 3+ | 2 JSONL agents per session + 1 git + 1 plan = 2N+2 parallel |

Always launch JSONL + git + plan agents in a **single message** for maximum parallelism.

---

## 10. Example Invocation

User says: "Generate a system card for the work between commits abc1234 and def5678"

Claude should:
1. Run `git log --format="%ai %s" abc1234^..def5678` to get date range and commit messages
2. Find JSONL files matching that date range (Python script from §2)
3. Find plan files matching topic keywords from commit messages
4. Launch 4+ parallel agents (§3) in a **single message**
5. Wait for all agents to complete
6. Compose system card using template (§5) with real data from agents
7. Run verification checklist (§7)
8. Present final document

**Total time target:** 3-5 minutes (parallel agent execution) + 1-2 minutes (composition + verification)
