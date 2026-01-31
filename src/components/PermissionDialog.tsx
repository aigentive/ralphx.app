import { useEffect, useState } from "react";
import { listen } from "@tauri-apps/api/event";
import { api } from "@/lib/tauri";
import {
  Dialog,
  DialogContent,
  DialogHeader,
  DialogTitle,
  DialogDescription,
  DialogFooter,
} from "@/components/ui/dialog";
import { Button } from "@/components/ui/button";
import { AlertTriangle, Shield, Terminal } from "lucide-react";
import type { PermissionRequest } from "@/types/permission";

/**
 * Global permission dialog for approving agent tool usage.
 *
 * Listens to `permission:request` events from the backend and displays
 * a modal dialog asking the user to approve or deny the tool call.
 *
 * Features:
 * - Queues multiple permission requests (shows first, counts remaining)
 * - Formats tool input preview based on tool type (Bash, Write, Edit, Read)
 * - Calls `resolve_permission_request` Tauri command on decision
 * - Closing dialog is treated as "deny"
 */
export function PermissionDialog() {
  const [requests, setRequests] = useState<PermissionRequest[]>([]);
  const currentRequest = requests[0];

  // Listen to permission request events from backend
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
      await api.permission.resolveRequest({
        requestId: currentRequest.request_id,
        decision,
        ...(decision === "deny" && { message: "User denied permission" }),
      });
    } catch (error) {
      console.error("Failed to resolve permission:", error);
    }

    // Remove from queue
    setRequests((prev) => prev.slice(1));
  };

  // Dialog not visible when no requests
  if (!currentRequest) return null;

  const toolInputPreview = formatToolInput(
    currentRequest.tool_name,
    currentRequest.tool_input
  );

  return (
    <Dialog open onOpenChange={() => handleDecision("deny")}>
      <DialogContent className="sm:max-w-[500px]">
        <DialogHeader>
          <div className="flex items-center gap-2">
            <div
              className="p-2 rounded-full"
              style={{
                backgroundColor: "var(--status-warning-muted, rgba(245, 158, 11, 0.15))",
              }}
            >
              <AlertTriangle className="h-5 w-5" style={{ color: "var(--status-warning)" }} />
            </div>
            <DialogTitle>Permission Required</DialogTitle>
          </div>
          <DialogDescription>
            An agent is requesting permission to use a tool
          </DialogDescription>
        </DialogHeader>

        <div className="space-y-4 py-4 px-6">
          {/* Tool name */}
          <div className="flex items-center gap-2 text-sm">
            <Terminal className="h-4 w-4" style={{ color: "var(--text-muted)" }} />
            <span className="font-medium" style={{ color: "var(--text-primary)" }}>
              {currentRequest.tool_name}
            </span>
          </div>

          {/* Tool input preview */}
          <div
            className="rounded-md p-3 font-mono text-sm overflow-x-auto"
            style={{
              backgroundColor: "var(--bg-surface)",
              border: "1px solid var(--border-subtle)",
            }}
          >
            <pre
              className="whitespace-pre-wrap break-all"
              style={{ color: "var(--text-secondary)" }}
            >
              {toolInputPreview}
            </pre>
          </div>

          {/* Context if provided */}
          {currentRequest.context && (
            <p className="text-sm" style={{ color: "var(--text-secondary)" }}>
              {currentRequest.context}
            </p>
          )}

          {/* Queue indicator */}
          {requests.length > 1 && (
            <p className="text-xs" style={{ color: "var(--text-muted)" }}>
              +{requests.length - 1} more permission request(s) waiting
            </p>
          )}
        </div>

        <DialogFooter>
          <Button variant="outline" onClick={() => handleDecision("deny")}>
            Deny
          </Button>
          <Button onClick={() => handleDecision("allow")}>
            <Shield className="h-4 w-4 mr-2" />
            Allow
          </Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  );
}

/**
 * Format tool input for display based on tool type.
 *
 * - Bash: show command
 * - Write: show file path + content preview (first 200 chars)
 * - Edit: show file path + old/new strings
 * - Read: show file path
 * - Default: JSON.stringify
 */
function formatToolInput(
  toolName: string,
  input: Record<string, unknown>
): string {
  switch (toolName) {
    case "Bash":
      return (input.command as string) || JSON.stringify(input, null, 2);
    case "Write": {
      const content = input.content as string;
      const preview = content?.slice(0, 200) || "";
      const truncated = content?.length > 200 ? "..." : "";
      return `Write to: ${input.file_path}\n\n${preview}${truncated}`;
    }
    case "Edit":
      return `Edit: ${input.file_path}\n- "${input.old_string}"\n+ "${input.new_string}"`;
    case "Read":
      return `Read: ${input.file_path}`;
    default:
      return JSON.stringify(input, null, 2);
  }
}
