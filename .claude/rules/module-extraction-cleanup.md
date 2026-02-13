---
paths:
  - "src-tauri/src/**/**/mod.rs"
  - "src-tauri/src/application/git_service/**"
  - "src-tauri/src/**/*service*.rs"
---

# Module Extraction Cleanup: Detecting & Removing Orphaned Code

> **Maintainer note:** This file optimizes for LLM context efficiency. Rules: (1) Tables > prose (2) One example max per concept (3) No redundant explanations (4) Use symbols: → = leads to, | = or, ❌/✅ = wrong/right (5) Before adding content, ask: "Can this be a single line?" If yes, make it one line.

## Problem Pattern

When extracting large `impl` blocks into separate leaf modules:
- Functions deleted from parent but **remain orphaned** between two `impl` closures
- Stray attributes like `#[cfg(test)]` break the first `impl` closure prematurely
- Orphaned functions have no impl wrapper → **compile error: "unexpected closing delimiter `}`"**
- Original intent: remove functions from parent. Reality: they're left behind.

### Example (git_service/mod.rs, task 09245033)
```rust
// Line 1515: legitimate impl block closes
impl GitService { /* 500 lines */ } // ✅ correct

// Lines 1517–1899: ORPHANED
#[cfg(test)]  // ❌ stray attribute (misplaced)
// orphaned functions without impl wrapper
fn is_commit_on_branch(...) { }
fn get_commits_since(...) { }
} // ❌ closes nothing (no matching `impl` opened)

// Line 1901: real test module starts
#[cfg(test)]
mod tests { }
```

## Detection Strategy

**Step 1:** Compare impl blocks before/after extraction
```bash
# List all fn in mod.rs
grep -n "^\s*fn " src-tauri/src/application/git_service/mod.rs | head -20

# List all pub fn in extracted modules
grep -n "pub fn " src-tauri/src/application/git_service/query.rs
grep -n "pub fn " src-tauri/src/application/git_service/worktree.rs
```

**Step 2:** Identify gaps
- Functions in mod.rs that should be in extracted modules → **duplicates**
- Stray attributes (e.g., `#[cfg(test)]`) between impl closures → **structural break**
- Unmatched `}` at unexpected lines → **scope corruption**

**Step 3:** Verify with compile errors
```bash
timeout 10m cargo test --lib --manifest-path src-tauri/Cargo.toml 2>&1 | grep -A5 "unexpected closing"
```

## Resolution

**Remove entire orphaned block:**
1. Identify exact line range (stray attribute start → orphaned `}` end)
2. Verify functions in block are fully defined in extracted modules (no partial moves)
3. Delete the entire range in one edit
4. Verify compile succeeds

**Example from task 09245033:**
- Removed lines 1517–1899 (orphaned duplicates + stray attribute + unmatched `}`)
- 384 lines deleted
- Result: 5393 → 5009 lines, all tests pass

## Prevention Checklist

| Step | Action | Verify |
|------|--------|--------|
| Extract module | Create new leaf file with functions | `pub fn` signatures match extracted body |
| Update mod.rs | Delete functions from parent | Grep confirms no duplicates in parent |
| Verify structure | Check impl block boundaries | No stray attributes between impl closing `}` and next item |
| Compile | Run `cargo test --lib` | 0 compile errors, structure is clean |
| Check whitespace | Ensure no orphaned blank lines or comments | Clean section removed |

## Quick Test

After cleanup, verify with focused test:
```bash
# Test the affected module
cargo test --lib git_service
# Should see: test result: ok. N passed; 0 failed
```

If compile fails with "unexpected closing delimiter", **you likely have an orphaned block**. Follow detection strategy above.

## Related Patterns

- **Partial extraction**: Only *some* functions moved. → Check all cross-module calls
- **Misplaced attributes**: `#[cfg(...)]` sits between impl blocks. → Remove or move to function
- **Comments left behind**: Old comments referencing deleted code. → Clean up as part of deletion

## Task Reference

- **Task 09245033** (`Create git_cmd utility and extract leaf git_service modules`): Applied this pattern, fixed git_service/mod.rs after extraction left 384 orphaned lines. Result: clean extraction with all tests passing.
