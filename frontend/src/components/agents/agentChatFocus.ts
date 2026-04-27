export type AgentsChatFocus =
  | { type: "workspace" }
  | { type: "ideation"; sessionId: string }
  | { type: "verification"; parentSessionId: string; childSessionId: string };

export type AgentsChatFocusTone = "accent" | "warning";

export interface AgentsChatFocusDisplay {
  type: Exclude<AgentsChatFocus["type"], "workspace">;
  label: string;
  description: string;
  tone: AgentsChatFocusTone;
}

export function getFocusedArtifactIdeationSessionId(
  chatFocus: AgentsChatFocus,
): string | null {
  if (chatFocus.type === "ideation") {
    return chatFocus.sessionId;
  }
  if (chatFocus.type === "verification") {
    return chatFocus.parentSessionId;
  }
  return null;
}

export function getAgentsChatFocusDisplay(
  chatFocus: AgentsChatFocus,
): AgentsChatFocusDisplay | null {
  if (chatFocus.type === "ideation") {
    return {
      type: "ideation",
      label: "Ideation",
      description: "Focused on an ideation run",
      tone: "accent",
    };
  }

  if (chatFocus.type === "verification") {
    return {
      type: "verification",
      label: "Verification",
      description: "Focused on a verification run",
      tone: "warning",
    };
  }

  return null;
}

export function getFocusedChatSessionId(chatFocus: AgentsChatFocus): string | null {
  if (chatFocus.type === "ideation") {
    return chatFocus.sessionId;
  }
  if (chatFocus.type === "verification") {
    return chatFocus.childSessionId;
  }
  return null;
}
