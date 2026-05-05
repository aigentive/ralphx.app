import { describe, expect, it } from "vitest";

import {
  DEFAULT_AGENT_RUNTIME,
  normalizeRuntimeSelection,
} from "./agentOptions";

describe("agentOptions", () => {
  it("falls back to the default runtime when the remembered provider is unknown", () => {
    expect(
      normalizeRuntimeSelection({
        provider: "removed-provider",
        modelId: "retired-model",
        effort: "high",
      } as never),
    ).toEqual(DEFAULT_AGENT_RUNTIME);
  });

  it("keeps a valid provider and falls back to that provider's default model", () => {
    expect(
      normalizeRuntimeSelection({
        provider: "claude",
        modelId: "retired-model",
        effort: "high",
      }),
    ).toEqual({
      provider: "claude",
      modelId: "sonnet",
      effort: "medium",
    });
  });

  it("keeps a valid provider/model and falls back to that model's default effort", () => {
    expect(
      normalizeRuntimeSelection({
        provider: "codex",
        modelId: "gpt-5.4-mini",
        effort: "retired-effort",
      }),
    ).toEqual({
      provider: "codex",
      modelId: "gpt-5.4-mini",
      effort: "medium",
    });
  });
});
