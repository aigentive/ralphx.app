# Permission Bridge System

When agents attempt to use tools that aren't pre-approved via `--allowedTools`, we need a mechanism to:
1. Pause Claude CLI execution
2. Present the permission request to the user in the UI
3. Capture the user's approve/reject decision
4. Resume Claude CLI with that decision

## Why This Is Needed

Claude CLI in `-p` mode is non-interactive. The built-in permission mechanisms are:
- `--allowedTools`: Pre-approve tools at spawn time (compile-time only)
- `--permission-prompt-tool`: Specify an MCP tool to handle permission prompts synchronously
- Hooks (`PermissionRequest`): Shell commands that run synchronously

None of these support **asynchronous UI-based approval**. We solve this by using `--permission-prompt-tool` with an MCP tool that long-polls our Tauri backend.

---

## Permission Bridge Architecture

```
┌─────────────────────────────────────────────────────────────────────────────┐
│                     PERMISSION BRIDGE FLOW                                   │
├─────────────────────────────────────────────────────────────────────────────┤
│                                                                              │
│  1. Claude CLI encounters tool needing permission                           │
│     (tool not in --allowedTools)                                            │
│           │                                                                  │
│           ▼                                                                  │
│  2. Claude CLI calls MCP tool: mcp__ralphx__permission_request              │
│     Args: { tool_name, tool_input, context }                                │
│           │                                                                  │
│           ▼                                                                  │
│  3. MCP Server receives permission_request call                             │
│     → POST to Tauri: /api/permission/request                                │
│     → Tauri stores pending request in memory                                │
│     → Tauri emits event: "permission:request"                               │
│     → MCP tool BLOCKS (long-poll /api/permission/await/:id)                 │
│           │                                                                  │
│           ▼                                                                  │
│  4. Frontend receives Tauri event                                           │
│     → Shows PermissionDialog with tool details                              │
│     → User clicks Allow / Deny                                              │
│           │                                                                  │
│           ▼                                                                  │
│  5. Frontend calls: invoke("resolve_permission_request", { id, decision })  │
│     → Tauri signals waiting long-poll request                               │
│     → MCP tool receives response, returns to Claude CLI                     │
│           │                                                                  │
│           ▼                                                                  │
│  6. Claude CLI continues or aborts based on decision                        │
│                                                                              │
└─────────────────────────────────────────────────────────────────────────────┘
```

---

## Permission Handler MCP Tool

**File:** `ralphx-mcp-server/src/permission-handler.ts`

