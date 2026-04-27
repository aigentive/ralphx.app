import { memo, useCallback, useEffect, useMemo, useState, type ElementType } from "react";
import {
  ArrowLeft,
  CheckCircle2,
  ClipboardList,
  FileText,
  GitBranch,
  GitPullRequestArrow,
  Lightbulb,
  Loader2,
  PanelRightClose,
  PanelRightOpen,
  ShieldCheck,
  Terminal as TerminalIcon,
} from "lucide-react";

import type { AgentConversationWorkspace } from "@/api/chat";
import { ChatSessionChips } from "@/components/Chat/ChatSessionChips";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import {
  Tooltip,
  TooltipContent,
  TooltipTrigger,
} from "@/components/ui/tooltip";
import { formatBranchDisplay } from "@/lib/branch-utils";
import { withAlpha } from "@/lib/theme-colors";
import { cn } from "@/lib/utils";
import { useChatStore } from "@/stores/chatStore";
import type { AgentArtifactTab } from "@/stores/agentSessionStore";
import type { ModelDisplay } from "@/types/chat-conversation";

import {
  getAgentConversationStoreKey,
  type AgentConversation,
} from "./agentConversations";
import {
  getAgentsChatFocusDisplay,
  type AgentsChatFocus,
  type AgentsChatFocusDisplay,
  type AgentsChatFocusTone,
} from "./agentChatFocus";
import type { IdeationArtifactTab } from "./agentArtifactTabs";
import { resolveConversationAgentMode } from "./agentConversationMode";

const HEADER_ARTIFACT_TABS: Array<{
  id: IdeationArtifactTab;
  label: string;
  icon: ElementType;
}> = [
  { id: "plan", label: "Plan", icon: FileText },
  { id: "verification", label: "Verification", icon: CheckCircle2 },
  { id: "proposal", label: "Proposals", icon: GitPullRequestArrow },
  { id: "tasks", label: "Tasks", icon: ClipboardList },
];

const FOCUS_TONE_STYLES: Record<
  AgentsChatFocusTone,
  { color: string; background: string; border: string }
> = {
  accent: {
    color: "var(--accent-primary)",
    background: "var(--accent-muted)",
    border: "var(--accent-border)",
  },
  warning: {
    color: "var(--status-warning)",
    background: "var(--status-warning-muted)",
    border: "var(--status-warning-border)",
  },
};

const FOCUS_TONE_ICONS: Record<AgentsChatFocusTone, ElementType> = {
  accent: Lightbulb,
  warning: ShieldCheck,
};

export interface AgentsChatHeaderProps {
  conversation: AgentConversation | null;
  workspace: AgentConversationWorkspace | null;
  chatFocus?: AgentsChatFocus | undefined;
  modelDisplay?: ModelDisplay | undefined;
  availableArtifactTabs?: readonly IdeationArtifactTab[] | undefined;
  focusReturnLabel?: string | undefined;
  onReturnToWorkspaceChat?: (() => void) | undefined;
  artifactOpen: boolean;
  activeArtifactTab: AgentArtifactTab;
  terminalOpen?: boolean;
  terminalUnavailableReason?: string | null;
  onRenameConversation: (conversationId: string, title: string) => Promise<void>;
  onPublishWorkspace?: (conversationId: string) => Promise<void>;
  onOpenPublishPane?: () => void;
  onPreloadArtifacts?: () => void;
  publishShortcutLabel?: string;
  isPublishingWorkspace?: boolean;
  onToggleTerminal?: () => void;
  onPreloadTerminal?: () => void;
  onToggleArtifacts: () => void;
  onSelectArtifact: (tab: AgentArtifactTab) => void;
}

const AgentsChatFocusBadge = memo(function AgentsChatFocusBadge({
  focusDisplay,
}: {
  focusDisplay: AgentsChatFocusDisplay;
}) {
  const Icon = FOCUS_TONE_ICONS[focusDisplay.tone];
  const style = FOCUS_TONE_STYLES[focusDisplay.tone];

  return (
    <div
      className="inline-flex shrink-0 items-center gap-1.5 rounded-full border px-2 py-1 text-[11px] font-semibold"
      style={{
        color: style.color,
        background: style.background,
        borderColor: style.border,
      }}
      data-testid="agents-chat-focus-badge"
      data-focus-tone={focusDisplay.tone}
      aria-label={`Focused chat: ${focusDisplay.label}`}
    >
      <Icon className="h-3.5 w-3.5 shrink-0" />
      <span>{focusDisplay.label}</span>
    </div>
  );
});

