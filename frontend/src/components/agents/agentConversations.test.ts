import { describe, expect, it } from "vitest";

import type { IdeationSessionResponse } from "@/api/ideation";
import type { ChatConversation } from "@/types/chat-conversation";
import {
  sortAgentConversations,
  toIdeationAgentConversation,
  toProjectAgentConversation,
} from "./agentConversations";

const conversation = (
  overrides: Partial<ChatConversation> = {}
): ChatConversation => ({
  id: "conversation-1",
  contextType: "project",
  contextId: "project-1",
  claudeSessionId: null,
  providerSessionId: null,
  providerHarness: null,
  upstreamProvider: null,
  providerProfile: null,
  title: null,
  messageCount: 0,
  lastMessageAt: null,
  createdAt: "2026-04-22T10:00:00Z",
  updatedAt: "2026-04-22T10:00:00Z",
  archivedAt: null,
  ...overrides,
});

const session = (
  overrides: Partial<IdeationSessionResponse> = {}
): IdeationSessionResponse => ({
  id: "session-1",
  projectId: "project-1",
  title: "Ideation title",
  titleSource: "user",
  status: "active",
  planArtifactId: null,
  seedTaskId: null,
  parentSessionId: null,
  teamMode: "solo",
  teamConfig: null,
  createdAt: "2026-04-22T10:00:00Z",
  updatedAt: "2026-04-22T10:05:00Z",
  archivedAt: null,
  convertedAt: null,
  verificationStatus: "unverified",
  verificationInProgress: false,
  gapScore: null,
  sessionPurpose: "general",
  acceptanceStatus: null,
  ...overrides,
});

describe("agent conversations", () => {
  it("keeps legacy project conversations grouped by project", () => {
    const result = toProjectAgentConversation(conversation({ title: "Project chat" }));

    expect(result.projectId).toBe("project-1");
    expect(result.ideationSessionId).toBeNull();
    expect(result.title).toBe("Project chat");
  });

  it("projects an ideation session conversation into an agent sidebar row", () => {
    const result = toIdeationAgentConversation(
      session({ title: "Fix flaky tests" }),
      conversation({ id: "conversation-2", contextType: "ideation", contextId: "session-1" })
    );

    expect(result.contextType).toBe("ideation");
    expect(result.contextId).toBe("session-1");
    expect(result.projectId).toBe("project-1");
    expect(result.ideationSessionId).toBe("session-1");
    expect(result.title).toBe("Fix flaky tests");
    expect(result.updatedAt).toBe("2026-04-22T10:05:00Z");
  });

  it("marks archived ideation sessions archived even when the chat row is active", () => {
    const result = toIdeationAgentConversation(
      session({
        status: "archived",
        archivedAt: "2026-04-22T11:00:00Z",
        updatedAt: "2026-04-22T11:00:00Z",
      }),
      conversation({ contextType: "ideation", contextId: "session-1" })
    );

    expect(result.archivedAt).toBe("2026-04-22T11:00:00Z");
  });

  it("sorts by creation time newest first", () => {
    const older = toProjectAgentConversation(
      conversation({
        id: "older",
        createdAt: "2026-04-22T10:00:00Z",
        lastMessageAt: "2026-04-22T12:00:00Z",
      })
    );
    const newer = toProjectAgentConversation(
      conversation({
        id: "newer",
        createdAt: "2026-04-22T11:00:00Z",
        lastMessageAt: "2026-04-22T11:01:00Z",
      })
    );

    expect(sortAgentConversations([older, newer]).map((item) => item.id)).toEqual([
      "newer",
      "older",
    ]);
  });
});