```typescript
import { TAURI_API_URL } from "./tauri-client.js";

// Tool definition for permission handling
export const permissionRequestTool = {
  name: "permission_request",
  description: "Internal tool for handling permission prompts from Claude CLI",
  inputSchema: {
    type: "object",
    properties: {
      tool_name: {
        type: "string",
        description: "Name of the tool requesting permission"
      },
      tool_input: {
        type: "object",
        description: "Input arguments for the tool"
      },
      context: {
        type: "string",
        description: "Additional context about why the tool is being called"
      },
    },
    required: ["tool_name", "tool_input"],
  },
};

interface PermissionDecision {
  decision: "allow" | "deny";
  message?: string;
}

/**
 * Handle a permission request by forwarding to Tauri backend
 * and waiting for user decision via long-poll.
 *
 * Timeout: 5 minutes (user may be away from keyboard)
 */
export async function handlePermissionRequest(args: {
  tool_name: string;
  tool_input: Record<string, unknown>;
  context?: string;
}): Promise<{ content: Array<{ type: "text"; text: string }> }> {
  // 1. Register permission request with Tauri backend
  const registerResponse = await fetch(`${TAURI_API_URL}/api/permission/request`, {
    method: "POST",
    headers: { "Content-Type": "application/json" },
    body: JSON.stringify({
      tool_name: args.tool_name,
      tool_input: args.tool_input,
      context: args.context,
    }),
  });

  if (!registerResponse.ok) {
    throw new Error(`Failed to register permission request: ${registerResponse.statusText}`);
  }

  const { request_id } = await registerResponse.json() as { request_id: string };

  // 2. Long-poll for user decision (5 minute timeout)
  const controller = new AbortController();
  const timeoutId = setTimeout(() => controller.abort(), 5 * 60 * 1000);

  try {
    const decisionResponse = await fetch(
      `${TAURI_API_URL}/api/permission/await/${request_id}`,
      {
        method: "GET",
        signal: controller.signal,
      }
    );

    clearTimeout(timeoutId);

    if (!decisionResponse.ok) {
      if (decisionResponse.status === 408) {
        // Timeout - treat as deny
        return {
          content: [{
            type: "text",
            text: JSON.stringify({
              allowed: false,
              reason: "Permission request timed out waiting for user response",
            }),
          }],
        };
      }
      throw new Error(`Permission decision error: ${decisionResponse.statusText}`);
    }

    const decision = await decisionResponse.json() as PermissionDecision;

    return {
      content: [{
        type: "text",
        text: JSON.stringify({
          allowed: decision.decision === "allow",
          reason: decision.message || (decision.decision === "allow"
            ? "User approved the tool call"
            : "User denied the tool call"),
        }),
      }],
    };
  } catch (error) {
    clearTimeout(timeoutId);
    if (error instanceof Error && error.name === "AbortError") {
      return {
        content: [{
          type: "text",
          text: JSON.stringify({
            allowed: false,
            reason: "Permission request timed out",
          }),
        }],
      };
    }
    throw error;
  }
}
```

**Update MCP Server index.ts:**

```typescript
import { permissionRequestTool, handlePermissionRequest } from "./permission-handler.js";

// Add to tool list (always available, not scoped by agent type)
const PERMISSION_TOOLS = [permissionRequestTool];

// In CallToolRequestSchema handler:
if (name === "permission_request") {
  return handlePermissionRequest(args as Parameters<typeof handlePermissionRequest>[0]);
}
```

---

## Tauri Backend: Permission Endpoints

**File:** `src-tauri/src/http_server.rs` (additions)