export const AgentsChatHeader = memo(function AgentsChatHeader({
  conversation,
  workspace,
  chatFocus = { type: "workspace" },
  modelDisplay,
  availableArtifactTabs = [],
  focusReturnLabel,
  onReturnToWorkspaceChat,
  artifactOpen,
  activeArtifactTab,
  terminalOpen = false,
  terminalUnavailableReason = null,
  onRenameConversation,
  onPublishWorkspace,
  onOpenPublishPane,
  onPreloadArtifacts,
  publishShortcutLabel = "Commit & Publish",
  isPublishingWorkspace = false,
  onToggleTerminal,
  onPreloadTerminal,
  onToggleArtifacts,
  onSelectArtifact,
}: AgentsChatHeaderProps) {
  const title = conversation?.title || "Untitled agent";
  const conversationMode = conversation
    ? resolveConversationAgentMode(conversation, workspace)
    : null;
  const focusDisplay = getAgentsChatFocusDisplay(chatFocus);
  const visibleHeaderArtifactTabs = useMemo(
    () =>
      HEADER_ARTIFACT_TABS.filter((tab) =>
        availableArtifactTabs.includes(tab.id),
      ),
    [availableArtifactTabs],
  );
  const showIdeationArtifacts =
    conversationMode === "ideation" && visibleHeaderArtifactTabs.length > 0;
  const showArtifactToggle = conversationMode === "ideation" || artifactOpen;
  const publishPaneOpen = artifactOpen && activeArtifactTab === "publish";
  const showPublishShortcut = Boolean(
    conversation &&
      workspace?.mode === "edit" &&
      !workspace.linkedPlanBranchId &&
      !publishPaneOpen,
  );
  const [isEditing, setIsEditing] = useState(false);
  const [draftTitle, setDraftTitle] = useState(title);
  const conversationStoreKey = useMemo(
    () => (conversation ? getAgentConversationStoreKey(conversation) : null),
    [conversation],
  );
  const agentStatus = useChatStore((state) =>
    conversationStoreKey
      ? state.agentStatus[conversationStoreKey] ?? "idle"
      : "idle",
  );
  const isSending = useChatStore((state) =>
    conversationStoreKey ? state.isSending[conversationStoreKey] ?? false : false,
  );
  const isAgentActive = isSending || agentStatus === "generating";

  useEffect(() => {
    if (!isEditing) {
      setDraftTitle(title);
    }
  }, [isEditing, title]);

  const commitTitle = useCallback(async () => {
    if (!conversation) {
      setIsEditing(false);
      return;
    }
    const trimmed = draftTitle.trim();
    if (!trimmed || trimmed === title) {
      setDraftTitle(title);
      setIsEditing(false);
      return;
    }
    await onRenameConversation(conversation.id, trimmed);
    setIsEditing(false);
  }, [conversation, draftTitle, onRenameConversation, title]);

  return (
    <div
      className="flex w-full flex-1 items-center justify-between gap-3 min-w-0 overflow-hidden"
      data-testid="agents-chat-header"
      data-focus-type={chatFocus.type}
    >
      <div
        className={cn(
          "flex min-w-0 flex-1 items-center gap-2 overflow-hidden",
          focusDisplay && "rounded-md border-l-2 pl-2",
        )}
        style={
          focusDisplay
            ? { borderLeftColor: FOCUS_TONE_STYLES[focusDisplay.tone].color }
            : undefined
        }
      >
        {onReturnToWorkspaceChat ? (
          <Tooltip>
            <TooltipTrigger asChild>
              <Button
                type="button"
                variant="ghost"
                size="sm"
                className="h-7 gap-1.5 px-2 text-xs"
                onClick={onReturnToWorkspaceChat}
                data-testid="agents-chat-focus-return"
              >
                <ArrowLeft className="h-3.5 w-3.5" />
                <span>{focusReturnLabel ?? "Workspace chat"}</span>
              </Button>
            </TooltipTrigger>
            <TooltipContent side="bottom" className="text-xs">
              Return to workspace chat
            </TooltipContent>
          </Tooltip>
        ) : null}
        {focusDisplay ? <AgentsChatFocusBadge focusDisplay={focusDisplay} /> : null}
        <div className="min-w-0 flex-1">
          {isEditing ? (
            <Input
              value={draftTitle}
              onChange={(event) => setDraftTitle(event.target.value)}
              onBlur={() => void commitTitle()}
              onKeyDown={(event) => {
                if (event.key === "Enter") {
                  event.preventDefault();
                  void commitTitle();
                }
                if (event.key === "Escape") {
                  event.preventDefault();
                  setDraftTitle(title);
                  setIsEditing(false);
                }
              }}
              className="h-7 max-w-[260px] text-sm font-semibold"
              autoFocus
              aria-label="Agent title"
            />
          ) : (
            <button
              type="button"
              className="block w-full max-w-full text-left text-sm font-semibold truncate"
              style={{ color: "var(--text-primary)" }}
              onClick={() => conversation && setIsEditing(true)}
              aria-label="Edit agent title"
              data-testid="agents-chat-title-button"
              data-theme-button-skip="true"
            >
              {title}
            </button>
          )}
        </div>
        {workspace && !publishPaneOpen && <AgentsWorkspaceStatusPill workspace={workspace} />}
      </div>

      <div className="hidden md:flex items-center gap-1 ml-auto shrink-0">
        {conversation && (
          <ChatSessionChips
            contextType={conversation.contextType}
            contextId={conversation.contextId}
            isAgentActive={isAgentActive}
            conversationId={conversation.id}
            providerHarness={conversation.providerHarness ?? null}
            providerSessionId={conversation.providerSessionId ?? null}
            upstreamProvider={conversation.upstreamProvider ?? null}
            providerProfile={conversation.providerProfile ?? null}
            fallbackConversation={conversation}
            showProviderModel={false}
            showStats
            {...(modelDisplay !== undefined ? { modelDisplay } : {})}
          />
        )}

        <Tooltip>
          <TooltipTrigger asChild>
            <Button
              type="button"
              variant="ghost"
              size="sm"
              className="h-8 w-8 p-0"
              onClick={onToggleTerminal}
              onPointerEnter={onPreloadTerminal}
              onFocus={onPreloadTerminal}
              disabled={!onToggleTerminal || Boolean(terminalUnavailableReason)}
              aria-label={terminalOpen ? "Close terminal" : "Open terminal"}
              data-testid="agents-terminal-toggle"
            >
              <TerminalIcon className="w-4 h-4" />
            </Button>
          </TooltipTrigger>
          <TooltipContent side="bottom" className="max-w-[280px] text-xs">
            {terminalUnavailableReason ??
              (terminalOpen ? "Close terminal" : "Open terminal")}
          </TooltipContent>
        </Tooltip>

        {showPublishShortcut && (
          <Tooltip>
            <TooltipTrigger asChild>
              <Button
                type="button"
                variant="ghost"
                size="sm"
                className="h-8 gap-1.5 px-2.5 text-xs"
                onClick={onOpenPublishPane}
                onPointerEnter={onPreloadArtifacts}
                onFocus={onPreloadArtifacts}
                disabled={
                  !onPublishWorkspace ||
                  !onOpenPublishPane ||
                  isPublishingWorkspace ||
                  workspace?.status === "missing"
                }
                aria-label={`Open workspace publish panel: ${publishShortcutLabel}`}
                data-testid="agents-publish-workspace"
              >
                {isPublishingWorkspace ? (
                  <Loader2 className="h-3.5 w-3.5 animate-spin" />
                ) : (
                  <GitPullRequestArrow className="h-3.5 w-3.5" />
                )}
                <span>{publishShortcutLabel}</span>
              </Button>
            </TooltipTrigger>
            <TooltipContent side="bottom" className="text-xs">
              Open the workspace publish panel
            </TooltipContent>
          </Tooltip>
        )}

        {showIdeationArtifacts && !artifactOpen &&
          visibleHeaderArtifactTabs.map(({ id, label, icon: Icon }) => {
            const isActive = activeArtifactTab === id && artifactOpen;
            return (
              <Tooltip key={id}>
                <TooltipTrigger asChild>
                  <Button
                    type="button"
                    variant="ghost"
                    size="sm"
                    className={cn("h-8 w-8 p-0", isActive ? "" : "opacity-80")}
                    onClick={() => onSelectArtifact(id)}
                    onPointerEnter={onPreloadArtifacts}
                    onFocus={onPreloadArtifacts}
                    style={{
                      color: isActive ? "var(--accent-primary)" : "var(--text-muted)",
                      background: isActive ? withAlpha("var(--accent-primary)", 12) : "transparent",
                      border: isActive
                        ? "1px solid var(--accent-border)"
                        : "1px solid var(--overlay-faint)",
                      boxShadow: "none",
                    }}
                    aria-label={label}
                  >
                    <Icon className="w-4 h-4" />
                  </Button>
                </TooltipTrigger>
                <TooltipContent side="bottom" className="text-xs">
                  {label}
                </TooltipContent>
              </Tooltip>
            );
          })}

        {showArtifactToggle ? (
          <Tooltip>
            <TooltipTrigger asChild>
              <Button
                type="button"
                variant="ghost"
                size="sm"
                className="h-8 w-8 p-0"
                onClick={onToggleArtifacts}
                onPointerEnter={onPreloadArtifacts}
                onFocus={onPreloadArtifacts}
                aria-label={artifactOpen ? "Close panel" : "Open artifacts"}
              >
                {artifactOpen ? (
                  <PanelRightClose className="w-4 h-4" />
                ) : (
                  <PanelRightOpen className="w-4 h-4" />
                )}
              </Button>
            </TooltipTrigger>
            <TooltipContent side="bottom" className="text-xs">
              {artifactOpen ? "Close panel" : "Open artifacts"}
            </TooltipContent>
          </Tooltip>
        ) : null}
      </div>
    </div>
  );
});

