import { describe, it, expect } from "vitest";
import type { z } from "zod";
import { IdeationSessionResponseSchema } from "./ideation.schemas";
import { transformNullableBool, transformSession } from "./ideation.transforms";

type RawSession = z.infer<typeof IdeationSessionResponseSchema>;

const baseRaw: RawSession = {
  id: "sess-1",
  project_id: "proj-1",
  title: null,
  status: "active",
  plan_artifact_id: null,
  parent_session_id: null,
  created_at: "2026-01-01T00:00:00Z",
  updated_at: "2026-01-01T00:00:00Z",
  archived_at: null,
  converted_at: null,
};

describe("transformNullableBool", () => {
  it("returns null for null input", () => {
    expect(transformNullableBool(null)).toBeNull();
  });

  it("returns null for undefined input", () => {
    expect(transformNullableBool(undefined)).toBeNull();
  });

  it("returns false for 0", () => {
    expect(transformNullableBool(0)).toBe(false);
  });

  it("returns true for 1", () => {
    expect(transformNullableBool(1)).toBe(true);
  });

  it("returns true for any non-zero number", () => {
    expect(transformNullableBool(2)).toBe(true);
    expect(transformNullableBool(-1)).toBe(true);
  });
});

describe("transformSession — lastEffectiveModel", () => {
  it("maps last_effective_model string to lastEffectiveModel", () => {
    const result = transformSession({ ...baseRaw, last_effective_model: "claude-sonnet-4-6" });
    expect(result.lastEffectiveModel).toBe("claude-sonnet-4-6");
  });

  it("returns null when last_effective_model is absent", () => {
    const result = transformSession({ ...baseRaw });
    expect(result.lastEffectiveModel).toBeNull();
  });

  it("returns null when last_effective_model is null", () => {
    const result = transformSession({ ...baseRaw, last_effective_model: null });
    expect(result.lastEffectiveModel).toBeNull();
  });
});
