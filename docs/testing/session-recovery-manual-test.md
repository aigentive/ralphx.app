# Session Recovery Manual Testing Guide

## Purpose
Verify that the automatic session recovery feature works correctly when a Claude session becomes stale.

## Prerequisites
1. RalphX application running with `ENABLE_SESSION_RECOVERY=true` in `.env`
2. An active conversation in RalphX (ideation chat or task chat)
3. Access to terminal for deleting session files

## Test Procedure

### Step 1: Identify Active Session
1. Open RalphX application
2. Navigate to a conversation (ideation chat or task chat)
3. Send a message to establish a session
4. Check the database to find the session ID:
   ```bash
   sqlite3 src-tauri/ralphx.db "SELECT id, claude_session_id FROM chat_conversations WHERE claude_session_id IS NOT NULL ORDER BY created_at DESC LIMIT 1;"
   ```
5. Note the `claude_session_id` value (e.g., `abc123def456`)

### Step 2: Simulate Stale Session
1. Close RalphX (or keep it open - either works)
2. Delete the Claude session directory:
   ```bash
   rm -rf ~/.claude/projects/<session-id>
   ```
   Replace `<session-id>` with the actual session ID from Step 1.
3. Verify the directory is deleted:
   ```bash
   ls ~/.claude/projects/<session-id>
   ```
   Should output: `No such file or directory`

### Step 3: Trigger Recovery
1. Open RalphX (if closed)
2. Navigate to the same conversation
3. Send a new message
4. **Expected behavior:**
   - Message is sent successfully
   - No error message visible in UI
   - A banner may appear: "Session restored from local history"
   - Response from Claude appears normally

### Step 4: Verify Recovery in Logs
1. Check Tauri logs for recovery events:
   ```bash
   # On macOS, Tauri logs are typically in Console.app or:
   # Check the terminal where you ran `npm run tauri dev`
   ```
2. Look for these log events:
   - `event=stale_session_detected`
   - `event=rehydrate_success`
3. **Expected log output:**
   ```
   WARN ralphx: event=stale_session_detected session_id=<old-session-id> conversation_id=<conv-id> context_type=IdeationChat
   DEBUG ralphx: Built conversation replay for rehydration turns=N estimated_tokens=X truncated=false conversation_id=<conv-id>
   INFO ralphx: event=rehydrate_success conversation_id=<conv-id> new_session_id=<new-session-id> replay_turns=N estimated_tokens=X duration_ms=Y
   ```

### Step 5: Verify Session ID Updated
1. Check that the conversation has a new session ID:
   ```bash
   sqlite3 src-tauri/ralphx.db "SELECT id, claude_session_id FROM chat_conversations WHERE id = '<conversation-id>';"
   ```
2. The `claude_session_id` should be different from the original one
3. Verify the new session directory exists:
   ```bash
   ls ~/.claude/projects/<new-session-id>
   ```

### Step 6: Verify Conversation Continuity
1. Send a follow-up message in the conversation
2. The message should reference previous context
3. Claude should respond as if the session never expired

## Success Criteria
✅ No error message shown to user
✅ Conversation continues without interruption
✅ Log shows `event=rehydrate_success`
✅ Database shows new session ID
✅ New session directory created in `~/.claude/projects/`
✅ Follow-up messages maintain context

## Failure Scenarios to Test

### Test Case 2: Recovery Failure (No History)
1. Create a brand new conversation with no messages
2. Manually set a fake `claude_session_id` in the database
3. Try to send a message
4. **Expected:** Error is shown (cannot recover with no history)

### Test Case 3: Retry Prevention
1. Mock the recovery to fail (requires code change)
2. Trigger stale session
3. **Expected:** Only one recovery attempt, then error shown

### Test Case 4: Tool Calls in History
1. Have a conversation with tool calls (e.g., ask Claude to read a file)
2. Delete session directory
3. Send new message
4. **Expected:** Recovery includes tool call history, new message works

## Troubleshooting

### No logs appearing
- Make sure you're running in dev mode: `npm run tauri dev`
- Check Console.app on macOS for application logs
- Verify `RUST_LOG=debug` or `RUST_LOG=info` is set

### Recovery not triggering
- Verify `.env` file exists with `ENABLE_SESSION_RECOVERY=true`
- Restart the application after creating `.env`
- Check that the feature flag is being read (add debug log)

### Error still shown to user
- Check logs for `event=rehydrate_failure`
- Verify the conversation has message history in the database
- Ensure the original session ID was actually stored

## Notes
- This test can be run multiple times on the same conversation
- Each successful recovery will create a new session ID
- The old session directory will remain deleted (no cleanup needed)
- Recovery should take < 5 seconds for most conversations
