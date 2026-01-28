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

2. Check if lock is stale (timestamp > 2 minutes old)
   → STALE: Delete it, log "Removed stale lock from <stream>", create your lock, proceed
   → NOT STALE: Continue to step 3

3. Wait and retry loop:
   a. Log: "Commit locked by <stream>. Waiting 5s..."
   b. Run: sleep 5
   c. Check .commit-lock again:
      → NOT EXISTS: Create your lock, proceed (step 4)
      → EXISTS but DIFFERENT content: Lock changed hands, save new content, go to 3a
      → EXISTS with SAME content: Same stream still holding, go to 3a (or check stale)

4. Create lock: echo "<your-stream> $(date -u +%Y-%m-%dT%H:%M:%S)" > .commit-lock
   → Proceed to commit
```

**Key insight:** When you wake from sleep, the lock might have been released and re-acquired by a faster stream. Always read the content to know if it's the same lock or a new one.

### After Committing (success or failure)

```
1. Remove .commit-lock file: rm -f .commit-lock
2. Proceed with normal workflow (or STOP)
```

### Stale Lock Detection

If lock file timestamp is older than **2 minutes**:
- Lock is considered stale (previous stream crashed)
- Safe to delete and acquire lock
- Log: "Removed stale commit lock from <stream>"

## Implementation Example

### Acquire Lock with Retry (Bash)
```bash
acquire_lock() {
  local stream_name="$1"
  local max_wait=120  # 2 minutes max
  local waited=0

  while [ -f .commit-lock ]; do
    local lock_content=$(cat .commit-lock 2>/dev/null)
    local lock_time=$(echo "$lock_content" | cut -d' ' -f2)

    # Check if stale (> 2 minutes)
    # ... stale check logic ...

    echo "Commit locked by $lock_content. Waiting 5s..."
    sleep 5
    waited=$((waited + 5))

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
4. **Stale locks (>2 min) can be safely removed**
5. **Always read lock content, not just existence** — lock may change hands while waiting

## Error Handling

If commit fails while holding lock:
1. Release the lock anyway
2. Log the failure
3. Do not retry immediately (let other streams proceed)
