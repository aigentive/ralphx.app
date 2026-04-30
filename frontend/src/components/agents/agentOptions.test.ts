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
      } as never),
    ).toEqual(DEFAULT_AGENT_RUNTIME);
  });

  it("keeps a valid provider and falls back to that provider's default model", () => {
    expect(
      normalizeRuntimeSelection({
        provider: "claude",
        modelId: "retired-model",
      }),
    ).toEqual({
      provider: "claude",
      modelId: "sonnet",
    });
  });
});
