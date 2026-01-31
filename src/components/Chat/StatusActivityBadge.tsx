/**
 * StatusActivityBadge - Unified status badge with activity navigation
 *
 * Replaces both the Badge + WorkerExecutingIndicator with a single component
 * that shows agent status and provides one-click navigation to Activity view
 * with context filter set.
 *
 * Behavior by state:
 * - Idle, no activity: Hidden
 * - Idle, has activity: Muted Activity icon (clickable)
 * - Agent active: Badge with status + spinner + Activity icon
 */

import { Activity, Loader2 } from "lucide-react";
import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import { useUiStore } from "@/stores/uiStore";
import type { ViewType } from "@/types/chat";

// ============================================================================
// Types
// ============================================================================

export type AgentType = "worker" | "reviewer" | "agent" | "idle";

export interface StatusActivityBadgeProps {
  /** Whether an agent is currently active (running/sending) */
  isAgentActive: boolean;
  /** Type of agent that is active */
  agentType: AgentType;
  /** Current view context type */
  contextType: ViewType;
  /** Context ID (taskId or sessionId) for activity filtering */
  contextId: string | null;
  /** Whether there is historical activity available for this context */
  hasActivity?: boolean;
}

// ============================================================================
// Helper Functions
// ============================================================================

function getStatusText(agentType: AgentType): string {
  switch (agentType) {
    case "worker":
      return "Worker running...";
    case "reviewer":
      return "Reviewing...";
    case "agent":
      return "Agent responding...";
    default:
      return "Working...";
  }
}

// ============================================================================
// Component
// ============================================================================

export function StatusActivityBadge({
  isAgentActive,
  agentType,
  contextType,
  contextId,
  hasActivity = false,
}: StatusActivityBadgeProps) {
  const setActivityFilter = useUiStore((s) => s.setActivityFilter);
  const setCurrentView = useUiStore((s) => s.setCurrentView);

  // Navigate to activity view with context filter
  const handleActivityClick = () => {
    // Set filter based on context type
    if (contextType === "ideation") {
      setActivityFilter({ sessionId: contextId, taskId: null });
    } else {
      // task, task_detail, kanban with selected task
      setActivityFilter({ taskId: contextId, sessionId: null });
    }
    setCurrentView("activity");
  };

  // Hidden state: idle with no activity
  if (!isAgentActive && !hasActivity) {
    return null;
  }

  // Idle but has activity: show muted activity icon
  if (!isAgentActive && hasActivity) {
    return (
      <Button
        variant="ghost"
        size="sm"
        onClick={handleActivityClick}
        className="shrink-0 h-7 px-2 text-white/40 hover:text-white/60"
        aria-label="View activity"
      >
        <Activity className="w-3.5 h-3.5" />
      </Button>
    );
  }

  // Active state: badge with status text, spinner, and activity button
  return (
    <div className="flex items-center gap-1.5 shrink-0">
      <Badge variant="secondary" className="shrink-0">
        <Loader2 className="w-3 h-3 mr-1.5 animate-spin" />
        {getStatusText(agentType)}
      </Badge>
      <Button
        variant="ghost"
        size="sm"
        onClick={handleActivityClick}
        className="shrink-0 h-7 px-2 text-[#ff6b35] hover:text-[#ff6b35]/80"
        aria-label="View activity"
      >
        <Activity className="w-3.5 h-3.5" />
      </Button>
    </div>
  );
}
