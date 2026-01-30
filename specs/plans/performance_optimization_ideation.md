# Performance Optimization Plan: Ideation Section

## Problem Statement

When navigating through many sessions with large conversations, the ideation section becomes laggy. Root causes:
1. No virtualization for message lists
2. Missing memoization causing cascading re-renders
3. Memory leaks from uncleaned Zustand stores
4. Dual storage pattern (TanStack Query + Zustand)

---

## Phase 1: Add Virtualization to Message Lists (High Impact) (BLOCKING)

**Dependencies:** None
**Atomic Commit:** `feat(chat): add virtualization with react-virtuoso`

**Why First:** This provides the biggest immediate performance gain for large conversations.

### Files to Modify:
- `src/components/Chat/IntegratedChatPanel.tsx` (line ~498-507)
- `src/components/Chat/ChatMessages.tsx` (line ~224)

### Implementation:
1. Install `react-virtuoso` (preferred over react-window for dynamic heights)
   ```bash
   npm install react-virtuoso
   ```

2. Replace message `.map()` with `<Virtuoso>`:
   ```tsx
   import { Virtuoso } from 'react-virtuoso';

   <Virtuoso
     data={sortedMessages}
     itemContent={(index, msg) => (
       <MessageItem
         key={msg.id}
         role={msg.role}
         content={msg.content}
         createdAt={msg.createdAt}
         toolCalls={msg.toolCalls}
         contentBlocks={msg.contentBlocks}
       />
     )}
     followOutput="smooth"
     alignToBottom
   />
   ```

3. Update `useIntegratedChatScroll.ts` to use Virtuoso's scroll API

---

## Phase 2: Memoize Components (High Impact)

**Dependencies:** None (can run in parallel with Phase 1)
**Atomic Commit:** `feat(chat): memoize message and proposal components`

**Why:** Prevents cascading re-renders when parent state changes.

### Files to Modify:
- `src/components/Chat/MessageItem.tsx`
- `src/components/Chat/ToolCallIndicator.tsx`
- `src/components/Ideation/ProposalCard.tsx`
- `src/components/Ideation/ProposalList.tsx`

### Implementation:

1. **MessageItem** - Wrap with React.memo + custom equality:
   ```tsx
   export const MessageItem = React.memo(function MessageItem({ ... }) {
     // existing implementation
   }, (prev, next) => {
     return prev.id === next.id
       && prev.content === next.content
       && prev.toolCalls === next.toolCalls;
   });
   ```

2. **ToolCallIndicator** - Wrap with React.memo:
   ```tsx
   export const ToolCallIndicator = React.memo(function ToolCallIndicator({ ... }) {
     // existing implementation
   });
   ```

3. **ProposalCard** - Wrap with React.memo:
   ```tsx
   export const ProposalCard = React.memo(function ProposalCard({ ... }) {
     // existing implementation
   });
   ```

4. **ProposalList** - Add useCallback for handlers:
   ```tsx
   const handleSelect = useCallback((id: string) => {
     onSelectProposal?.(id);
   }, [onSelectProposal]);

   const handleCardSelect = useCallback((id: string, selected: boolean) => {
     onUpdateProposal?.(id, { selected });
   }, [onUpdateProposal]);
   ```

---

## Phase 3: Fix Memory Leaks (Critical for Long Sessions)

**Dependencies:** None (can run in parallel with Phase 1-2)
**Atomic Commit:** `fix(stores): add cleanup and LRU eviction for memory management`

**Why:** Prevents memory growth during extended use.

### Files to Modify:
- `src/stores/chatStore.ts`
- `src/stores/ideationStore.ts`
- `src/components/Chat/IntegratedChatPanel.tsx`
- `src/App.tsx`

### Implementation:

1. **Add context cleanup on switch** (IntegratedChatPanel.tsx):
   ```tsx
   useEffect(() => {
     if (prevContextKeyRef.current !== contextKey && prevContextKeyRef.current) {
       // Clear old context messages from store
       clearMessages(prevContextKeyRef.current);
     }
     prevContextKeyRef.current = contextKey;
   }, [contextKey, clearMessages]);
   ```

2. **Add session cleanup on archive/delete** (App.tsx):
   ```tsx
   const handleArchiveSession = useCallback(async (sessionId: string) => {
     await ideationApi.sessions.archive(sessionId);
     removeSession(sessionId); // Clean from Zustand store
     clearMessages(`ideation:${sessionId}`); // Clean chat messages
   }, [removeSession, clearMessages]);
   ```

3. **Add LRU eviction for sessions** (ideationStore.ts):
   ```tsx
   const MAX_CACHED_SESSIONS = 20;

   addSession: (session) => set((state) => {
     state.sessions[session.id] = session;
     // Evict oldest if over limit
     const sessionIds = Object.keys(state.sessions);
     if (sessionIds.length > MAX_CACHED_SESSIONS) {
       const oldest = sessionIds
         .filter(id => id !== state.activeSessionId)
         .sort((a, b) =>
           new Date(state.sessions[a].updatedAt).getTime() -
           new Date(state.sessions[b].updatedAt).getTime()
         )[0];
       if (oldest) delete state.sessions[oldest];
     }
   }),
   ```

---

## Phase 4: Optimize JSON Parsing (Medium Impact)

**Dependencies:** Phase 2 (memoization should be in place first)
**Atomic Commit:** `refactor(chat): move JSON parsing to API layer`

**Why:** Prevents redundant parsing on every render.

### Files to Modify:
- `src/components/Chat/MessageItem.tsx` (lines 295-323)
- `src/api/chat.ts` (parse at fetch time)

### Implementation:

