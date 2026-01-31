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

## CRITICAL: Atomic Lock Acquisition

**The check-acquire-commit-release sequence MUST be a SINGLE Bash tool call.**

```
❌ WRONG - Race condition between tool calls:
   Tool call 1: Check if lock exists → "No lock"
   Tool call 2: Acquire lock and commit
   ↑ Another agent could create a lock between these two calls!

✅ CORRECT - Single atomic command:
   Tool call 1: Check + wait + acquire + commit + release (all in one)
```

**Why:** Between separate tool calls, another agent can create a lock. You would then overwrite it, causing concurrent commits and potential conflicts.

## CRITICAL: Project Root Requirement

**ALL lock operations and git commands MUST use absolute paths to the project root.**

Agents may be working in subdirectories. To ensure the lock protocol works:

1. **Determine project root** at the start of any commit operation using `git rev-parse --show-toplevel`
2. **Use absolute paths** for all lock file operations
3. **Run git commands from project root** or use `-C` flag

**Why this matters:** If an agent creates `.commit-lock` in `src-tauri/` instead of the project root, other agents won't see it, defeating the entire protocol.

## Protocol

### Complete Commit Operation (SINGLE COMMAND)

**All of this MUST be in ONE Bash tool call:**

```bash
PROJECT_ROOT="$(git rev-parse --show-toplevel)" && \
LOCK_FILE="$PROJECT_ROOT/.commit-lock" && \
STREAM_NAME="<your-stream>" && \
MAX_WAIT=30 && \
WAITED=0 && \
while [ -f "$LOCK_FILE" ]; do
  LOCK_CONTENT=$(cat "$LOCK_FILE" 2>/dev/null)
  echo "Commit locked by $LOCK_CONTENT. Waiting 3s..."
  sleep 3
  WAITED=$((WAITED + 3))
  if [ $WAITED -ge $MAX_WAIT ]; then
    echo "Lock wait timeout, removing stale lock"
    rm -f "$LOCK_FILE"
    break
  fi
done && \
echo "$STREAM_NAME $(date -u +%Y-%m-%dT%H:%M:%S)" > "$LOCK_FILE" && \
git -C "$PROJECT_ROOT" add <files> && \
git -C "$PROJECT_ROOT" commit -m "<message>" && \
rm -f "$LOCK_FILE"
```

**If commit fails, still release lock:**
```bash
# Use subshell or trap to ensure cleanup
(
  # ... acquire lock ...
  git -C "$PROJECT_ROOT" add <files> && \
  git -C "$PROJECT_ROOT" commit -m "<message>"
  RESULT=$?
  rm -f "$LOCK_FILE"  # ALWAYS release
  exit $RESULT
)
```

### Stale Lock Detection

A lock is stale ONLY if:
1. The content (stream + timestamp) is **the same** as when you first saw it, AND
2. The timestamp is older than **30 seconds**

If the content changed, it's a **new lock** from a different stream - NOT stale.

A commit should take ~10-15 seconds max. 30 seconds gives generous buffer for slow operations.

## Complete Example (Copy-Paste Template)

Replace `<stream-name>`, `<files>`, and `<message>`:

```bash
PROJECT_ROOT="$(git rev-parse --show-toplevel)" && \
LOCK_FILE="$PROJECT_ROOT/.commit-lock" && \
STREAM_NAME="<stream-name>" && \
MAX_WAIT=30 && \
WAITED=0 && \
while [ -f "$LOCK_FILE" ]; do
  LOCK_CONTENT=$(cat "$LOCK_FILE" 2>/dev/null)
  echo "Commit locked by $LOCK_CONTENT. Waiting 3s..."
  sleep 3
  WAITED=$((WAITED + 3))
  if [ $WAITED -ge $MAX_WAIT ]; then
    echo "Lock wait timeout, removing stale lock"
    rm -f "$LOCK_FILE"
    break
  fi
done && \
echo "$STREAM_NAME $(date -u +%Y-%m-%dT%H:%M:%S)" > "$LOCK_FILE" && \
git -C "$PROJECT_ROOT" add <files> && \
git -C "$PROJECT_ROOT" commit -m "$(cat <<'EOF'
<message>

Co-Authored-By: Claude Opus 4.5 <noreply@anthropic.com>
EOF
)" && \
rm -f "$LOCK_FILE"
```

## Rules

1. **SINGLE TOOL CALL** — check + acquire + commit + release in ONE Bash command
2. **ALWAYS use absolute paths to project root** for lock file and git operations
3. **ALWAYS acquire lock before `git add`**
4. **ALWAYS release lock after commit completes (success or failure)**
5. **NEVER force-delete another stream's active lock (unless stale)**
6. **Stale = SAME lock content + >30 sec old** (not just any old timestamp)
7. **Always read lock content, not just existence** — lock may change hands while waiting

## Error Handling

If commit fails while holding lock:
1. Release the lock anyway (the command template handles this)
2. Log the failure
3. Do not retry immediately (let other streams proceed)
