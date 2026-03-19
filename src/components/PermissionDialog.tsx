import { useEffect, useState } from "react";
import { toast } from "sonner";
import { useEventBus } from "@/providers/EventProvider";
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
import { useTaskStore } from "@/stores/taskStore";
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
 * - Shows agent identity (agent type, context type, task name) when available
 * - Prevents double-submit with resolving state
 */

const AGENT_BADGE_CONFIG: Record<string, { label: string; colorVar: string }> = {
  "ralphx-worker": { label: "Worker", colorVar: "--status-info" },
  "ralphx-coder": { label: "Coder", colorVar: "--status-info" },
  "ralphx-worker-team": { label: "Worker", colorVar: "--status-info" },
  "ralphx-merger": { label: "Merger", colorVar: "--status-warning" },
  "orchestrator-ideation": { label: "Ideation", colorVar: "--accent-primary" },
  "ideation-team-lead": { label: "Ideation", colorVar: "--accent-primary" },
};

const CONTEXT_LABEL_MAP: Record<string, string> = {
  task_execution: "Executing",
  review: "Reviewing",
  merge: "Merging",
  ideation: "Ideation",
  task: "Task Chat",
  project: "Project Chat",
};

export function PermissionDialog() {
  const [requests, setRequests] = useState<PermissionRequest[]>([]);
  const [resolving, setResolving] = useState(false);
  const eventBus = useEventBus();
  const currentRequest = requests[0];

  // Listen to permission request events from backend
  useEffect(() => {
    const unsubscribe = eventBus.subscribe<PermissionRequest>("permission:request", (payload) => {
      setRequests((prev) => [...prev, payload]);
    });

    return unsubscribe;
  }, [eventBus]);

  const handleDecision = async (decision: "allow" | "deny") => {
    if (!currentRequest) return;

    setResolving(true);
    try {
      await api.permission.resolveRequest({
        requestId: currentRequest.request_id,
        decision,
        ...(decision === "deny" && { message: "User denied permission" }),
      });
      // Remove from queue only on success
      setRequests((prev) => prev.slice(1));
    } catch (error) {
      console.error("Failed to resolve permission:", error);
      toast.error("Failed to resolve permission request");
    } finally {
      setResolving(false);
    }
  };

  // Dialog not visible when no requests
  if (!currentRequest) return null;

  const toolInputPreview = formatToolInput(
    currentRequest.tool_name,
    currentRequest.tool_input
  );

  const hasIdentity =
    Boolean(currentRequest.agent_type) ||
    Boolean(currentRequest.context_type) ||
    Boolean(currentRequest.task_id);

  return (
    <Dialog
      open
      onOpenChange={(open) => {
        if (!resolving && !open) void handleDecision("deny");
      }}
    >
      <DialogContent className="sm:max-w-[500px] max-h-[85vh] flex flex-col">
        <DialogHeader className="shrink-0">
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

        <div className="space-y-4 py-4 px-6 overflow-y-auto min-h-0">
          {/* Agent identity row */}
          {hasIdentity && (
            <div
              className="rounded-md p-3 space-y-1"
              style={{
                backgroundColor: "var(--bg-surface)",
                border: "1px solid var(--border-subtle)",
              }}
            >
              {/* Badges row */}
              <div className="flex items-center gap-2 flex-wrap">
                {currentRequest.agent_type && (() => {
                  const badge =
                    AGENT_BADGE_CONFIG[currentRequest.agent_type] ??
                    { label: currentRequest.agent_type, colorVar: "--text-secondary" };
                  return (
                    <span
                      className="text-xs font-medium px-2 py-0.5 rounded"
                      style={{
                        backgroundColor: `color-mix(in srgb, var(${badge.colorVar}) 15%, transparent)`,
                        color: `var(${badge.colorVar})`,
                      }}
                    >
                      {badge.label}
                    </span>
                  );
                })()}
                {currentRequest.context_type && (
                  <span className="text-xs" style={{ color: "var(--text-secondary)" }}>
                    {CONTEXT_LABEL_MAP[currentRequest.context_type] ?? currentRequest.context_type}
                  </span>
                )}
              </div>
              {/* Task name row */}
              {currentRequest.task_id && (() => {
                const tasks = useTaskStore.getState().tasks;
                const taskTitle =
                  tasks[currentRequest.task_id]?.title ??
                  currentRequest.task_id.slice(0, 8);
                return (
                  <p className="text-xs" style={{ color: "var(--text-muted)" }}>
                    Task: {taskTitle}
                  </p>
                );
              })()}
            </div>
          )}

          {/* Tool name */}
          <div className="flex items-center gap-2 text-sm">
            <Terminal className="h-4 w-4 shrink-0" style={{ color: "var(--text-muted)" }} />
            <span className="font-medium" style={{ color: "var(--text-primary)" }}>
              {currentRequest.tool_name}
            </span>
          </div>

          {/* Tool input preview */}
          <div
            className="rounded-md p-3 font-mono text-sm overflow-x-auto max-h-[50vh]"
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

        <DialogFooter className="shrink-0">
          <Button
            variant="outline"
            onClick={() => void handleDecision("deny")}
            disabled={resolving}
          >
            Deny
          </Button>
          <Button onClick={() => void handleDecision("allow")} disabled={resolving}>
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
