import { lazy, Suspense, useCallback } from "react";
import { createPortal } from "react-dom";
import { Terminal as TerminalIcon } from "lucide-react";

import type { AgentConversationWorkspace } from "@/api/chat";
import type { AgentArtifactTab } from "@/stores/agentSessionStore";

import { useResolvedAgentArtifactState } from "./agentArtifactState";
import { useAfterPaintMounted } from "./agentDeferredFrame";
import { preloadAgentTerminalDrawer } from "./agentTerminalPreload";
import {
  AGENT_TERMINAL_DEFAULT_HEIGHT,
  useAgentTerminalStore,
  type AgentTerminalPlacement,
} from "./agentTerminalStore";

const LazyAgentTerminalDrawer = lazy(() =>
  preloadAgentTerminalDrawer().then((module) => ({ default: module.AgentTerminalDrawer })),
);

function AgentTerminalLoadingShell({
  height,
  dockElement,
}: {
  height: number;
  dockElement: HTMLElement | null;
}) {
  const shell = (
    <div
      className="relative shrink-0 overflow-hidden border-t"
      style={{
        height,
        background: "var(--bg-base)",
        borderColor: "var(--overlay-weak)",
        boxShadow: "0 -16px 36px var(--shadow-card)",
      }}
      data-testid="agent-terminal-loading-shell"
    >
      <div
        className="flex h-9 items-center gap-2 border-b px-3 text-xs"
        style={{
          background: "var(--bg-surface)",
          borderColor: "var(--overlay-faint)",
          color: "var(--text-secondary)",
        }}
      >
        <TerminalIcon
          className="h-3.5 w-3.5 shrink-0"
          style={{ color: "var(--accent-primary)" }}
        />
        <span className="font-medium" style={{ color: "var(--text-primary)" }}>
          Terminal
        </span>
        <span className="h-1 w-1 rounded-full" style={{ background: "var(--text-muted)" }} />
        <span>Opening</span>
      </div>
      <div className="px-3 py-2 font-mono text-xs" style={{ color: "var(--text-muted)" }}>
        Starting terminal...
      </div>
    </div>
  );

  return dockElement ? createPortal(shell, dockElement) : shell;
}

interface AgentsTerminalPresentationInput {
  conversationId: string | null;
  workspace: AgentConversationWorkspace | null;
  terminalUnavailableReason: string | null;
  hasAutoOpenArtifacts: boolean;
}

function useAgentTerminalPresentation({
  conversationId,
  workspace,
  terminalUnavailableReason,
  hasAutoOpenArtifacts,
}: AgentsTerminalPresentationInput) {
  const isOpen = useAgentTerminalStore((state) =>
    conversationId ? state.openByConversationId[conversationId] ?? false : false,
  );
  const height = useAgentTerminalStore((state) =>
    conversationId
      ? state.heightByConversationId[conversationId] ?? AGENT_TERMINAL_DEFAULT_HEIGHT
      : AGENT_TERMINAL_DEFAULT_HEIGHT,
  );
  const placement = useAgentTerminalStore((state) => state.placement);
  const { artifactPaneOpen } = useResolvedAgentArtifactState(
    conversationId,
    hasAutoOpenArtifacts,
  );
  const canRender = Boolean(conversationId && workspace && !terminalUnavailableReason);
  const dockTarget =
    artifactPaneOpen && (placement === "panel" || placement === "auto")
      ? "panel"
      : "chat";

  return {
    canRender,
    isOpen,
    height,
    placement,
    dockTarget,
    artifactPaneOpen,
  };
}

interface AgentsTerminalDockHostProps extends AgentsTerminalPresentationInput {
  dock: "chat" | "panel";
  setDockElement: (element: HTMLDivElement | null) => void;
}

export function AgentsTerminalDockHost({
  dock,
  conversationId,
  workspace,
  terminalUnavailableReason,
  hasAutoOpenArtifacts,
  setDockElement,
}: AgentsTerminalDockHostProps) {
  const { canRender, isOpen, height, dockTarget } = useAgentTerminalPresentation({
    conversationId,
    workspace,
    terminalUnavailableReason,
    hasAutoOpenArtifacts,
  });

  if (!canRender) {
    return null;
  }

  const isVisible = isOpen && dockTarget === dock;

  return (
    <div
      ref={setDockElement}
      className="shrink-0 overflow-hidden"
      style={{
        height: isVisible ? height : 0,
        opacity: isVisible ? 1 : 0,
        pointerEvents: isVisible ? "auto" : "none",
        transition: "none",
      }}
      data-testid={dock === "panel" ? "agent-terminal-host-panel" : "agent-terminal-host-chat"}
    />
  );
}

interface AgentsTerminalRegionProps extends AgentsTerminalPresentationInput {
  chatDockElement: HTMLElement | null;
  panelDockElement: HTMLElement | null;
  onOpenArtifactTab: (conversationId: string, tab: AgentArtifactTab) => void;
}

export function AgentsTerminalRegion({
  conversationId,
  workspace,
  terminalUnavailableReason,
  hasAutoOpenArtifacts,
  chatDockElement,
  panelDockElement,
  onOpenArtifactTab,
}: AgentsTerminalRegionProps) {
  const {
    canRender,
    isOpen,
    height,
    placement,
    dockTarget,
    artifactPaneOpen,
  } = useAgentTerminalPresentation({
    conversationId,
    workspace,
    terminalUnavailableReason,
    hasAutoOpenArtifacts,
  });
  const contentMounted = useAfterPaintMounted(canRender && isOpen);
  const setTerminalHeight = useAgentTerminalStore((state) => state.setHeight);
  const setTerminalOpen = useAgentTerminalStore((state) => state.setOpen);
  const setTerminalPlacement = useAgentTerminalStore((state) => state.setPlacement);

  const handlePlacementChange = useCallback(
    (nextPlacement: AgentTerminalPlacement) => {
      setTerminalPlacement(nextPlacement);
      if (nextPlacement === "panel" && conversationId && !artifactPaneOpen) {
        onOpenArtifactTab(conversationId, "publish");
      }
    },
    [artifactPaneOpen, conversationId, onOpenArtifactTab, setTerminalPlacement],
  );

  if (!canRender || !conversationId || !workspace) {
    return null;
  }

  const dockElement = dockTarget === "panel" ? panelDockElement : chatDockElement;

  if (isOpen && !contentMounted) {
    return <AgentTerminalLoadingShell height={height} dockElement={dockElement} />;
  }

  if (!contentMounted) {
    return null;
  }

  return (
    <Suspense
      fallback={
        <AgentTerminalLoadingShell
          height={height}
          dockElement={dockElement}
        />
      }
    >
      <LazyAgentTerminalDrawer
        conversationId={conversationId}
        workspace={workspace}
        height={height}
        onHeightChange={(nextHeight) => setTerminalHeight(conversationId, nextHeight)}
        onClose={() => setTerminalOpen(conversationId, false)}
        placement={placement}
        onPlacementChange={handlePlacementChange}
        dockElement={dockElement}
      />
    </Suspense>
  );
}
