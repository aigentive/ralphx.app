import type { ReactNode } from "react";
import { StatusActivityBadge } from "./StatusActivityBadge";
import type { AgentType, StatusActivityBadgeProps } from "./StatusActivityBadge";
import type { ContextType, ModelDisplay } from "@/types/chat-conversation";
import type { AgentStatus } from "@/stores/chatStore";
import {
  formatProviderHarnessLabel,
  formatProviderTooltip,
  getProviderHarnessBadgeStyle,
} from "./provider-harness";
import { ModelChip } from "./ModelChip";

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
  const providerTooltip = formatProviderTooltip({
    providerHarness,
    providerSessionId,
  });

  return (
    <div
      className="flex items-center gap-3 px-3 py-1.5 shrink-0"
      style={{ borderBottom: "1px solid hsl(220 10% 14%)" }}
    >
      <div className="flex min-w-0 flex-1 items-center gap-2">
        {backAction && (
          <button
            data-testid="back-to-plan-button"
            onClick={backAction.onClick}
            className="flex shrink-0 items-center gap-1 text-xs text-white/50 hover:text-white/80 transition-colors"
          >
            {backAction.icon}
            <span>{backAction.label}</span>
          </button>
        )}
        <div
          className="flex min-w-0 flex-1 items-center gap-2"
          data-testid="chat-session-provider-context"
        >
          {harnessLabel && (
            <span
              className="rounded-full px-1.5 py-0.5 text-[9px] font-semibold uppercase tracking-[0.08em]"
              style={harnessStyle}
              title={providerTooltip ?? undefined}
              aria-label={providerTooltip ?? harnessLabel}
              data-testid="chat-session-provider-badge"
            >
              {harnessLabel}
            </span>
          )}
          {modelDisplay && <ModelChip model={modelDisplay} />}
        </div>
      </div>
      <div className="flex shrink-0 items-center">
        <StatusActivityBadge
          isAgentActive={isAgentActive}
          agentType={agentType}
          contextType={contextType}
          contextId={contextId}
          {...(hasActivity !== undefined ? { hasActivity } : {})}
          {...(agentStatus !== undefined ? { agentStatus } : {})}
          {...(storeKey !== undefined ? { storeKey } : {})}
          {...(modelDisplay !== undefined ? { modelDisplay } : {})}
          hideModelChip
        />
      </div>
    </div>
  );
}
