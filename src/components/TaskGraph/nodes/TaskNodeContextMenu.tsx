/**
 * TaskNodeContextMenu - Right-click context menu for task graph nodes
 *
 * Provides status-appropriate quick actions based on the PRD spec:
 * - ready: Start Execution, Block (with reason)
 * - blocked: Unblock, View Blockers
 * - executing: View Agent Chat
 * - pending_review: View Work Summary
 * - review_passed: Approve, Request Changes
 * - escalated: Approve, Reject, Request Changes
 * - revision_needed: View Feedback
 * - merge_conflict: View Conflicts, Mark Resolved
 *
 * Per spec: Phase E.1 of Task Graph View implementation
 */

import { useState } from "react";
import {
  ContextMenu,
  ContextMenuContent,
  ContextMenuItem,
  ContextMenuSeparator,
  ContextMenuTrigger,
} from "@/components/ui/context-menu";
import {
  Eye,
  Play,
  Ban,
  Unlock,
  MessageSquare,
  ClipboardCheck,
  Check,
  RotateCcw,
  X,
  AlertTriangle,
  GitMerge,
  FileWarning,
} from "lucide-react";
import type { Task } from "@/types/task";
import type { InternalStatus } from "@/types/status";
import { useConfirmation } from "@/hooks/useConfirmation";
import { BlockReasonDialog } from "@/components/tasks/BlockReasonDialog";

// ============================================================================
// Types
// ============================================================================

export interface TaskNodeContextMenuProps {
  task: Task;
  children: React.ReactNode;
  /** Handler to view task details (opens TaskDetailOverlay) */
  onViewDetails: () => void;
  /** Handler to start task execution */
  onStartExecution?: () => void;
  /** Handler for blocking a task with an optional reason */
  onBlockWithReason?: (reason?: string) => void;
  /** Handler for unblocking a task */
  onUnblock?: () => void;
  /** Handler to view agent chat */
  onViewAgentChat?: () => void;
  /** Handler to approve a task */
  onApprove?: () => void;
  /** Handler to reject a task */
  onReject?: () => void;
  /** Handler to request changes */
  onRequestChanges?: () => void;
  /** Handler to mark merge conflict as resolved */
  onMarkResolved?: () => void;
}

// ============================================================================
// Action Configuration
// ============================================================================

interface QuickAction {
  id: string;
  label: string;
  icon: React.ComponentType<{ className?: string }>;
  variant?: "default" | "destructive";
  /** Which handler key this action uses */
  handlerKey: keyof Omit<
    TaskNodeContextMenuProps,
    "task" | "children" | "onViewDetails"
  >;
}

/**
 * Get quick actions based on current task status
 * Per PRD spec: "Quick Actions Context Menu" section
 */
function getQuickActions(status: InternalStatus): QuickAction[] {
  const actions: QuickAction[] = [];

  switch (status) {
    case "ready":
      actions.push(
        {
          id: "start",
          label: "Start Execution",
          icon: Play,
          handlerKey: "onStartExecution",
        },
        {
          id: "block",
          label: "Block",
          icon: Ban,
          handlerKey: "onBlockWithReason",
        }
      );
      break;

    case "blocked":
      actions.push(
        {
          id: "unblock",
          label: "Unblock",
          icon: Unlock,
          handlerKey: "onUnblock",
        },
        {
          id: "view-blockers",
          label: "View Blockers",
          icon: AlertTriangle,
          handlerKey: "onViewAgentChat", // Opens details to see blocked_reason
        }
      );
      break;

    case "executing":
    case "re_executing":
      actions.push({
        id: "view-chat",
        label: "View Agent Chat",
        icon: MessageSquare,
        handlerKey: "onViewAgentChat",
      });
      break;

    case "pending_review":
      actions.push({
        id: "view-summary",
        label: "View Work Summary",
        icon: ClipboardCheck,
        handlerKey: "onViewAgentChat", // Opens details to see work summary
      });
      break;

    case "review_passed":
      actions.push(
        {
          id: "approve",
          label: "Approve",
          icon: Check,
          handlerKey: "onApprove",
        },
        {
          id: "request-changes",
          label: "Request Changes",
          icon: RotateCcw,
          handlerKey: "onRequestChanges",
        }
      );
      break;

    case "escalated":
      actions.push(
        {
          id: "approve",
          label: "Approve",
          icon: Check,
          handlerKey: "onApprove",
        },
        {
          id: "reject",
          label: "Reject",
          icon: X,
          variant: "destructive",
          handlerKey: "onReject",
        },
        {
          id: "request-changes",
          label: "Request Changes",
          icon: RotateCcw,
          handlerKey: "onRequestChanges",
        }
      );
      break;

    case "revision_needed":
      actions.push({
        id: "view-feedback",
        label: "View Feedback",
        icon: FileWarning,
        handlerKey: "onViewAgentChat", // Opens details to see revision feedback
      });
      break;

    case "merge_conflict":
      actions.push(
        {
          id: "view-conflicts",
          label: "View Conflicts",
          icon: GitMerge,
          handlerKey: "onViewAgentChat", // Opens details to see conflicts
        },
        {
          id: "mark-resolved",
          label: "Mark Resolved",
          icon: Check,
          handlerKey: "onMarkResolved",
        }
      );
      break;

    // Other statuses don't have special quick actions
    // But all statuses have "View Details" available
  }

  return actions;
}

