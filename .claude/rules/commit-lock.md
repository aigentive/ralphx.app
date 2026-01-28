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
   → EXISTS: Another stream is committing
      - Read lock file to see which stream
      - Output: "Commit locked by <stream>. Waiting 5s..."
      - Run: sleep 5
      - Check again (loop until lock is released or stale)

   → NOT EXISTS: Safe to commit
      - Create .commit-lock with your stream name and timestamp
      - Proceed to commit
```

### After Committing (success or failure)

```
1. Remove .commit-lock file
2. Proceed with normal workflow (or STOP)
```

### Stale Lock Detection

If lock file exists but timestamp is older than 5 minutes:
- Lock is considered stale (previous stream crashed)
- Safe to delete and acquire lock
- Log: "Removed stale commit lock from <stream>"

## Implementation

### Acquire Lock (Bash)
```bash
if [ -f .commit-lock ]; then
  echo "Commit locked by $(cat .commit-lock)"
  # Either wait or skip commit this iteration
else
  echo "$(STREAM_NAME) $(date -u +%Y-%m-%dT%H:%M:%S)" > .commit-lock
  # Proceed with commit
fi
```

### Release Lock (Bash)
```bash
rm -f .commit-lock
```

### Check Stale (Bash)
```bash
if [ -f .commit-lock ]; then
  lock_time=$(cut -d' ' -f2 .commit-lock)
  # If lock_time is > 5 minutes old, delete and proceed
fi
```

## Rules

1. **ALWAYS acquire lock before `git add`**
2. **ALWAYS release lock after commit completes (success or failure)**
3. **NEVER force-delete another stream's active lock**
4. **Stale locks (>5 min) can be safely removed**

## Error Handling

If commit fails while holding lock:
1. Release the lock anyway
2. Log the failure
3. Do not retry immediately (let other streams proceed)