1. **Move parsing to API layer** (chat.ts):
   ```tsx
   async getConversation(id: string): Promise<Conversation> {
     const result = await invoke<ConversationResponse>('get_conversation', { id });
     return {
       ...result,
       messages: result.messages.map(msg => ({
         ...msg,
         contentBlocks: parseContentBlocks(msg.contentBlocks),
         toolCalls: parseToolCalls(msg.toolCalls),
       })),
     };
   }
   ```

2. **Remove redundant useMemo in MessageItem** - data arrives pre-parsed

---

## Phase 5: Fix Selector Stability (Medium Impact)

**Dependencies:** Phase 3 (store changes should be in place first)
**Atomic Commit:** `fix(stores): stabilize selectors with memoized empty arrays`

**Why:** Prevents unnecessary re-renders from reference inequality.

### Files to Modify:
- `src/stores/chatStore.ts` (lines 290-323)

### Implementation:

1. **Use shallow equality for array selectors**:
   ```tsx
   import { shallow } from 'zustand/shallow';

   // In component:
   const messages = useChatStore(
     (state) => state.messages[contextKey] ?? [],
     shallow
   );
   ```

2. **Or memoize empty array**:
   ```tsx
   const EMPTY_ARRAY: ChatMessage[] = [];

   export const selectMessagesForContext =
     (contextKey: string) =>
     (state: ChatState): ChatMessage[] =>
       state.messages[contextKey] ?? EMPTY_ARRAY;
   ```

---

## Phase 6: Memoize Markdown Components (Low Impact)

**Dependencies:** Phase 2 (should happen after component memoization)
**Atomic Commit:** `refactor(chat): extract markdown components to module constant`

**Why:** Prevents object recreation on every render.

### Files to Modify:
- `src/components/Chat/MessageItem.tsx`

### Implementation:

1. **Extract to module-level constant**:
   ```tsx
   // Top of file, outside component
   const MARKDOWN_COMPONENTS: Components = {
     code({ children, className }) { ... },
     pre({ children }) { ... },
     // ... rest of config
   };

   // In component - just reference the constant
   <ReactMarkdown components={MARKDOWN_COMPONENTS}>
     {content}
   </ReactMarkdown>
   ```

---

## Phase 7: Refactor Oversized Files

**Dependencies:** Phases 1-6 (extract after core changes are complete)
**Atomic Commit:** `refactor(chat): extract MessageItem and IntegratedChatPanel components`

**Why:** Files exceed LOC limits per code-quality-standards.md (500 for components, 300 for hooks).

### Files to Extract:

1. **MessageItem.tsx (416 LOC → ~200 LOC)**
   - Extract `MessageContent.tsx` - markdown rendering + code blocks
   - Extract `useMessageParsing.ts` - content/toolCall parsing logic
   - Extract `messageUtils.ts` - timestamp formatting, helpers

2. **IntegratedChatPanel.tsx (564 LOC → ~300 LOC)**
   - Extract `ChatHeader.tsx` - context indicator + conversation selector
   - Extract `ChatMessageList.tsx` - virtualized message list wrapper
   - Extract `useChatPanelState.ts` - context management, conversation selection

### Extraction Pattern:
```
src/components/Chat/
├── IntegratedChatPanel.tsx (orchestrator, ~300 LOC)
├── ChatHeader.tsx (new, ~80 LOC)
├── ChatMessageList.tsx (new, ~120 LOC)
├── MessageItem.tsx (display, ~200 LOC)
├── MessageContent.tsx (new, ~100 LOC)
└── hooks/
    ├── useChatPanelState.ts (new, ~100 LOC)
    └── useMessageParsing.ts (new, ~60 LOC)
```

---

## Implementation Order

| Phase | Impact | Effort | Priority | Dependencies |
|-------|--------|--------|----------|--------------|
| 1. Virtualization | High | Medium | P0 | None |
| 2. React.memo | High | Low | P0 | None |
| 3. Memory leaks | High | Medium | P0 | None |
| 4. JSON parsing | Medium | Low | P1 | Phase 2 |
| 5. Selector stability | Medium | Low | P1 | Phase 3 |
| 6. Markdown memoization | Low | Low | P2 | Phase 2 |
| 7. File extractions | Quality | Medium | P0 | Phases 1-6 |

**Recommended execution:** Phases 1-3 in parallel (all P0, no dependencies), then 4-6, then 7.

---

## Verification

### Manual Testing:
1. Create/select a session with 100+ messages
2. Rapidly switch between 5+ sessions
3. Monitor memory in DevTools (should stabilize, not grow)
4. Check for scroll jank in message list

### Performance Metrics:
```bash
# Open Chrome DevTools > Performance
# Record while switching sessions
# Look for:
# - Long tasks > 50ms
# - Layout thrashing
# - Excessive JS heap growth
```

### Memory Check:
```javascript
// In DevTools console after switching 10 sessions:
console.log(Object.keys(useChatStore.getState().messages).length); // Should be ~1-3
console.log(Object.keys(useIdeationStore.getState().sessions).length); // Should be ≤20
```

---

## Files Summary

**Primary modifications:**
- `src/components/Chat/IntegratedChatPanel.tsx` - virtualization + cleanup
- `src/components/Chat/MessageItem.tsx` - memo + markdown const
- `src/components/Chat/ToolCallIndicator.tsx` - memo
- `src/components/Ideation/ProposalCard.tsx` - memo
- `src/components/Ideation/ProposalList.tsx` - useCallback
- `src/stores/chatStore.ts` - selector fix + cleanup method
- `src/stores/ideationStore.ts` - LRU eviction
- `src/App.tsx` - cleanup on archive/delete

**New dependency:**
- `react-virtuoso`

---

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
