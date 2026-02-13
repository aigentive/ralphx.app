---
paths:
  - "src/components/Chat/**"
  - "src/hooks/useChat*"
---

# Chat Handler Patterns

> **Maintainer note:** This file optimizes for LLM context efficiency. Rules: (1) Tables > prose (2) One example max per concept (3) No redundant explanations (4) Use symbols: → = leads to, | = or, ❌/✅ = wrong/right (5) Before adding content, ask: "Can this be a single line?" If yes, make it one line.

## Summary

- Special states (questionMode) must bypass message queue and send immediately
- Conditional order critical: special states → transient states → normal flow
- Incorrect order causes agent deadlock waiting for queued answer

## Memory References

- `96aa4a01-02a4-418c-8732-5ffc36779a24` (implementation_discoveries)

## Conditional Priority in Input Handlers

**Rule:** In input handlers where multiple paths exist (agent running, special modes, normal flow), check **special states first**, then **transient states**, then **normal flow**.

**Why:** Special states (like `questionMode`) represent request-response pairs that bypass normal message flow. If queued, they deadlock the agent.

### ❌ Wrong Order (Deadlock)
```typescript
if (isAgentRunning && onQueue) {
  // Agent waiting for answer — but answer goes to queue instead!
  onQueue(trimmedValue);
} else {
  await onSend(trimmedValue);
}
```

### ✅ Correct Order
```typescript
if (questionMode) {
  // Special state: must send immediately, bypass queue
  await onSend(trimmedValue);
} else if (isAgentRunning && onQueue) {
  // Normal message → queue for buffering
  onQueue(trimmedValue);
} else {
  // No agent → send normally
  await onSend(trimmedValue);
}
```

**Impacted:** `src/components/Chat/ChatInput.tsx:handleSend` (fixed 2026-02-13)