```rust
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::{Mutex, oneshot};
use uuid::Uuid;

/// Pending permission request waiting for user decision
struct PendingPermission {
    request_id: String,
    tool_name: String,
    tool_input: serde_json::Value,
    context: Option<String>,
    response_tx: oneshot::Sender<PermissionDecision>,
}

#[derive(Clone, Serialize)]
struct PermissionDecision {
    decision: String,  // "allow" or "deny"
    message: Option<String>,
}

/// Shared state for pending permissions
pub struct PermissionState {
    pending: Mutex<HashMap<String, PendingPermission>>,
}

impl PermissionState {
    pub fn new() -> Self {
        Self {
            pending: Mutex::new(HashMap::new()),
        }
    }
}

// ============================================================================
// HTTP Endpoints
// ============================================================================

#[derive(Deserialize)]
struct PermissionRequestInput {
    tool_name: String,
    tool_input: serde_json::Value,
    context: Option<String>,
}

#[derive(Serialize)]
struct PermissionRequestResponse {
    request_id: String,
}

/// POST /api/permission/request
///
/// Called by MCP server when Claude CLI needs permission for a tool.
/// Registers the request, emits Tauri event, returns request_id.
async fn request_permission(
    State(state): State<Arc<AppState>>,
    Json(input): Json<PermissionRequestInput>,
) -> Json<PermissionRequestResponse> {
    let request_id = Uuid::new_v4().to_string();
    let (tx, _rx) = oneshot::channel();  // rx stored in pending map

    // Store pending request
    {
        let mut pending = state.permission_state.pending.lock().await;
        pending.insert(request_id.clone(), PendingPermission {
            request_id: request_id.clone(),
            tool_name: input.tool_name.clone(),
            tool_input: input.tool_input.clone(),
            context: input.context.clone(),
            response_tx: tx,
        });
    }

    // Emit Tauri event to frontend
    let _ = state.app_handle.emit("permission:request", serde_json::json!({
        "request_id": request_id,
        "tool_name": input.tool_name,
        "tool_input": input.tool_input,
        "context": input.context,
    }));

    Json(PermissionRequestResponse { request_id })
}

/// GET /api/permission/await/:request_id
///
/// Long-poll endpoint. MCP server calls this and blocks until user decides.
/// Returns 408 on timeout (5 minutes).
async fn await_permission(
    State(state): State<Arc<AppState>>,
    Path(request_id): Path<String>,
) -> Result<Json<PermissionDecision>, StatusCode> {
    // Extract the receiver from pending map
    let rx = {
        let mut pending = state.permission_state.pending.lock().await;
        if let Some(p) = pending.remove(&request_id) {
            // Re-insert without the sender (we took it)
            // Actually, we need a different approach - use a channel per request
            // that we can await on
            Some(p.response_tx)  // This won't work as-is
        } else {
            None
        }
    };

    // Better approach: use a broadcast or watch channel
    // For now, simplified polling approach:
    let timeout = tokio::time::Duration::from_secs(300);
    let start = tokio::time::Instant::now();

    loop {
        // Check if decision has been made
        {
            let pending = state.permission_state.pending.lock().await;
            if !pending.contains_key(&request_id) {
                // Request was resolved - check decisions map
                if let Some(decision) = state.permission_decisions.lock().await.remove(&request_id) {
                    return Ok(Json(decision));
                }
            }
        }

        if start.elapsed() > timeout {
            // Clean up and return timeout
            state.permission_state.pending.lock().await.remove(&request_id);
            return Err(StatusCode::REQUEST_TIMEOUT);
        }

        tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
    }
}

#[derive(Deserialize)]
struct ResolvePermissionInput {
    request_id: String,
    decision: String,  // "allow" or "deny"
    message: Option<String>,
}

/// POST /api/permission/resolve
///
/// Called by frontend when user makes a decision.
async fn resolve_permission(
    State(state): State<Arc<AppState>>,
    Json(input): Json<ResolvePermissionInput>,
) -> StatusCode {
    // Store decision for the await endpoint to pick up
    state.permission_decisions.lock().await.insert(
        input.request_id.clone(),
        PermissionDecision {
            decision: input.decision,
            message: input.message,
        },
    );

    // Remove from pending
    state.permission_state.pending.lock().await.remove(&input.request_id);

    StatusCode::OK
}

// Add routes to router:
// .route("/api/permission/request", post(request_permission))
// .route("/api/permission/await/:request_id", get(await_permission))
// .route("/api/permission/resolve", post(resolve_permission))
```

**Alternative: Cleaner implementation with tokio::sync::watch**

```rust
use tokio::sync::watch;

pub struct PermissionState {
    pending: Mutex<HashMap<String, watch::Sender<Option<PermissionDecision>>>>,
}

async fn request_permission(...) -> Json<PermissionRequestResponse> {
    let request_id = Uuid::new_v4().to_string();
    let (tx, _rx) = watch::channel(None);

    state.permission_state.pending.lock().await
        .insert(request_id.clone(), tx);

    // Emit event...

    Json(PermissionRequestResponse { request_id })
}

async fn await_permission(...) -> Result<Json<PermissionDecision>, StatusCode> {
    let mut rx = {
        let pending = state.permission_state.pending.lock().await;
        pending.get(&request_id)
            .map(|tx| tx.subscribe())
            .ok_or(StatusCode::NOT_FOUND)?
    };

    let timeout = tokio::time::Duration::from_secs(300);

    match tokio::time::timeout(timeout, rx.wait_for(|v| v.is_some())).await {
        Ok(Ok(_)) => {
            let decision = rx.borrow().clone().unwrap();
            state.permission_state.pending.lock().await.remove(&request_id);
            Ok(Json(decision))
        }
        _ => {
            state.permission_state.pending.lock().await.remove(&request_id);
            Err(StatusCode::REQUEST_TIMEOUT)
        }
    }
}

async fn resolve_permission(...) -> StatusCode {
    let pending = state.permission_state.pending.lock().await;
    if let Some(tx) = pending.get(&input.request_id) {
        let _ = tx.send(Some(PermissionDecision {
            decision: input.decision,
            message: input.message,
        }));
        StatusCode::OK
    } else {
        StatusCode::NOT_FOUND
    }
}
```

