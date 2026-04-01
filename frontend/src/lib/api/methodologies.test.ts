import { describe, it, expect, vi, beforeEach } from "vitest";
import { invoke } from "@tauri-apps/api/core";
import {
  getMethodologies,
  getActiveMethodology,
  activateMethodology,
  deactivateMethodology,
  MethodologyResponseSchema,
  MethodologyPhaseResponseSchema,
  MethodologyTemplateResponseSchema,
  MethodologyActivationResponseSchema,
} from "./methodologies";

// Mock Tauri invoke
vi.mock("@tauri-apps/api/core", () => ({
  invoke: vi.fn(),
}));

const mockInvoke = invoke as ReturnType<typeof vi.fn>;

// Test data helpers
const createMockPhase = (overrides = {}) => ({
  id: "phase-1",
  name: "Planning",
  order: 1,
  description: "Initial planning phase",
  agent_profiles: ["analyst", "architect"],
  column_ids: ["backlog", "planning"],
  ...overrides,
});

const createMockTemplate = (overrides = {}) => ({
  artifact_type: "prd",
  template_path: "templates/prd.md",
  name: "PRD Template",
  description: "Template for creating PRDs",
  ...overrides,
});

const createMockMethodology = (overrides = {}) => ({
  id: "bmad-method",
  name: "BMAD Method",
  description: "Business Model Aligned Development",
  agent_profiles: ["analyst", "architect", "developer", "reviewer"],
  skills: ["analysis", "design", "implementation", "review"],
  workflow_id: "bmad-workflow",
  workflow_name: "BMAD Workflow",
  phases: [
    createMockPhase({ id: "planning", name: "Planning", order: 1 }),
    createMockPhase({ id: "building", name: "Building", order: 2 }),
  ],
  templates: [createMockTemplate()],
  is_active: false,
  phase_count: 2,
  agent_count: 4,
  created_at: "2026-01-24T12:00:00Z",
  ...overrides,
});

const createMockActivationResponse = (overrides = {}) => ({
  methodology: createMockMethodology({ is_active: true }),
  workflow: {
    id: "bmad-workflow",
    name: "BMAD Workflow",
    description: "Workflow for BMAD method",
    column_count: 6,
  },
  agent_profiles: ["analyst", "architect", "developer", "reviewer"],
  skills: ["analysis", "design", "implementation", "review"],
  previous_methodology_id: null,
  ...overrides,
});

describe("MethodologyPhaseResponseSchema", () => {
  it("should parse valid phase response", () => {
    const phase = createMockPhase();
    expect(() => MethodologyPhaseResponseSchema.parse(phase)).not.toThrow();
  });

  it("should parse phase with null description", () => {
    const phase = createMockPhase({ description: null });
    const result = MethodologyPhaseResponseSchema.parse(phase);
    expect(result.description).toBeNull();
  });

  it("should parse phase with empty arrays", () => {
    const phase = createMockPhase({ agent_profiles: [], column_ids: [] });
    const result = MethodologyPhaseResponseSchema.parse(phase);
    expect(result.agent_profiles).toEqual([]);
    expect(result.column_ids).toEqual([]);
  });

  it("should reject phase without required fields", () => {
    expect(() => MethodologyPhaseResponseSchema.parse({})).toThrow();
    expect(() => MethodologyPhaseResponseSchema.parse({ id: "p1" })).toThrow();
  });
});

describe("MethodologyTemplateResponseSchema", () => {
  it("should parse valid template response", () => {
    const template = createMockTemplate();
    expect(() => MethodologyTemplateResponseSchema.parse(template)).not.toThrow();
  });

  it("should parse template with null optional fields", () => {
    const template = createMockTemplate({ name: null, description: null });
    const result = MethodologyTemplateResponseSchema.parse(template);
    expect(result.name).toBeNull();
    expect(result.description).toBeNull();
  });

  it("should reject template without required fields", () => {
    expect(() => MethodologyTemplateResponseSchema.parse({})).toThrow();
  });
});

describe("MethodologyResponseSchema", () => {
  it("should parse valid methodology response", () => {
    const methodology = createMockMethodology();
    expect(() => MethodologyResponseSchema.parse(methodology)).not.toThrow();
  });

  it("should parse methodology without description", () => {
    const methodology = createMockMethodology({ description: null });
    const result = MethodologyResponseSchema.parse(methodology);
    expect(result.description).toBeNull();
  });

  it("should parse methodology with empty phases and templates", () => {
    const methodology = createMockMethodology({
      phases: [],
      templates: [],
      phase_count: 0,
    });
    const result = MethodologyResponseSchema.parse(methodology);
    expect(result.phases).toEqual([]);
    expect(result.templates).toEqual([]);
  });

  it("should parse active methodology", () => {
    const methodology = createMockMethodology({ is_active: true });
    const result = MethodologyResponseSchema.parse(methodology);
    expect(result.is_active).toBe(true);
  });

  it("should reject methodology without required fields", () => {
    expect(() => MethodologyResponseSchema.parse({})).toThrow();
    expect(() => MethodologyResponseSchema.parse({ id: "m1" })).toThrow();
  });

  it("should validate phases array", () => {
    const methodology = createMockMethodology({
      phases: [{ invalid: "phase" }],
    });
    expect(() => MethodologyResponseSchema.parse(methodology)).toThrow();
  });
});

