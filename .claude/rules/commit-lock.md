# Commit Lock Protocol

## Overview

When multiple streams run in parallel, they must coordinate commits to avoid conflicts. A simple lock file prevents simultaneous commit attempts.

## Lock File

**Location:** `.commit-lock` (project root)

**Format:** Single line with stream name and timestamp
```
<stream-name> <ISO-timestamp>
```

Example: `features 2026-01-29T10:30:45`

## CRITICAL: Project Root Requirement

**ALL lock operations and git commands MUST use absolute paths to the project root.**

Agents may be working in subdirectories. To ensure the lock protocol works:

1. **Determine project root** at the start of any commit operation using `git rev-parse --show-toplevel`
2. **Use absolute paths** for all lock file operations
3. **Run git commands from project root** or use `-C` flag

```bash
# At start of commit operation, establish project root (works from any subdirectory)
PROJECT_ROOT="$(git rev-parse --show-toplevel)"

# Lock operations - ALWAYS use absolute path
[ -f "$PROJECT_ROOT/.commit-lock" ]                    # Check
cat "$PROJECT_ROOT/.commit-lock"                        # Read
echo "stream $(date -u +%Y-%m-%dT%H:%M:%S)" > "$PROJECT_ROOT/.commit-lock"  # Create
rm -f "$PROJECT_ROOT/.commit-lock"                      # Release

# Git operations - ALWAYS from project root
git -C "$PROJECT_ROOT" status
git -C "$PROJECT_ROOT" add <files>
git -C "$PROJECT_ROOT" commit -m "message"
```

**Why this matters:** If an agent creates `.commit-lock` in `src-tauri/` instead of the project root, other agents won't see it, defeating the entire protocol.

## Protocol

### Before Committing

```
0. Set PROJECT_ROOT="$(git rev-parse --show-toplevel)" (use for ALL operations below)

1. Check if $PROJECT_ROOT/.commit-lock exists
   → NOT EXISTS: Create lock and proceed (step 4)
   → EXISTS: Read and save the lock content (who + when)

2. Wait and retry loop:
   a. Save current lock content (stream + timestamp)
   b. Log: "Commit locked by <stream>. Waiting 3s..."
   c. Run: sleep 3
   d. Re-read $PROJECT_ROOT/.commit-lock:
      → NOT EXISTS: Lock released! Create your lock, proceed (step 3)
      → DIFFERENT content: Lock changed hands (new stream). Go to 2a with new content.
      → SAME content + timestamp > 30s: STALE (crashed). Delete, create your lock, proceed.
      → SAME content + timestamp < 30s: Still active. Go to 2b.

**IMPORTANT:** "Stale" only applies to the SAME lock sitting too long. If content changed,
it's a fresh lock from another stream - NOT stale.

3. Create lock: echo "<your-stream> $(date -u +%Y-%m-%dT%H:%M:%S)" > "$PROJECT_ROOT/.commit-lock"
   → Proceed to commit (use git -C "$PROJECT_ROOT" for all git commands)
```

### After Committing (success or failure)

```
1. Remove lock file: rm -f "$PROJECT_ROOT/.commit-lock"
2. Proceed with normal workflow (or STOP)
```

### Stale Lock Detection

A lock is stale ONLY if:
1. The content (stream + timestamp) is **the same** as when you first saw it, AND
2. The timestamp is older than **30 seconds**

If the content changed, it's a **new lock** from a different stream - NOT stale.

A commit should take ~10-15 seconds max. 30 seconds gives generous buffer for slow operations.

## Implementation Example

### Acquire Lock with Retry (Bash)
```bash
acquire_lock() {
  local stream_name="$1"
  local project_root="$(git rev-parse --show-toplevel)"  # ALWAYS use absolute path
  local lock_file="$project_root/.commit-lock"
  local max_wait=30  # 30 seconds max (commits are fast)
  local waited=0

  while [ -f "$lock_file" ]; do
    local lock_content=$(cat "$lock_file" 2>/dev/null)
    local lock_time=$(echo "$lock_content" | cut -d' ' -f2)

    # Check if stale (> 30 seconds)
    # ... stale check logic ...

    echo "Commit locked by $lock_content. Waiting 3s..."
    sleep 3
    waited=$((waited + 3))

    if [ $waited -ge $max_wait ]; then
      echo "Lock wait timeout, removing stale lock"
      rm -f "$lock_file"
      break
    fi
  done

  echo "$stream_name $(date -u +%Y-%m-%dT%H:%M:%S)" > "$lock_file"
}
```

### Release Lock (Bash)
```bash
PROJECT_ROOT="$(git rev-parse --show-toplevel)"
rm -f "$PROJECT_ROOT/.commit-lock"
```

### Git Commands (from any directory)
```bash
PROJECT_ROOT="$(git rev-parse --show-toplevel)"
git -C "$PROJECT_ROOT" add src/file.tsx src-tauri/src/file.rs
git -C "$PROJECT_ROOT" commit -m "feat: description"
```

## Rules

1. **ALWAYS use absolute paths to project root** for lock file and git operations
2. **ALWAYS acquire lock before `git add`**
3. **ALWAYS release lock after commit completes (success or failure)**
4. **NEVER force-delete another stream's active lock (unless stale)**
5. **Stale = SAME lock content + >30 sec old** (not just any old timestamp)
6. **Always read lock content, not just existence** — lock may change hands while waiting

## Error Handling

If commit fails while holding lock:
1. Release the lock anyway
2. Log the failure
3. Do not retry immediately (let other streams proceed)