---

## Tauri Command for Frontend

**File:** `src-tauri/src/commands/permission_commands.rs`

```rust
use tauri::State;
use crate::AppState;

#[derive(serde::Deserialize)]
pub struct ResolvePermissionArgs {
    request_id: String,
    decision: String,
    message: Option<String>,
}

#[tauri::command]
pub async fn resolve_permission_request(
    state: State<'_, AppState>,
    args: ResolvePermissionArgs,
) -> Result<(), String> {
    let pending = state.permission_state.pending.lock().await;

    if let Some(tx) = pending.get(&args.request_id) {
        tx.send(Some(PermissionDecision {
            decision: args.decision,
            message: args.message,
        })).map_err(|_| "Failed to send decision")?;
        Ok(())
    } else {
        Err("Permission request not found".to_string())
    }
}

#[tauri::command]
pub async fn get_pending_permissions(
    state: State<'_, AppState>,
) -> Vec<PendingPermissionInfo> {
    let pending = state.permission_state.pending.lock().await;
    pending.values()
        .map(|p| PendingPermissionInfo {
            request_id: p.request_id.clone(),
            tool_name: p.tool_name.clone(),
            tool_input: p.tool_input.clone(),
            context: p.context.clone(),
        })
        .collect()
}
```

---

## Frontend: Permission Dialog Component

**File:** `src/components/PermissionDialog.tsx`

```tsx
import { useEffect, useState } from "react";
import { listen } from "@tauri-apps/api/event";
import { invoke } from "@tauri-apps/api/core";
import {
  Dialog,
  DialogContent,
  DialogHeader,
  DialogTitle,
  DialogFooter,
  DialogDescription,
} from "@/components/ui/dialog";
import { Button } from "@/components/ui/button";
import { AlertTriangle, Shield, Terminal } from "lucide-react";
import { cn } from "@/lib/utils";

interface PermissionRequest {
  request_id: string;
  tool_name: string;
  tool_input: Record<string, unknown>;
  context?: string;
}

export function PermissionDialog() {
  const [requests, setRequests] = useState<PermissionRequest[]>([]);
  const currentRequest = requests[0];

  useEffect(() => {
    const unlisten = listen<PermissionRequest>("permission:request", (event) => {
      setRequests((prev) => [...prev, event.payload]);
    });

    return () => {
      unlisten.then((fn) => fn());
    };
  }, []);

  const handleDecision = async (decision: "allow" | "deny") => {
    if (!currentRequest) return;

    try {
      await invoke("resolve_permission_request", {
        args: {
          request_id: currentRequest.request_id,
          decision,
          message: decision === "deny" ? "User denied permission" : undefined,
        },
      });
    } catch (error) {
      console.error("Failed to resolve permission:", error);
    }

    // Remove from queue
    setRequests((prev) => prev.slice(1));
  };

  if (!currentRequest) return null;

  const toolInputPreview = formatToolInput(currentRequest.tool_name, currentRequest.tool_input);

  return (
    <Dialog open onOpenChange={() => handleDecision("deny")}>
      <DialogContent className="sm:max-w-[500px]">
        <DialogHeader>
          <div className="flex items-center gap-2">
            <div className="p-2 rounded-full bg-warning/10">
              <AlertTriangle className="h-5 w-5 text-warning" />
            </div>
            <DialogTitle>Permission Required</DialogTitle>
          </div>
          <DialogDescription>
            An agent is requesting permission to use a tool
          </DialogDescription>
        </DialogHeader>

        <div className="space-y-4 py-4">
          {/* Tool name */}
          <div className="flex items-center gap-2 text-sm">
            <Terminal className="h-4 w-4 text-muted-foreground" />
            <span className="font-medium">{currentRequest.tool_name}</span>
          </div>

          {/* Tool input preview */}
          <div className="rounded-md bg-muted p-3 font-mono text-sm overflow-x-auto">
            <pre className="whitespace-pre-wrap break-all">
              {toolInputPreview}
            </pre>
          </div>

          {/* Context if provided */}
          {currentRequest.context && (
            <p className="text-sm text-muted-foreground">
              {currentRequest.context}
            </p>
          )}

          {/* Queue indicator */}
          {requests.length > 1 && (
            <p className="text-xs text-muted-foreground">
              +{requests.length - 1} more permission request(s) waiting
            </p>
          )}
        </div>

        <DialogFooter className="gap-2 sm:gap-0">
          <Button
            variant="outline"
            onClick={() => handleDecision("deny")}
          >
            Deny
          </Button>
          <Button
            onClick={() => handleDecision("allow")}
            className="bg-primary"
          >
            <Shield className="h-4 w-4 mr-2" />
            Allow
          </Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  );
}

function formatToolInput(toolName: string, input: Record<string, unknown>): string {
  // Special formatting for common tools
  switch (toolName) {
    case "Bash":
      return input.command as string || JSON.stringify(input, null, 2);
    case "Write":
      return `Write to: ${input.file_path}\n\n${(input.content as string)?.slice(0, 200)}${
        (input.content as string)?.length > 200 ? "..." : ""
      }`;
    case "Edit":
      return `Edit: ${input.file_path}\n- "${input.old_string}"\n+ "${input.new_string}"`;
    case "Read":
      return `Read: ${input.file_path}`;
    default:
      return JSON.stringify(input, null, 2);
  }
}
```

