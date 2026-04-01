---
name: priority-assessment
description: Guide for calculating and explaining task priority scores
---

# Priority Assessment Skill

This skill helps calculate meaningful priority scores for tasks based on multiple factors.

## Priority Score Formula

Priority scores range from 0-100 and are calculated from five factors:

```
Priority Score = Dependency (30) + Critical Path (25) + Business Value (20) + Complexity (15) + User Hints (10)
```

## Factor Breakdown

### 1. Dependency Factor (Max 30 points)

How many other tasks does this task block?

| Blocks | Score | Reasoning |
|--------|-------|-----------|
| 0 tasks | 0 | No other tasks waiting on this |
| 1 task | 10 | Minor blocker |
| 2 tasks | 18 | Moderate blocker |
| 3 tasks | 24 | Significant blocker |
| 4+ tasks | 30 | Critical path item |

**Example**: "Create auth context" blocks login, logout, protected routes, and session persistence = 30 points

### 2. Critical Path Factor (Max 25 points)

Is this task on the longest path to completion?

| Position | Score | Reasoning |
|----------|-------|-----------|
| Not on critical path | 0 | Can be done in parallel |
| On critical path, short | 10 | Part of main sequence |
| On critical path, medium | 18 | Important stepping stone |
| On critical path, start | 25 | Must be done first |

**Example**: "Define database schema" at the start of data layer critical path = 25 points

### 3. Business Value Factor (Max 20 points)

How much value does this deliver to users/stakeholders?

| Value | Score | Indicators |
|-------|-------|------------|
| Low | 0-5 | Technical debt, internal only |
| Medium | 6-12 | Incremental improvement |
| High | 13-17 | Core functionality |
| Critical | 18-20 | User-facing, revenue-impacting |

**Keywords that suggest high value**:
- "user", "customer", "revenue"
- "security", "performance"
- "core", "essential", "required"
- "blocking", "urgent"

**Example**: "Add payment processing" = 20 points (revenue-critical)

### 4. Complexity Factor (Max 15 points)

Simpler tasks should be done first (higher priority) when other factors are equal.

| Complexity | Score | Characteristics |
|------------|-------|-----------------|
| Trivial | 15 | <30 min, config change |
| Simple | 12 | 1-2 hours, single file |
| Moderate | 8 | 2-4 hours, multiple files |
| Complex | 4 | 4+ hours, architecture |
| Very Complex | 0 | Multi-day, high risk |

**Reasoning**: Quick wins build momentum and reduce backlog.

**Example**: "Fix typo in error message" = 15 points (trivial fix)

### 5. User Hint Factor (Max 10 points)

Did the user express urgency or importance?

| Signal | Score |
|--------|-------|
| Explicit "urgent", "ASAP", "critical" | 10 |
| "Important", "need this soon" | 7 |
| "Nice to have", "eventually" | 2 |
| No signal | 5 (neutral) |

**Example**: "We need this for the demo next week" = 10 points

## Priority Levels

Map scores to priority levels:

| Score Range | Level | Action |
|-------------|-------|--------|
| 85-100 | Critical | Do immediately |
| 65-84 | High | Do soon |
| 40-64 | Medium | Normal queue |
| 20-39 | Low | When time permits |
| 0-19 | Trivial | Backlog |

## Example Assessment

**Task**: "Create user authentication context"

**Factor Analysis**:
1. **Dependency (30)**: Blocks 4 tasks (login, logout, protected routes, session) = 30/30
2. **Critical Path (25)**: First item in auth chain = 25/25
3. **Business Value (20)**: Enables all user features = 18/20
4. **Complexity (15)**: Moderate (2-3 hours, state management) = 8/15
5. **User Hints (10)**: User said "auth is a prerequisite" = 7/10

**Total**: 30 + 25 + 18 + 8 + 7 = **88/100** (Critical)

**Reason**: "Critical blocker for authentication chain. Must be completed first to unblock login, logout, and protected routes. Moderate complexity but high value - enables all user-specific features."

## Priority Explanation Template

When explaining priority to users:

```
[Task Title] - Score: [X]/100 ([Level])

This task scores [high/medium/low] because:
- [Key factor 1]: [explanation]
- [Key factor 2]: [explanation]

[If blocking]: This must be done before [dependent tasks].
[If blocked]: This depends on [blocker tasks] being completed first.
```

## Common Priority Patterns

### Foundation First
Infrastructure and types typically score high due to dependency factor:
- Database schemas
- Type definitions
- API contracts
- Shared utilities

### User Value Wins
When dependencies are equal, prioritize user-facing work:
- Visible features over internal refactoring
- Bug fixes over new features
- Performance improvements users notice

### Quick Wins Matter
Simple, high-value tasks should be prioritized:
- Easy bug fixes
- Configuration changes
- Copy updates
- Small UX improvements

## Anti-Patterns

**Don't**:
- Prioritize based only on user excitement
- Ignore technical dependencies
- Let all tasks become "high priority"
- Forget to reassess after scope changes

**Do**:
- Consider the full picture
- Update priorities when dependencies change
- Communicate trade-offs clearly
- Revisit priorities as context evolves
