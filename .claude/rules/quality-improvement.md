# Quality Improvement Enforcement

## Mandatory Process (ENFORCED BY HOOK)

Every task that modifies code MUST include a quality improvement:

1. **Scan** — Launch Explore agent to scan codebase subset for issues
2. **Pick** — Select ONE actionable improvement
3. **Fix** — Execute the improvement
4. **Commit** — Create commit with `refactor:` prefix

## Quality Targets

### Frontend (src/)
- Replace `any` with proper types
- Fix naming inconsistencies
- Add missing error handling
- Remove dead code
- Extract repeated logic into hooks/functions
- Fix lint warnings

### Backend (src-tauri/)
- Fix clippy warnings
- Improve error handling (domain-specific variants)
- Fix naming inconsistencies
- Remove dead code
- Extract repeated logic into helpers

## Scope Guidelines

| Task Size | Improvement Scope |
|-----------|-------------------|
| Small (1-2 files, <50 LOC) | Single lint fix or type improvement |
| Medium (3-5 files, 50-150 LOC) | Extract a helper or fix error handling |
| Large (>5 files, >150 LOC) | Refactor pattern or extract module |

## Skip Conditions

Quality improvement is NOT required for:
- Pure research/exploration (no code changes)
- Documentation-only changes
- Configuration changes

## Verification

A `Stop` hook verifies compliance. Task will not complete without `refactor:` commit when code was modified.
