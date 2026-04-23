import { render, screen } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";

import { TooltipProvider } from "@/components/ui/tooltip";
import type { AgentRuntimeSelection } from "@/stores/agentSessionStore";
import type { AgentConversation } from "./agentConversations";
import { AgentsChatHeader } from "./AgentsView";

const runtime: AgentRuntimeSelection = {
  provider: "codex",
  modelId: "gpt-5.4",
};

const conversation = (
  overrides: Partial<AgentConversation> = {}
): AgentConversation => ({
  id: "conversation-1",
  contextType: "project",
  contextId: "project-1",
  claudeSessionId: null,
  providerSessionId: "thread-1",
  providerHarness: "codex",
  upstreamProvider: null,
  providerProfile: null,
  title: "Untitled agent",
  messageCount: 1,
  lastMessageAt: "2026-04-23T09:00:00Z",
  createdAt: "2026-04-23T09:00:00Z",
  updatedAt: "2026-04-23T09:00:00Z",
  archivedAt: null,
  projectId: "project-1",
  ideationSessionId: null,
  ...overrides,
});

describe("AgentsChatHeader", () => {
  it("opts the title button out of the high-contrast default button border", () => {
    render(
      <TooltipProvider>
        <AgentsChatHeader
          conversation={conversation()}
          runtime={runtime}
          artifactOpen={false}
          activeArtifactTab="plan"
          onRenameConversation={vi.fn().mockResolvedValue(undefined)}
          onToggleArtifacts={vi.fn()}
          onSelectArtifact={vi.fn()}
        />
      </TooltipProvider>
    );

    expect(screen.getByTestId("agents-chat-title-button")).toHaveAttribute(
      "data-theme-button-skip",
      "true"
    );
  });
});
