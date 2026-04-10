import type { ReactNode } from "react";
import { StatusActivityBadge } from "./StatusActivityBadge";
import type { AgentType, StatusActivityBadgeProps } from "./StatusActivityBadge";
import type { ContextType, ModelDisplay } from "@/types/chat-conversation";
import type { AgentStatus } from "@/stores/chatStore";
import {
  describeProviderLineage,
  formatProviderHarnessLabel,
  formatProviderSessionSnippet,
  getProviderHarnessBadgeStyle,
} from "./provider-harness";

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
  providerHarness?: string | null;
  providerSessionId?: string | null;
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
  providerHarness,
  providerSessionId,
}: ChatSessionToolbarProps) {
  const harnessLabel = formatProviderHarnessLabel(providerHarness);
  const harnessStyle = getProviderHarnessBadgeStyle(providerHarness);
  const providerLineage = describeProviderLineage({
    providerHarness,
    providerSessionId,
  });
  const providerSessionSnippet = formatProviderSessionSnippet(providerSessionId);

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
        <div
          className="flex min-w-0 items-center gap-2"
          data-testid="chat-session-provider-context"
        >
          {harnessLabel && (
            <span
              className="rounded-full px-1.5 py-0.5 text-[9px] font-semibold uppercase tracking-[0.08em]"
              style={harnessStyle}
            >
              {harnessLabel}
            </span>
          )}
          <span
            className="truncate text-[11px] text-white/45"
            data-testid="chat-session-routing"
          >
            {providerLineage}
          </span>
          {providerSessionSnippet && (
            <span
              className="font-mono text-[10px] text-white/35"
              data-testid="chat-session-provider-id"
            >
              {providerSessionSnippet}
            </span>
          )}
        </div>
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