// ============================================================================
// Confirmation Messages
// ============================================================================

const confirmationMessages: Record<
  string,
  { title: string; description: string; variant: "default" | "destructive" }
> = {
  start: {
    title: "Start Execution?",
    description: "The task will be queued for execution by an AI worker.",
    variant: "default",
  },
  unblock: {
    title: "Unblock this task?",
    description:
      "The task will be moved back to ready and the blocked reason will be cleared.",
    variant: "default",
  },
  approve: {
    title: "Approve this task?",
    description: "The task will be marked as approved and completed.",
    variant: "default",
  },
  reject: {
    title: "Reject this task?",
    description: "The task will be marked as failed.",
    variant: "destructive",
  },
  "request-changes": {
    title: "Request changes?",
    description: "The task will be sent back for revision.",
    variant: "default",
  },
  "mark-resolved": {
    title: "Mark conflicts as resolved?",
    description: "The task will proceed with the merge.",
    variant: "default",
  },
};

// ============================================================================
// Component
// ============================================================================

export function TaskNodeContextMenu({
  task,
  children,
  onViewDetails,
  onStartExecution,
  onBlockWithReason,
  onUnblock,
  onViewAgentChat,
  onApprove,
  onReject,
  onRequestChanges,
  onMarkResolved,
}: TaskNodeContextMenuProps) {
  const { confirm, confirmationDialogProps, ConfirmationDialog } =
    useConfirmation();
  const [showBlockDialog, setShowBlockDialog] = useState(false);

  const quickActions = getQuickActions(task.internalStatus);

  // Map handler keys to actual handlers
  const handlers: Record<string, (() => void) | undefined> = {
    onStartExecution,
    onBlockWithReason: () => setShowBlockDialog(true), // Opens dialog instead
    onUnblock,
    onViewAgentChat: onViewAgentChat ?? onViewDetails, // Fallback to details
    onApprove,
    onReject,
    onRequestChanges,
    onMarkResolved,
  };

  const handleAction = async (action: QuickAction) => {
    const handler = handlers[action.handlerKey];
    if (!handler) {
      // If no specific handler, fall back to view details
      onViewDetails();
      return;
    }

    // Special case: block opens dialog without confirmation
    if (action.id === "block") {
      setShowBlockDialog(true);
      return;
    }

    // Special case: view actions don't need confirmation
    if (
      action.id.startsWith("view-") ||
      action.handlerKey === "onViewAgentChat"
    ) {
      handler();
      return;
    }

    // Other actions need confirmation
    const messages = confirmationMessages[action.id];
    if (messages) {
      const confirmed = await confirm({
        title: messages.title,
        description: messages.description,
        confirmText: action.label,
        variant: messages.variant,
      });
      if (confirmed) {
        handler();
      }
    } else {
      // No confirmation config, just execute
      handler();
    }
  };

  return (
    <ContextMenu>
      <ContextMenuTrigger asChild>{children}</ContextMenuTrigger>
      <ContextMenuContent data-testid="task-node-context-menu">
        {/* Always show View Details first */}
        <ContextMenuItem
          onClick={onViewDetails}
          data-testid="view-details-action"
        >
          <Eye className="w-4 h-4 mr-2" />
          View Details
        </ContextMenuItem>

        {/* Quick actions based on status */}
        {quickActions.length > 0 && (
          <>
            <ContextMenuSeparator />
            {quickActions.map((action) => (
              <ContextMenuItem
                key={action.id}
                onClick={() => handleAction(action)}
                className={action.variant === "destructive" ? "text-destructive" : ""}
                data-testid={`${action.id}-action`}
              >
                <action.icon className="w-4 h-4 mr-2" />
                {action.label}
              </ContextMenuItem>
            ))}
          </>
        )}
      </ContextMenuContent>

      {/* Dialogs */}
      <ConfirmationDialog {...confirmationDialogProps} />
      <BlockReasonDialog
        isOpen={showBlockDialog}
        onClose={() => setShowBlockDialog(false)}
        onConfirm={(reason) => {
          onBlockWithReason?.(reason);
          setShowBlockDialog(false);
        }}
        taskTitle={task.title}
      />
    </ContextMenu>
  );
}
