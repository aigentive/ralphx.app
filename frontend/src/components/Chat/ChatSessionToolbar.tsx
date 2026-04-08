import type { ReactNode } from "react";
import { StatusActivityBadge } from "./StatusActivityBadge";
import type { AgentType, StatusActivityBadgeProps } from "./StatusActivityBadge";
import type { ContextType, ModelDisplay } from "@/types/chat-conversation";
import type { AgentStatus } from "@/stores/chatStore";

export interface ChatSessionToolbarProps {
  backAction?: {
    label: string;
    icon?: ReactNode;
    onClick: () => void;
  };
  isAgentActive: StatusActivityBadgeProps["isAgentActive"];
  agentType: AgentType;
  contextType: ContextType;
  contextId: string | null;
  hasActivity?: boolean;
  agentStatus?: AgentStatus;
  storeKey?: string;
  modelDisplay?: ModelDisplay;
}

export function ChatSessionToolbar({
  backAction,
  isAgentActive,
  agentType,
  contextType,
  contextId,
  hasActivity,
  agentStatus,
  storeKey,
  modelDisplay,
}: ChatSessionToolbarProps) {
  return (
    <div
      className="flex items-center px-3 py-1.5 shrink-0"
      style={{ borderBottom: "1px solid hsl(220 10% 14%)" }}
    >
      <div className="flex items-center flex-1 gap-2">
        {backAction && (
          <button
            data-testid="back-to-plan-button"
            onClick={backAction.onClick}
            className="flex items-center gap-1 text-xs text-white/50 hover:text-white/80 transition-colors"
          >
            {backAction.icon}
            <span>{backAction.label}</span>
          </button>
        )}
      </div>
      <div className="flex items-center shrink-0">
        <StatusActivityBadge
          isAgentActive={isAgentActive}
          agentType={agentType}
          contextType={contextType}
          contextId={contextId}
          {...(hasActivity !== undefined ? { hasActivity } : {})}
          {...(agentStatus !== undefined ? { agentStatus } : {})}
          {...(storeKey !== undefined ? { storeKey } : {})}
          {...(modelDisplay !== undefined ? { modelDisplay } : {})}
        />
      </div>
    </div>
  );
}
