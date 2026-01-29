# IDA Session Auto-Naming Feature

## Overview
Add auto-generated session titles for IDA conversations using a lightweight Haiku agent, with real-time streaming updates and manual rename capability.

## Components

### 1. Backend (src-tauri/)

**New Tauri Command** (`src-tauri/src/commands/ideation_commands/ideation_commands_session.rs`):
```rust
#[tauri::command]
pub async fn update_ideation_session_title(
    id: String,
    title: Option<String>,
    state: State<'_, AppState>,
    app: tauri::AppHandle,
) -> Result<IdeationSessionResponse, String>
```
- Calls existing `session_repo.update_title()`
- Emits `ideation:session_title_updated` event for real-time UI update

**New HTTP Endpoint** (`src-tauri/src/http_server/`):
- Route: `POST /api/update_session_title`
- Handler in `handlers/ideation.rs`
- Request: `{ session_id: string, title: string }`

### 2. MCP Server (ralphx-plugin/ralphx-mcp-server/)

**New Tool** (`src/tools.ts`):
```typescript
{
  name: "update_session_title",
  description: "Update the title of an ideation session",
  inputSchema: {
    type: "object",
    properties: {
      session_id: { type: "string" },
      title: { type: "string" }
    },
    required: ["session_id", "title"]
  }
}
```

**Agent Allowlist**: Add `session-namer` agent with access to `update_session_title`

### 3. Plugin Agent (ralphx-plugin/agents/)

**New Agent** (`agents/session-namer.md`):
```yaml
---
name: session-namer
description: Generates concise titles for ideation sessions based on user's first message
model: haiku
---
```

System prompt:
- Given user's first message, generate a concise (3-6 word) title
- Call `update_session_title` with the generated title
- No other tools needed

### 4. Frontend (src/)

**API Wrapper** (`src/api/ideation.ts`):
```typescript
sessions: {
  ...existing,
  updateTitle: async (sessionId: string, title: string | null): Promise<void>
}
```

**Event Listener** (`src/hooks/useIdeationEvents.ts`):
- Listen for `ideation:session_title_updated`
- Update Zustand store's session record

**UI Changes** (`src/components/Ideation/SessionBrowser.tsx`):
- Add three-dot menu (DropdownMenu) on session items
- Options: "Rename", "Archive", "Delete"
- Inline edit mode for rename

### 5. Triggering Mechanism (Tauri Command)

**New Tauri Command** (`src-tauri/src/commands/ideation_commands/`):
```rust
#[tauri::command]
pub async fn spawn_session_namer(
    session_id: String,
    first_message: String,
    state: State<'_, AppState>,
) -> Result<(), String>
```
- Uses `AgenticClientSpawner` to spawn `session-namer` agent
- Passes session_id and first_message as context
- Runs in background, returns immediately

**Frontend Call** (`src/hooks/useIdaChat.ts` or chat send logic):
- After sending the first message to a session (message count goes 0→1)
- Call `invoke("spawn_session_namer", { sessionId, firstMessage })`
- Agent runs in background, calls MCP tool, title streams to UI via event

## File Changes

| Layer | File | Change |
|-------|------|--------|
| Backend | `src-tauri/src/commands/ideation_commands/ideation_commands_session.rs` | Add `update_ideation_session_title` command |
| Backend | `src-tauri/src/commands/ideation_commands/ideation_commands_session.rs` | Add `spawn_session_namer` command |
| Backend | `src-tauri/src/http_server/handlers/ideation.rs` | Add `update_session_title` handler |
| Backend | `src-tauri/src/http_server/mod.rs` | Add route |
| Backend | `src-tauri/lib.rs` | Register new commands |
| MCP | `ralphx-mcp-server/src/tools.ts` | Add tool definition + allowlist |
| MCP | `ralphx-mcp-server/src/index.ts` | Add routing for new tool |
| Plugin | `ralphx-plugin/agents/session-namer.md` | New agent file |
| Frontend | `src/api/ideation.ts` | Add `updateTitle` and `spawnSessionNamer` wrappers |
| Frontend | `src/hooks/useIdeationEvents.ts` | Add title update listener |
| Frontend | `src/stores/ideationStore.ts` | Verify `updateSession` action exists (it does) |
| Frontend | `src/components/Ideation/SessionBrowser.tsx` | Add three-dot menu with rename |
| Frontend | `src/hooks/useIdaChat.ts` (or equivalent) | Call `spawn_session_namer` on first message |

## Event Flow

```
1. User sends first message in IDA session
   ↓
2. Frontend sends message via existing chat mechanism
   ↓
3. Frontend detects first message, calls Tauri: spawn_session_namer(session_id, message)
   ↓
4. Tauri spawns session-namer agent via AgenticClientSpawner (returns immediately)
   ↓
5. Agent (Haiku) receives prompt with session_id and first message
   ↓
6. Agent generates title, calls MCP tool: update_session_title(session_id, title)
   ↓
7. MCP server → HTTP POST /api/update_session_title → Tauri
   ↓
8. Tauri HTTP handler updates DB via session_repo.update_title()
   ↓
9. Tauri emits event: ideation:session_title_updated { session_id, title }
   ↓
10. Frontend listener receives event, calls ideationStore.updateSession()
    ↓
11. SessionBrowser re-renders with new title (real-time)
```

## Verification

1. **Auto-naming test**: Create new IDA session, send first message like "I want to build a task management app", verify title appears (e.g., "Task Management App")
2. **Manual rename test**: Click three-dot menu → Rename, edit title, verify persists after refresh
3. **Event test**: Open DevTools Network tab, watch for `ideation:session_title_updated` event
4. **Agent CLI test**: `claude --plugin-dir ./ralphx-plugin --agent session-namer -p "Session ID: test-123. User message: How do I implement authentication?"` - should call MCP tool
5. **Backend test**: `cargo test update_ideation_session_title` for new command
