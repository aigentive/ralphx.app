---
name: code-review-checklist
description: Code quality and security review checklist
disable-model-invocation: true
user-invocable: false
---

# Code Review Checklist

## Correctness

- [ ] Code does what it's supposed to do
- [ ] Edge cases are handled
- [ ] Error handling is appropriate
- [ ] No off-by-one errors
- [ ] Async operations complete properly

## Code Quality

### Clarity
- [ ] Names are descriptive and consistent
- [ ] Functions do one thing
- [ ] Complex logic has comments
- [ ] No dead code or TODOs

### Structure
- [ ] Appropriate abstractions
- [ ] No deep nesting (max 3 levels)
- [ ] Functions under 50 lines
- [ ] Files under 150 lines

### Patterns
- [ ] Follows existing patterns
- [ ] No reinvented wheels
- [ ] DRY principles applied

## Testing

- [ ] New code has tests
- [ ] Edge cases covered
- [ ] Tests are meaningful
- [ ] Tests pass reliably

## Security

### Data Handling
- [ ] No hardcoded secrets
- [ ] Sensitive data not logged
- [ ] Input validated at boundaries

### Injection Prevention
- [ ] SQL queries use parameters
- [ ] Commands sanitize input
- [ ] URLs are validated

### Authentication
- [ ] Auth checks present
- [ ] Permissions verified
- [ ] Session handling correct

## Performance

- [ ] No obvious N+1 queries
- [ ] Expensive operations cached
- [ ] No memory leaks
- [ ] Efficient algorithms used

## Documentation

- [ ] Public APIs documented
- [ ] Complex logic explained
- [ ] README updated if needed

## Review Output

Use this template:

```
## Decision
- [ ] Approve
- [ ] Needs Changes
- [ ] Escalate to Human

## Findings

### Critical (blocks merge)
- [issue description, file:line]

### Important (should fix)
- [issue description, file:line]

### Minor (nice to have)
- [issue description, file:line]

## Summary
[Brief overall assessment]
```
