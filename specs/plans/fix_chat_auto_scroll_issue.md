# Plan: Fix Chat Auto-Scroll Issue

## Problem

When loading a conversation in the ideation chat (or any IntegratedChatPanel context):
1. Chat doesn't auto-scroll to bottom when a session is selected
2. Chat doesn't scroll when switching between conversations
3. Chat doesn't scroll when new messages arrive (user sends, backend responds)

## Root Cause

In `src/hooks/useIntegratedChatScroll.ts` (line 30):

```typescript
useEffect(() => {
  if (messagesEndRef.current && messagesData.length) {
    messagesEndRef.current.scrollIntoView({ behavior: "smooth" });
  }
}, [messagesData.length]);  // ❌ Only depends on LENGTH
```

The scroll effect only triggers when `messagesData.length` changes. This fails when:
- Two conversations have the same number of messages (length unchanged)
- The conversation ID changes but message count is similar
- React Query data updates in the same render cycle

## Solution

Add `activeConversationId` to the scroll hook to trigger scroll when:
1. Message count changes (existing behavior)
2. Conversation changes (new)
3. Messages array reference changes (new - catches content updates)

## Files to Modify

1. **`src/hooks/useIntegratedChatScroll.ts`**
   - Add `activeConversationId?: string | null` to props interface
   - Add new effect that scrolls when conversation changes
   - Keep existing message length effect as backup

2. **`src/components/Chat/IntegratedChatPanel.tsx`**
   - Pass `activeConversationId` to `useIntegratedChatScroll`

## Implementation Tasks

### Task 1: Update `useIntegratedChatScroll.ts` (BLOCKING)
**Dependencies:** None
**Atomic Commit:** `fix(chat): add conversation ID to scroll hook dependencies`

Add `activeConversationId` parameter and update effect dependencies.

```typescript
interface UseIntegratedChatScrollProps {
  messagesData: unknown[];
  isAgentRunning: boolean;
  streamingToolCallsLength: number;
  activeConversationId?: string | null;  // NEW
}

export function useIntegratedChatScroll({
  messagesData,
  isAgentRunning,
  streamingToolCallsLength,
  activeConversationId,  // NEW
}: UseIntegratedChatScrollProps) {
  const messagesEndRef = useRef<HTMLDivElement>(null);
  const scrollRAFRef = useRef<number | null>(null);

  // Scroll when conversation changes OR messages change
  useEffect(() => {
    if (messagesEndRef.current && messagesData.length > 0) {
      // Use setTimeout to ensure DOM has updated
      setTimeout(() => {
        messagesEndRef.current?.scrollIntoView({ behavior: "smooth" });
      }, 0);
    }
  }, [activeConversationId, messagesData.length, messagesData]);  // Added conversationId and messagesData reference

  // ... rest unchanged
}
```

### Task 2: Update `IntegratedChatPanel.tsx`
**Dependencies:** Task 1
**Atomic Commit:** `fix(chat): pass conversation ID to scroll hook`

Pass `activeConversationId` to `useIntegratedChatScroll`.

```typescript
const { messagesEndRef } = useIntegratedChatScroll({
  messagesData,
  isAgentRunning,
  streamingToolCallsLength: streamingToolCalls.length,
  activeConversationId,  // NEW
});
```

## Verification

1. **Session Selection Test:**
   - Open ideation page
   - Select a session with existing messages
   - ✓ Chat should scroll to show last message

2. **Conversation Switch Test:**
   - Have two sessions with conversations
   - Switch between them
   - ✓ Each time should scroll to bottom

3. **New Message Test:**
   - Send a message
   - ✓ Chat scrolls to show your message
   - Wait for backend response
   - ✓ Chat scrolls to show response

4. **Tool Call Test:**
   - Trigger an agent that uses tools
   - ✓ Chat scrolls as tool calls stream in

## Commit Lock Workflow (Parallel Agent Coordination)

Reference: `.claude/rules/commit-lock.md`

### Before Committing
```bash
# 1. Establish project root (works from any subdirectory)
PROJECT_ROOT="$(git rev-parse --show-toplevel)"

# 2. Check/acquire lock
if [ -f "$PROJECT_ROOT/.commit-lock" ]; then
  # Read lock content, wait 3s, retry up to 30s
  # If stale (same content >30s), delete and proceed
fi

# 3. Create lock
echo "<stream-name> $(date -u +%Y-%m-%dT%H:%M:%S)" > "$PROJECT_ROOT/.commit-lock"

# 4. Stage and commit
git -C "$PROJECT_ROOT" add <files>
git -C "$PROJECT_ROOT" commit -m "message"
```

### After Committing
```bash
# ALWAYS release lock (success or failure)
rm -f "$PROJECT_ROOT/.commit-lock"
```

### Lock Rules
1. Acquire lock BEFORE `git add`
2. Release lock AFTER commit (success OR failure)
3. Stale = same content + >30 sec old
4. Never force-delete active lock from another agent
