import { describe, expect, it } from "vitest";

import {
  AgentHarnessAvailabilityResponseSchema,
  AgentLaneSettingsResponseSchema,
  mergeAgentHarnessState,
} from "./ideation-harness";

describe("ideation harness api contracts", () => {
  it("accepts provider-neutral harness strings in lane settings responses", () => {
    const parsed = AgentLaneSettingsResponseSchema.parse({
      projectId: null,
      lane: "ideation_primary",
      harness: "openai-cli",
      model: null,
      effort: null,
      approvalPolicy: null,
      sandboxMode: null,
      fallbackHarness: "claude",
      updatedAt: new Date().toISOString(),
    });

    expect(parsed.harness).toBe("openai-cli");
    expect(parsed.fallbackHarness).toBe("claude");
  });

  it("merges unknown harness availability without collapsing it to claude or codex", () => {
    const rows = [
      AgentLaneSettingsResponseSchema.parse({
        projectId: null,
        lane: "execution_worker",
        harness: "openai-cli",
        model: "gpt-5.5",
        effort: "high",
        approvalPolicy: "on-request",
        sandboxMode: "workspace-write",
        fallbackHarness: "claude",
        updatedAt: new Date().toISOString(),
      }),
    ];
    const availability = [
      AgentHarnessAvailabilityResponseSchema.parse({
        projectId: null,
        lane: "execution_worker",
        configuredHarness: "openai-cli",
        fallbackHarness: "claude",
        effectiveHarness: "openai-cli",
        fallbackActivated: false,
        binaryPath: "/usr/local/bin/openai-cli",
        binaryFound: true,
        probeSucceeded: true,
        available: true,
        missingCoreExecFeatures: [],
        error: null,
      }),
    ];

    const merged = mergeAgentHarnessState(rows, availability);
    const worker = merged.find((entry) => entry.lane === "execution_worker");

    expect(worker?.configuredHarness).toBe("openai-cli");
    expect(worker?.effectiveHarness).toBe("openai-cli");
    expect(worker?.fallbackHarness).toBe("claude");
  });
});
