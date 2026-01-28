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

## Protocol

### Before Committing

```
1. Check if .commit-lock exists
   → NOT EXISTS: Create lock and proceed (step 4)
   → EXISTS: Read and save the lock content (who + when)

2. Wait and retry loop:
   a. Save current lock content (stream + timestamp)
   b. Log: "Commit locked by <stream>. Waiting 3s..."
   c. Run: sleep 3
   d. Re-read .commit-lock:
      → NOT EXISTS: Lock released! Create your lock, proceed (step 3)
      → DIFFERENT content: Lock changed hands (new stream). Go to 2a with new content.
      → SAME content + timestamp > 30s: STALE (crashed). Delete, create your lock, proceed.
      → SAME content + timestamp < 30s: Still active. Go to 2b.

**IMPORTANT:** "Stale" only applies to the SAME lock sitting too long. If content changed,
it's a fresh lock from another stream - NOT stale.

3. Create lock: echo "<your-stream> $(date -u +%Y-%m-%dT%H:%M:%S)" > .commit-lock
   → Proceed to commit
```

### After Committing (success or failure)

```
1. Remove .commit-lock file: rm -f .commit-lock
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
  local max_wait=30  # 30 seconds max (commits are fast)
  local waited=0

  while [ -f .commit-lock ]; do
    local lock_content=$(cat .commit-lock 2>/dev/null)
    local lock_time=$(echo "$lock_content" | cut -d' ' -f2)

    # Check if stale (> 30 seconds)
    # ... stale check logic ...

    echo "Commit locked by $lock_content. Waiting 3s..."
    sleep 3
    waited=$((waited + 3))

    if [ $waited -ge $max_wait ]; then
      echo "Lock wait timeout, removing stale lock"
      rm -f .commit-lock
      break
    fi
  done

  echo "$stream_name $(date -u +%Y-%m-%dT%H:%M:%S)" > .commit-lock
}
```

### Release Lock (Bash)
```bash
rm -f .commit-lock
```

## Rules

1. **ALWAYS acquire lock before `git add`**
2. **ALWAYS release lock after commit completes (success or failure)**
3. **NEVER force-delete another stream's active lock (unless stale)**
4. **Stale = SAME lock content + >30 sec old** (not just any old timestamp)
5. **Always read lock content, not just existence** — lock may change hands while waiting

## Error Handling

If commit fails while holding lock:
1. Release the lock anyway
2. Log the failure
3. Do not retry immediately (let other streams proceed)
