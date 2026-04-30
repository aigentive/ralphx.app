import { describe, expect, it } from "vitest";

import { deriveAgentTitleFromMessages, isDefaultAgentTitle } from "./agentTitle";

describe("agentTitle", () => {
  it("derives an imperative title from the first user message", () => {
    expect(deriveAgentTitleFromMessages(["I want to build a task board"])).toBe(
      "Build a task board"
    );
  });

  it("uses the best of the first few short messages", () => {
    expect(deriveAgentTitleFromMessages(["wus", "w", "good I like jokes"])).toBe(
      "Discuss good I like jokes"
    );
  });

  it("preserves work item identifiers", () => {
    expect(deriveAgentTitleFromMessages(["PDM-301 please fix token refresh"])).toBe(
      "PDM-301: Fix token refresh"
    );
  });

  it("detects default titles", () => {
    expect(isDefaultAgentTitle(null)).toBe(true);
    expect(isDefaultAgentTitle("Untitled agent")).toBe(true);
    expect(isDefaultAgentTitle("Build sidebar")).toBe(false);
  });
});