const AgentsWorkspaceStatusPill = memo(function AgentsWorkspaceStatusPill({
  workspace,
}: {
  workspace: AgentConversationWorkspace;
}) {
  const branch = formatBranchDisplay(workspace.branchName);
  const status =
    workspace.publicationPrStatus ?? workspace.publicationPushStatus ?? workspace.status;
  const statusLabel = status.replace(/_/g, " ");
  const baseLabel = workspace.baseDisplayName ?? workspace.baseRef;

  return (
    <Tooltip>
      <TooltipTrigger asChild>
        <div
          tabIndex={0}
          className="inline-flex min-w-0 max-w-[180px] items-center gap-1.5 rounded-full border px-2.5 py-1 text-[11px] font-medium sm:max-w-[300px]"
          style={{
            color: "var(--text-secondary)",
            background: "var(--bg-surface)",
            borderColor: "var(--overlay-weak)",
          }}
          data-testid="agents-workspace-status"
        >
          <GitBranch className="h-3.5 w-3.5 shrink-0" />
          <span className="truncate font-mono">{branch.short}</span>
          <span
            className="h-1 w-1 shrink-0 rounded-full"
            style={{ background: "var(--accent-primary)" }}
          />
          <span className="shrink-0 capitalize">{statusLabel}</span>
        </div>
      </TooltipTrigger>
      <TooltipContent side="bottom" className="max-w-[360px] text-xs">
        <div className="space-y-1">
          <div>Branch: {branch.full}</div>
          <div>Base: {baseLabel}</div>
          {workspace.publicationPrUrl && (
            <div>
              PR:{" "}
              {workspace.publicationPrNumber
                ? `#${workspace.publicationPrNumber}`
                : workspace.publicationPrUrl}
            </div>
          )}
        </div>
      </TooltipContent>
    </Tooltip>
  );
});
