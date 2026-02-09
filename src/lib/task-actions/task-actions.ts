/**
 * Unified task action registry — single source of truth for which actions
 * are available per status, for both Kanban and Graph context menus.
 *
 * Actions are data-only (label, icon, handler key, confirmation config).
 * Rendering is handled by each surface's context menu component.
 */

import {
  Play,
  Ban,
  Unlock,
  X,
  RotateCcw,
  MessageSquare,
  ClipboardCheck,
  Check,
  AlertTriangle,
  GitMerge,
  FileWarning,
} from "lucide-react";
import type { InternalStatus } from "@/types/status";
import type { TaskAction, ActionSurface } from "./types";
import { CONFIRMATION_CONFIGS } from "./constants";

/**
 * Get task actions for a given status and surface context.
 *
 * Kanban surface returns lifecycle/CRUD actions (cancel, block, unblock, re-open, retry).
 * Graph surface returns status-appropriate quick actions (start, approve, reject, etc.).
 * Both surfaces share the same action definitions and confirmation configs.
 */
export function getTaskActions(status: InternalStatus, surface: ActionSurface): TaskAction[] {
  if (surface === "kanban") {
    return getKanbanActions(status);
  }
  return getGraphActions(status);
}

/** Kanban-specific actions: lifecycle transitions */
function getKanbanActions(status: InternalStatus): TaskAction[] {
  switch (status) {
    case "backlog":
      return [
        {
          id: "cancel",
          label: "Cancel",
          icon: X,
          handlerKey: "onStatusChange",
          variant: "destructive",
          confirmConfig: CONFIRMATION_CONFIGS.cancelled,
        },
      ];

    case "ready":
      return [
        {
          id: "block",
          label: "Block",
          icon: Ban,
          handlerKey: "onBlockWithReason",
          opensDialog: true,
        },
        {
          id: "cancel",
          label: "Cancel",
          icon: X,
          handlerKey: "onStatusChange",
          variant: "destructive",
          confirmConfig: CONFIRMATION_CONFIGS.cancelled,
        },
      ];

    case "blocked":
      return [
        {
          id: "unblock",
          label: "Unblock",
          icon: Unlock,
          handlerKey: "onUnblock",
          confirmConfig: CONFIRMATION_CONFIGS.unblock,
        },
        {
          id: "cancel",
          label: "Cancel",
          icon: X,
          handlerKey: "onStatusChange",
          variant: "destructive",
          confirmConfig: CONFIRMATION_CONFIGS.cancelled,
        },
      ];

    case "approved":
      return [
        {
          id: "reopen",
          label: "Re-open",
          icon: RotateCcw,
          handlerKey: "onStatusChange",
          confirmConfig: CONFIRMATION_CONFIGS.backlog,
        },
      ];

    case "failed":
      return [
        {
          id: "retry",
          label: "Retry",
          icon: RotateCcw,
          handlerKey: "onStatusChange",
          confirmConfig: CONFIRMATION_CONFIGS.retry,
        },
      ];

    case "cancelled":
      return [
        {
          id: "reopen",
          label: "Re-open",
          icon: RotateCcw,
          handlerKey: "onStatusChange",
          confirmConfig: CONFIRMATION_CONFIGS.backlog,
        },
      ];

    default:
      return [];
  }
}

/** Graph-specific actions: status-appropriate quick actions */
function getGraphActions(status: InternalStatus): TaskAction[] {
  switch (status) {
    case "ready":
      return [
        {
          id: "start",
          label: "Start Execution",
          icon: Play,
          handlerKey: "onStartExecution",
          confirmConfig: CONFIRMATION_CONFIGS.start,
        },
        {
          id: "block",
          label: "Block",
          icon: Ban,
          handlerKey: "onBlockWithReason",
          opensDialog: true,
        },
      ];

    case "blocked":
      return [
        {
          id: "unblock",
          label: "Unblock",
          icon: Unlock,
          handlerKey: "onUnblock",
          confirmConfig: CONFIRMATION_CONFIGS.unblock,
        },
        {
          id: "view-blockers",
          label: "View Blockers",
          icon: AlertTriangle,
          handlerKey: "onViewAgentChat",
          isViewAction: true,
        },
      ];

    case "executing":
    case "re_executing":
      return [
        {
          id: "view-chat",
          label: "View Agent Chat",
          icon: MessageSquare,
          handlerKey: "onViewAgentChat",
          isViewAction: true,
        },
      ];

    case "pending_review":
      return [
        {
          id: "view-summary",
          label: "View Work Summary",
          icon: ClipboardCheck,
          handlerKey: "onViewAgentChat",
          isViewAction: true,
        },
      ];

    case "review_passed":
      return [
        {
          id: "approve",
          label: "Approve",
          icon: Check,
          handlerKey: "onApprove",
          confirmConfig: CONFIRMATION_CONFIGS.approve,
        },
        {
          id: "request-changes",
          label: "Request Changes",
          icon: RotateCcw,
          handlerKey: "onRequestChanges",
          confirmConfig: CONFIRMATION_CONFIGS["request-changes"],
        },
      ];

    case "escalated":
      return [
        {
          id: "approve",
          label: "Approve",
          icon: Check,
          handlerKey: "onApprove",
          confirmConfig: CONFIRMATION_CONFIGS.approve,
        },
        {
          id: "reject",
          label: "Reject",
          icon: X,
          variant: "destructive",
          handlerKey: "onReject",
          confirmConfig: CONFIRMATION_CONFIGS.reject,
        },
        {
          id: "request-changes",
          label: "Request Changes",
          icon: RotateCcw,
          handlerKey: "onRequestChanges",
          confirmConfig: CONFIRMATION_CONFIGS["request-changes"],
        },
      ];

    case "revision_needed":
      return [
        {
          id: "view-feedback",
          label: "View Feedback",
          icon: FileWarning,
          handlerKey: "onViewAgentChat",
          isViewAction: true,
        },
      ];

    case "merge_conflict":
      return [
        {
          id: "view-conflicts",
          label: "View Conflicts",
          icon: GitMerge,
          handlerKey: "onViewAgentChat",
          isViewAction: true,
        },
        {
          id: "mark-resolved",
          label: "Mark Resolved",
          icon: Check,
          handlerKey: "onMarkResolved",
          confirmConfig: CONFIRMATION_CONFIGS["mark-resolved"],
        },
      ];

    default:
      return [];
  }
}