describe("MethodologyActivationResponseSchema", () => {
  it("should parse valid activation response", () => {
    const response = createMockActivationResponse();
    expect(() => MethodologyActivationResponseSchema.parse(response)).not.toThrow();
  });

  it("should parse activation with previous methodology", () => {
    const response = createMockActivationResponse({
      previous_methodology_id: "old-method",
    });
    const result = MethodologyActivationResponseSchema.parse(response);
    expect(result.previous_methodology_id).toBe("old-method");
  });

  it("should parse activation without previous methodology", () => {
    const response = createMockActivationResponse({ previous_methodology_id: null });
    const result = MethodologyActivationResponseSchema.parse(response);
    expect(result.previous_methodology_id).toBeNull();
  });

  it("should validate nested methodology", () => {
    const response = createMockActivationResponse();
    const result = MethodologyActivationResponseSchema.parse(response);
    expect(result.methodology.is_active).toBe(true);
  });

  it("should validate nested workflow", () => {
    const response = createMockActivationResponse();
    const result = MethodologyActivationResponseSchema.parse(response);
    expect(result.workflow.column_count).toBe(6);
  });
});

describe("getMethodologies", () => {
  beforeEach(() => {
    mockInvoke.mockReset();
  });

  it("should call get_methodologies command", async () => {
    mockInvoke.mockResolvedValue([createMockMethodology()]);

    await getMethodologies();

    expect(mockInvoke).toHaveBeenCalledWith("get_methodologies", {});
  });

  it("should return validated array of methodologies", async () => {
    const methodologies = [
      createMockMethodology({ id: "m1", name: "Method 1" }),
      createMockMethodology({ id: "m2", name: "Method 2" }),
    ];
    mockInvoke.mockResolvedValue(methodologies);

    const result = await getMethodologies();

    expect(result).toHaveLength(2);
    expect(result[0]?.name).toBe("Method 1");
    expect(result[1]?.name).toBe("Method 2");
  });

  it("should return empty array when no methodologies", async () => {
    mockInvoke.mockResolvedValue([]);

    const result = await getMethodologies();

    expect(result).toEqual([]);
  });

  it("should throw on invalid response", async () => {
    mockInvoke.mockResolvedValue([{ invalid: "methodology" }]);

    await expect(getMethodologies()).rejects.toThrow();
  });

  it("should propagate backend errors", async () => {
    mockInvoke.mockRejectedValue(new Error("Database error"));

    await expect(getMethodologies()).rejects.toThrow("Database error");
  });
});

describe("getActiveMethodology", () => {
  beforeEach(() => {
    mockInvoke.mockReset();
  });

  it("should call get_active_methodology command", async () => {
    mockInvoke.mockResolvedValue(null);

    await getActiveMethodology();

    expect(mockInvoke).toHaveBeenCalledWith("get_active_methodology", {});
  });

  it("should return null when no active methodology", async () => {
    mockInvoke.mockResolvedValue(null);

    const result = await getActiveMethodology();

    expect(result).toBeNull();
  });

  it("should return active methodology when exists", async () => {
    const active = createMockMethodology({ is_active: true });
    mockInvoke.mockResolvedValue(active);

    const result = await getActiveMethodology();

    expect(result?.is_active).toBe(true);
    expect(result?.name).toBe("BMAD Method");
  });

  it("should throw on invalid response", async () => {
    mockInvoke.mockResolvedValue({ invalid: "methodology" });

    await expect(getActiveMethodology()).rejects.toThrow();
  });
});

describe("activateMethodology", () => {
  beforeEach(() => {
    mockInvoke.mockReset();
  });

  it("should call activate_methodology command with id", async () => {
    mockInvoke.mockResolvedValue(createMockActivationResponse());

    await activateMethodology("bmad-method");

    expect(mockInvoke).toHaveBeenCalledWith("activate_methodology", { id: "bmad-method" });
  });

  it("should return activation response", async () => {
    const response = createMockActivationResponse();
    mockInvoke.mockResolvedValue(response);

    const result = await activateMethodology("bmad-method");

    expect(result.methodology.is_active).toBe(true);
    expect(result.agent_profiles).toContain("analyst");
    expect(result.workflow.column_count).toBe(6);
  });

  it("should include previous methodology id when switching", async () => {
    const response = createMockActivationResponse({
      previous_methodology_id: "gsd-method",
    });
    mockInvoke.mockResolvedValue(response);

    const result = await activateMethodology("bmad-method");

    expect(result.previous_methodology_id).toBe("gsd-method");
  });

  it("should propagate already active error", async () => {
    mockInvoke.mockRejectedValue(new Error("Methodology 'BMAD Method' is already active"));

    await expect(activateMethodology("bmad-method")).rejects.toThrow(
      "already active"
    );
  });

  it("should propagate not found error", async () => {
    mockInvoke.mockRejectedValue(new Error("Methodology not found: nonexistent"));

    await expect(activateMethodology("nonexistent")).rejects.toThrow(
      "Methodology not found"
    );
  });
});

describe("deactivateMethodology", () => {
  beforeEach(() => {
    mockInvoke.mockReset();
  });

  it("should call deactivate_methodology command with id", async () => {
    mockInvoke.mockResolvedValue(createMockMethodology({ is_active: false }));

    await deactivateMethodology("bmad-method");

    expect(mockInvoke).toHaveBeenCalledWith("deactivate_methodology", {
      id: "bmad-method",
    });
  });

  it("should return deactivated methodology", async () => {
    const deactivated = createMockMethodology({ is_active: false });
    mockInvoke.mockResolvedValue(deactivated);

    const result = await deactivateMethodology("bmad-method");

    expect(result.is_active).toBe(false);
  });

  it("should propagate not active error", async () => {
    mockInvoke.mockRejectedValue(new Error("Methodology 'BMAD Method' is not active"));

    await expect(deactivateMethodology("bmad-method")).rejects.toThrow(
      "is not active"
    );
  });

  it("should propagate not found error", async () => {
    mockInvoke.mockRejectedValue(new Error("Methodology not found: nonexistent"));

    await expect(deactivateMethodology("nonexistent")).rejects.toThrow(
      "Methodology not found"
    );
  });
});
