/**
 * chat-context-registry team extension tests — supportsTeamMode and teamActivityPanelPosition
 */

import { describe, it, expect } from "vitest";
import { CHAT_CONTEXT_REGISTRY, getContextConfig } from "./chat-context-registry";
import type { ContextType } from "@/types/chat-conversation";

describe("chat-context-registry — team extensions", () => {
  it("ideation supports team mode with right panel", () => {
    const config = getContextConfig("ideation");
    expect(config.supportsTeamMode).toBe(true);
    expect(config.teamActivityPanelPosition).toBe("right");
  });

  it("task_execution supports team mode with bottom panel", () => {
    const config = getContextConfig("task_execution");
    expect(config.supportsTeamMode).toBe(true);
    expect(config.teamActivityPanelPosition).toBe("bottom");
  });

  it("task does not support team mode", () => {
    const config = getContextConfig("task");
    expect(config.supportsTeamMode).toBe(false);
    expect(config.teamActivityPanelPosition).toBeNull();
  });

  it("project does not support team mode", () => {
    const config = getContextConfig("project");
    expect(config.supportsTeamMode).toBe(false);
    expect(config.teamActivityPanelPosition).toBeNull();
  });

  it("review does not support team mode", () => {
    const config = getContextConfig("review");
    expect(config.supportsTeamMode).toBe(false);
    expect(config.teamActivityPanelPosition).toBeNull();
  });

  it("merge does not support team mode", () => {
    const config = getContextConfig("merge");
    expect(config.supportsTeamMode).toBe(false);
    expect(config.teamActivityPanelPosition).toBeNull();
  });

  it("all contexts have supportsTeamMode and teamActivityPanelPosition defined", () => {
    const contextTypes: ContextType[] = ["ideation", "task", "project", "task_execution", "review", "merge"];
    for (const ct of contextTypes) {
      const config = CHAT_CONTEXT_REGISTRY[ct];
      expect(typeof config.supportsTeamMode).toBe("boolean");
      expect(
        config.teamActivityPanelPosition === null ||
        config.teamActivityPanelPosition === "right" ||
        config.teamActivityPanelPosition === "bottom",
      ).toBe(true);
    }
  });

  it("teamActivityPanelPosition is null when supportsTeamMode is false", () => {
    const contextTypes: ContextType[] = ["ideation", "task", "project", "task_execution", "review", "merge"];
    for (const ct of contextTypes) {
      const config = CHAT_CONTEXT_REGISTRY[ct];
      if (!config.supportsTeamMode) {
        expect(config.teamActivityPanelPosition).toBeNull();
      }
    }
  });
});
