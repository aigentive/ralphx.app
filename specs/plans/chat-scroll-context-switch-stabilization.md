## Stabilize Task-Context Chat Scrolling and Remove Nested Scroll Jank

### Summary
We will make chat scroll behavior deterministic across task state/context switches and eliminate bottom "wiggle" caused by nested auto-scrolling cards competing with the outer chat list.  
Chosen defaults:
1. Context switch policy: **Pin to latest** for the new context (except explicit history mode).
2. Nested card policy: **Contain wheel events** inside card scroll areas.

### What We'll Change
1. **Fix context-switch sequencing so chat does not bounce through an empty remount**
- File: `src/hooks/useChatPanelContext.ts`
- Remove the separate "track previous context type/id" effect that overwrites old context too early.
- In the context-change effect, capture old context from refs before updating them, then clean old queries using true previous values.
- Avoid forcing a transient `activeConversationId = null` when switching context if a replacement conversation exists.
- Update stale-conversation handling in `autoSelectConversation`:
  - If current active conversation is not in new context and new context has conversations, directly select the most recent one in the same pass.
  - Only set `null` when new context has no conversations.
- Result: prevents `ChatMessageList` key cycling `old -> empty -> new`, which is a major source of jumpiness.

2. **Make Virtuoso the single source of outer chat scrolling**
- Files: `src/hooks/useChatAutoScroll.ts`, `src/components/Chat/ChatMessageList.tsx`, `src/components/Chat/ChatMessages.tsx`
- Stop using `messagesEndRef.scrollIntoView` for Virtuoso-based lists during normal streaming/message updates.
- Keep `followOutput` + `atBottomStateChange` as the only outer-list auto-scroll control path.
- Keep `scrollToBottom` button behavior, but route it through Virtuoso (`scrollToIndex` last item) instead of marker element scrolling.
- Preserve history mode behavior (`scrollToTimestamp` disables live auto-follow).

3. **Eliminate nested scroll chaining and inner/outer competition**
- Files: `src/components/Chat/TaskSubagentCard.tsx`, `src/components/Chat/StreamingToolIndicator.tsx`
- Add `overscroll-behavior: contain` on inner scrollable card content.
- Add "stick-to-bottom" guard for inner card autoscroll:
  - Auto-scroll inner card only if user was already near card bottom.
  - If user manually scrolls up inside card, pause inner autoscroll until they return near bottom.
- Keep outer chat follow behavior intact while preventing card-edge wheel propagation from nudging parent scroll.

4. **Smoothness tuning for streaming**
- File: `src/hooks/useChatAutoScroll.ts`
- Keep bottom threshold logic, but avoid dual smooth animations from multiple mechanisms.
- Ensure streaming updates do not trigger redundant scroll commands in both hook effects and Virtuoso callbacks.

### Public APIs / Interfaces / Types
1. **No backend/API contract changes.**
2. Internal hook contract refinement:
- `useChatAutoScroll` may gain a mode flag (for example `listMode: "virtuoso" | "dom"`) or equivalent behavior split to disable DOM marker scrolling for Virtuoso consumers.
- `ChatMessageList`/`ChatMessages` will use Virtuoso-native imperative scroll for manual "scroll to bottom".

### Test Cases and Scenarios
1. **Context switch without jump**
- Start in `task_execution` conversation, switch task status to `reviewing`.
- Verify no intermediate empty-key remount and viewport lands at newest content in review conversation.

2. **No "last message then jump up" regression**
- Load long conversation in split task view.
- Trigger state transition that switches context.
- Assert final viewport is bottom-stable and remains stable after first render tick.

3. **Nested scroll isolation**
- With active `TaskSubagentCard`/`StreamingToolIndicator`, scroll inside inner list to top/bottom edges.
- Verify outer chat does not wiggle or move from wheel-chain propagation.

4. **Inner autoscroll respect**
- Manually scroll up inside an inner card while new tool calls stream in.
- Verify inner card does not force-scroll to bottom until user returns near bottom.

5. **History mode safety**
- Enter timeline history state and verify:
  - `scrollToTimestamp` works.
  - Live follow remains disabled.
  - No auto-jump back to latest until exiting history mode.

6. **Regression checks**
- `useChatAutoScroll` unit tests updated for Virtuoso mode behavior.
- `ChatMessageList` tests updated to verify single-path scroll behavior and conversation-switch stability.
- `useChatPanelContext` tests for old-context cleanup correctness and direct stale->new conversation handoff.

### Assumptions and Defaults
1. "Perfect" behavior means: **latest message pinned on context switches** in live mode.
2. In history mode, timestamp anchoring remains authoritative (no live follow).
3. Inner streaming cards remain scrollable, but they are isolated from parent scroll (`overscroll-behavior: contain`).
4. No product change to conversation selection strategy beyond removing transient empty state and stabilizing auto-selection handoff.