**File:** `src/components/PermissionDialog.module.css` (optional styling)

---

## Update Claude CLI Spawn to Use Permission Handler

**File:** `src-tauri/src/infrastructure/agents/claude/claude_code_client.rs`

```rust
impl ClaudeCodeClient {
    pub async fn spawn_agent(&self, config: AgentSpawnConfig) -> Result<AgentHandle, AgentError> {
        let mut cmd = Command::new(&self.cli_path);

        cmd.args([
            "--plugin-dir", "./ralphx-plugin",
            "--output-format", "stream-json",
        ]);

        // Add permission prompt tool for UI-based approval
        // The MCP tool name format: mcp__<server>__<tool>
        cmd.args([
            "--permission-prompt-tool",
            "mcp__ralphx__permission_request"
        ]);

        // Pass agent type for MCP tool scoping
        cmd.env("RALPHX_AGENT_TYPE", &config.agent);

        // ... rest of spawn logic
    }
}
```

---

## Frontend Integration

**File:** `src/App.tsx` (or root layout)

```tsx
import { PermissionDialog } from "@/components/PermissionDialog";

function App() {
  return (
    <>
      {/* ... existing app content */}

      {/* Global permission dialog - always mounted */}
      <PermissionDialog />
    </>
  );
}
```

---

## Files Summary for Permission Bridge

**New Files:**

| File | Purpose |
|------|---------|
| `ralphx-mcp-server/src/permission-handler.ts` | MCP tool that handles permission prompts |
| `src-tauri/src/application/permission_state.rs` | Shared state for pending permissions |
| `src-tauri/src/commands/permission_commands.rs` | Tauri commands for permission resolution |
| `src/components/PermissionDialog.tsx` | UI for permission approval/denial |
| `src/types/permission.ts` | TypeScript types for permission events |

**Modified Files:**

| File | Change |
|------|--------|
| `ralphx-mcp-server/src/index.ts` | Register permission_request tool |
| `src-tauri/src/http_server.rs` | Add permission endpoints |
| `src-tauri/src/lib.rs` | Initialize PermissionState, register commands |
| `src-tauri/src/infrastructure/agents/claude/claude_code_client.rs` | Add `--permission-prompt-tool` flag |
| `src/App.tsx` | Mount PermissionDialog globally |
