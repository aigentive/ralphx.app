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

Route: `POST /api/update_session_title`

**Request Type** (`src-tauri/src/http_server/types.rs`):
```rust
#[derive(Debug, Deserialize)]
pub struct UpdateSessionTitleRequest {
    pub session_id: String,
    pub title: String,
}
```

**Handler** (`src-tauri/src/http_server/handlers/ideation.rs`):
```rust
pub async fn update_session_title(
    State(state): State<HttpServerState>,
    Json(req): Json<UpdateSessionTitleRequest>,
) -> Result<Json<SuccessResponse>, StatusCode> {
    let session_id = IdeationSessionId::from_string(req.session_id.clone());

    // Update title in database
    state.app_state.session_repo
        .update_title(&session_id, Some(req.title.clone()))
        .await
        .map_err(|e| {
            error!("Failed to update session title: {}", e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?;

    // Emit event for real-time UI update
    if let Some(app_handle) = &state.app_state.app_handle {
        let _ = app_handle.emit("ideation:session_title_updated", serde_json::json!({
            "session_id": req.session_id,
            "title": req.title
        }));
    }

    Ok(Json(SuccessResponse {
        success: true,
        message: "Session title updated".to_string(),
    }))
}
```

**Route Registration** (`src-tauri/src/http_server/mod.rs`):
```rust
// Add after existing ideation routes
.route("/api/update_session_title", post(update_session_title))
```

### 2. MCP Server (ralphx-plugin/ralphx-mcp-server/)

**New Tool Definition** (`src/tools.ts`):
```typescript
// Add to ALL_TOOLS array (in IDEATION TOOLS section)
{
  name: "update_session_title",
  description: "Update the title of an ideation session. Used by session-namer agent to set auto-generated titles.",
  inputSchema: {
    type: "object",
    properties: {
      session_id: {
        type: "string",
        description: "The ideation session ID to update",
      },
      title: {
        type: "string",
        description: "The new title for the session (3-6 words recommended)",
      },
    },
    required: ["session_id", "title"],
  },
},
```

**Agent Allowlist** (`src/tools.ts`):
```typescript
// Add to TOOL_ALLOWLIST object
"session-namer": ["update_session_title"],
```

**Tool Routing** (`src/index.ts`):
No changes needed - the tool uses the default POST routing:
```typescript
// Default case at line ~258 handles POST requests:
result = await callTauri(name, (args as Record<string, unknown>) || {});
// This will POST to /api/update_session_title with the args as JSON body
```

### 3. Plugin Agent (ralphx-plugin/agents/)

**New Agent** (`ralphx-plugin/agents/session-namer.md`):
```markdown
---
name: session-namer
description: Generates concise titles for ideation sessions based on user's first message
model: haiku
---

You are a session title generator. Your job is to create a concise, descriptive title for an ideation session based on the user's first message.

## Instructions

1. Read the user's first message carefully
2. Generate a title that captures the main topic or intent (3-6 words)
3. Use title case (capitalize first letter of major words)
4. Avoid generic titles like "New Session" or "Untitled"
5. Call the `update_session_title` tool with the session_id and generated title

## Examples

- "I want to build a task management app" → "Task Management App"
- "How do I implement authentication?" → "Authentication Implementation"
- "Let's discuss the API design" → "API Design Discussion"
- "I need help with database schema" → "Database Schema Design"

## Context

The session_id and user's first message will be provided in the prompt.
```

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
