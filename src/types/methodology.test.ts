import { describe, it, expect } from "vitest";
import {
  // Schemas
  MethodologyStatusSchema,
  MethodologyPhaseSchema,
  MethodologyTemplateSchema,
  MethodologyExtensionSchema,
  CreateMethodologyExtensionInputSchema,
  // Constants
  METHODOLOGY_STATUS_VALUES,
  BUILTIN_METHODOLOGIES,
  BMAD_METHODOLOGY,
  GSD_METHODOLOGY,
  // Functions
  isMethodologyActive,
  isMethodologyAvailable,
  isMethodologyDisabled,
  getBuiltinMethodology,
  parseMethodologyExtension,
  safeParseMethodologyExtension,
  parseMethodologyPhase,
  safeParseMethodologyPhase,
  // Types
  type MethodologyStatus,
  type MethodologyPhase,
  type MethodologyTemplate,
  type MethodologyExtension,
} from "./methodology";

// ============================================
// MethodologyStatus Tests
// ============================================

describe("MethodologyStatusSchema", () => {
  it("should validate all status values", () => {
    expect(MethodologyStatusSchema.parse("available")).toBe("available");
    expect(MethodologyStatusSchema.parse("active")).toBe("active");
    expect(MethodologyStatusSchema.parse("disabled")).toBe("disabled");
  });

  it("should reject invalid values", () => {
    expect(() => MethodologyStatusSchema.parse("invalid")).toThrow();
    expect(() => MethodologyStatusSchema.parse("")).toThrow();
    expect(() => MethodologyStatusSchema.parse(123)).toThrow();
  });
});

describe("METHODOLOGY_STATUS_VALUES", () => {
  it("should have 3 statuses", () => {
    expect(METHODOLOGY_STATUS_VALUES).toHaveLength(3);
  });

  it("should include all statuses", () => {
    expect(METHODOLOGY_STATUS_VALUES).toContain("available");
    expect(METHODOLOGY_STATUS_VALUES).toContain("active");
    expect(METHODOLOGY_STATUS_VALUES).toContain("disabled");
  });
});

describe("isMethodologyActive", () => {
  it("should return true for active", () => {
    expect(isMethodologyActive("active")).toBe(true);
  });

  it("should return false for other statuses", () => {
    expect(isMethodologyActive("available")).toBe(false);
    expect(isMethodologyActive("disabled")).toBe(false);
  });
});

describe("isMethodologyAvailable", () => {
  it("should return true for available", () => {
    expect(isMethodologyAvailable("available")).toBe(true);
  });

  it("should return false for other statuses", () => {
    expect(isMethodologyAvailable("active")).toBe(false);
    expect(isMethodologyAvailable("disabled")).toBe(false);
  });
});

describe("isMethodologyDisabled", () => {
  it("should return true for disabled", () => {
    expect(isMethodologyDisabled("disabled")).toBe(true);
  });

  it("should return false for other statuses", () => {
    expect(isMethodologyDisabled("available")).toBe(false);
    expect(isMethodologyDisabled("active")).toBe(false);
  });
});

// ============================================
// MethodologyPhase Tests
// ============================================

describe("MethodologyPhaseSchema", () => {
  it("should validate minimal phase", () => {
    const phase: MethodologyPhase = {
      id: "analysis",
      name: "Analysis",
      order: 0,
      agentProfiles: [],
      columnIds: [],
    };
    expect(MethodologyPhaseSchema.parse(phase)).toEqual(phase);
  });

  it("should validate full phase", () => {
    const phase: MethodologyPhase = {
      id: "analysis",
      name: "Analysis Phase",
      order: 0,
      description: "Analyze requirements and research domain",
      agentProfiles: ["analyst", "researcher"],
      columnIds: ["brainstorm", "research"],
    };
    expect(MethodologyPhaseSchema.parse(phase)).toEqual(phase);
  });

  it("should default agentProfiles to empty array", () => {
    const result = MethodologyPhaseSchema.parse({
      id: "test",
      name: "Test",
      order: 0,
    });
    expect(result.agentProfiles).toEqual([]);
  });

  it("should default columnIds to empty array", () => {
    const result = MethodologyPhaseSchema.parse({
      id: "test",
      name: "Test",
      order: 0,
    });
    expect(result.columnIds).toEqual([]);
  });

  it("should reject missing id", () => {
    expect(() =>
      MethodologyPhaseSchema.parse({ name: "Test", order: 0 })
    ).toThrow();
  });

  it("should reject missing name", () => {
    expect(() =>
      MethodologyPhaseSchema.parse({ id: "test", order: 0 })
    ).toThrow();
  });

  it("should reject negative order", () => {
    expect(() =>
      MethodologyPhaseSchema.parse({ id: "test", name: "Test", order: -1 })
    ).toThrow();
  });
});

// ============================================
// MethodologyTemplate Tests
// ============================================

describe("MethodologyTemplateSchema", () => {
  it("should validate minimal template", () => {
    const template: MethodologyTemplate = {
      artifactType: "prd",
      templatePath: "templates/prd.md",
    };
    expect(MethodologyTemplateSchema.parse(template)).toEqual(template);
  });

  it("should validate full template", () => {
    const template: MethodologyTemplate = {
      artifactType: "prd",
      templatePath: "templates/prd.md",
      name: "PRD Template",
      description: "Product Requirements Document template",
    };
    expect(MethodologyTemplateSchema.parse(template)).toEqual(template);
  });

  it("should reject missing artifactType", () => {
    expect(() =>
      MethodologyTemplateSchema.parse({ templatePath: "templates/prd.md" })
    ).toThrow();
  });

  it("should reject missing templatePath", () => {
    expect(() =>
      MethodologyTemplateSchema.parse({ artifactType: "prd" })
    ).toThrow();
  });
});

// ============================================
// MethodologyExtension Tests
// ============================================

describe("MethodologyExtensionSchema", () => {
  const minimalMethodology: MethodologyExtension = {
    id: "test-method",
    name: "Test Methodology",
    workflow: {
      id: "test-workflow",
      name: "Test Workflow",
      columns: [
        { id: "backlog", name: "Backlog", mapsTo: "backlog" },
        { id: "done", name: "Done", mapsTo: "approved" },
      ],
      isDefault: false,
    },
    agentProfiles: [],
    skills: [],
    phases: [],
    templates: [],
    isActive: false,
    createdAt: "2024-01-15T10:00:00Z",
  };

  it("should validate minimal methodology", () => {
    expect(MethodologyExtensionSchema.parse(minimalMethodology)).toEqual(
      minimalMethodology
    );
  });

  it("should validate full methodology", () => {
    const full: MethodologyExtension = {
      ...minimalMethodology,
      description: "A test methodology",
      agentProfiles: ["analyst", "developer"],
      skills: ["skills/prd-creation"],
      phases: [{ id: "phase1", name: "Phase 1", order: 0, agentProfiles: [], columnIds: [] }],
      templates: [{ artifactType: "prd", templatePath: "templates/prd.md" }],
      hooksConfig: { phase_gates: {} },
    };
    expect(MethodologyExtensionSchema.parse(full)).toEqual(full);
  });

  it("should default arrays to empty", () => {
    const result = MethodologyExtensionSchema.parse({
      id: "test",
      name: "Test",
      workflow: minimalMethodology.workflow,
      isActive: false,
      createdAt: "2024-01-15T10:00:00Z",
    });
    expect(result.agentProfiles).toEqual([]);
    expect(result.skills).toEqual([]);
    expect(result.phases).toEqual([]);
    expect(result.templates).toEqual([]);
  });

  it("should default isActive to false", () => {
    const result = MethodologyExtensionSchema.parse({
      id: "test",
      name: "Test",
      workflow: minimalMethodology.workflow,
      createdAt: "2024-01-15T10:00:00Z",
    });
    expect(result.isActive).toBe(false);
  });

  it("should reject missing id", () => {
    expect(() =>
      MethodologyExtensionSchema.parse({
        name: "Test",
        workflow: minimalMethodology.workflow,
        createdAt: "2024-01-15T10:00:00Z",
      })
    ).toThrow();
  });

  it("should reject missing name", () => {
    expect(() =>
      MethodologyExtensionSchema.parse({
        id: "test",
        workflow: minimalMethodology.workflow,
        createdAt: "2024-01-15T10:00:00Z",
      })
    ).toThrow();
  });

  it("should reject missing workflow", () => {
    expect(() =>
      MethodologyExtensionSchema.parse({
        id: "test",
        name: "Test",
        createdAt: "2024-01-15T10:00:00Z",
      })
    ).toThrow();
  });
});

describe("CreateMethodologyExtensionInputSchema", () => {
  it("should validate minimal input", () => {
    const input = {
      name: "Test Methodology",
      workflow: {
        id: "test-workflow",
        name: "Test Workflow",
        columns: [{ id: "backlog", name: "Backlog", mapsTo: "backlog" }],
        isDefault: false,
      },
    };
    expect(CreateMethodologyExtensionInputSchema.parse(input)).toBeTruthy();
  });

  it("should validate input with optional fields", () => {
    const input = {
      name: "Test Methodology",
      description: "A test",
      workflow: {
        id: "test-workflow",
        name: "Test Workflow",
        columns: [{ id: "backlog", name: "Backlog", mapsTo: "backlog" }],
        isDefault: false,
      },
      agentProfiles: ["analyst"],
      skills: ["skills/prd"],
    };
    expect(CreateMethodologyExtensionInputSchema.parse(input)).toBeTruthy();
  });

  it("should reject missing name", () => {
    expect(() =>
      CreateMethodologyExtensionInputSchema.parse({
        workflow: {
          id: "test",
          name: "Test",
          columns: [],
          isDefault: false,
        },
      })
    ).toThrow();
  });

  it("should reject missing workflow", () => {
    expect(() =>
      CreateMethodologyExtensionInputSchema.parse({
        name: "Test Methodology",
      })
    ).toThrow();
  });
});

// ============================================
// Built-in Methodologies Tests
// ============================================

describe("BMAD_METHODOLOGY", () => {
  it("should have correct id", () => {
    expect(BMAD_METHODOLOGY.id).toBe("bmad-method");
  });

  it("should have correct name", () => {
    expect(BMAD_METHODOLOGY.name).toBe("BMAD Method");
  });

  it("should have description", () => {
    expect(BMAD_METHODOLOGY.description).toBeDefined();
    expect(BMAD_METHODOLOGY.description).toContain("Breakthrough Method");
  });

  it("should have 8 agent profiles", () => {
    expect(BMAD_METHODOLOGY.agentProfiles).toHaveLength(8);
    expect(BMAD_METHODOLOGY.agentProfiles).toContain("bmad-analyst");
    expect(BMAD_METHODOLOGY.agentProfiles).toContain("bmad-pm");
    expect(BMAD_METHODOLOGY.agentProfiles).toContain("bmad-architect");
    expect(BMAD_METHODOLOGY.agentProfiles).toContain("bmad-ux");
    expect(BMAD_METHODOLOGY.agentProfiles).toContain("bmad-developer");
  });

  it("should have 4 phases", () => {
    expect(BMAD_METHODOLOGY.phases).toHaveLength(4);
    expect(BMAD_METHODOLOGY.phases[0].name).toBe("Analysis");
    expect(BMAD_METHODOLOGY.phases[1].name).toBe("Planning");
    expect(BMAD_METHODOLOGY.phases[2].name).toBe("Solutioning");
    expect(BMAD_METHODOLOGY.phases[3].name).toBe("Implementation");
  });

  it("should have 10 workflow columns", () => {
    expect(BMAD_METHODOLOGY.workflow.columns).toHaveLength(10);
  });

  it("should have templates", () => {
    expect(BMAD_METHODOLOGY.templates.length).toBeGreaterThan(0);
    const types = BMAD_METHODOLOGY.templates.map((t) => t.artifactType);
    expect(types).toContain("prd");
  });

  it("should not be active by default", () => {
    expect(BMAD_METHODOLOGY.isActive).toBe(false);
  });

  it("should be valid according to schema", () => {
    expect(MethodologyExtensionSchema.parse(BMAD_METHODOLOGY)).toBeTruthy();
  });
});

describe("GSD_METHODOLOGY", () => {
  it("should have correct id", () => {
    expect(GSD_METHODOLOGY.id).toBe("gsd-method");
  });

  it("should have correct name", () => {
    expect(GSD_METHODOLOGY.name).toBe("GSD (Get Shit Done)");
  });

  it("should have description", () => {
    expect(GSD_METHODOLOGY.description).toBeDefined();
    expect(GSD_METHODOLOGY.description).toContain("wave-based");
  });

  it("should have 11 agent profiles", () => {
    expect(GSD_METHODOLOGY.agentProfiles).toHaveLength(11);
    expect(GSD_METHODOLOGY.agentProfiles).toContain("gsd-project-researcher");
    expect(GSD_METHODOLOGY.agentProfiles).toContain("gsd-planner");
    expect(GSD_METHODOLOGY.agentProfiles).toContain("gsd-executor");
    expect(GSD_METHODOLOGY.agentProfiles).toContain("gsd-verifier");
  });

  it("should have 4 phases", () => {
    expect(GSD_METHODOLOGY.phases).toHaveLength(4);
    expect(GSD_METHODOLOGY.phases[0].name).toBe("Initialize");
    expect(GSD_METHODOLOGY.phases[1].name).toBe("Plan");
    expect(GSD_METHODOLOGY.phases[2].name).toBe("Execute");
    expect(GSD_METHODOLOGY.phases[3].name).toBe("Verify");
  });

  it("should have 11 workflow columns", () => {
    expect(GSD_METHODOLOGY.workflow.columns).toHaveLength(11);
  });

  it("should have templates", () => {
    expect(GSD_METHODOLOGY.templates.length).toBeGreaterThan(0);
  });

  it("should not be active by default", () => {
    expect(GSD_METHODOLOGY.isActive).toBe(false);
  });

  it("should be valid according to schema", () => {
    expect(MethodologyExtensionSchema.parse(GSD_METHODOLOGY)).toBeTruthy();
  });
});

describe("BUILTIN_METHODOLOGIES", () => {
  it("should have 2 methodologies", () => {
    expect(BUILTIN_METHODOLOGIES).toHaveLength(2);
  });

  it("should include BMAD and GSD", () => {
    const ids = BUILTIN_METHODOLOGIES.map((m) => m.id);
    expect(ids).toContain("bmad-method");
    expect(ids).toContain("gsd-method");
  });
});

describe("getBuiltinMethodology", () => {
  it("should return BMAD methodology", () => {
    const bmad = getBuiltinMethodology("bmad-method");
    expect(bmad).toBeDefined();
    expect(bmad?.name).toBe("BMAD Method");
  });

  it("should return GSD methodology", () => {
    const gsd = getBuiltinMethodology("gsd-method");
    expect(gsd).toBeDefined();
    expect(gsd?.name).toBe("GSD (Get Shit Done)");
  });

  it("should return undefined for unknown id", () => {
    const unknown = getBuiltinMethodology("unknown-method");
    expect(unknown).toBeUndefined();
  });
});

// ============================================
// Parsing Helpers Tests
// ============================================

describe("parseMethodologyExtension", () => {
  it("should parse valid methodology", () => {
    const data = {
      id: "test",
      name: "Test",
      workflow: {
        id: "wf",
        name: "Workflow",
        columns: [{ id: "col1", name: "Col 1", mapsTo: "backlog" }],
        isDefault: false,
      },
      createdAt: "2024-01-15T10:00:00Z",
    };
    const result = parseMethodologyExtension(data);
    expect(result.id).toBe("test");
    expect(result.name).toBe("Test");
  });

  it("should throw for invalid data", () => {
    expect(() => parseMethodologyExtension({})).toThrow();
  });
});

describe("safeParseMethodologyExtension", () => {
  it("should return methodology for valid data", () => {
    const data = {
      id: "test",
      name: "Test",
      workflow: {
        id: "wf",
        name: "Workflow",
        columns: [{ id: "col1", name: "Col 1", mapsTo: "backlog" }],
        isDefault: false,
      },
      createdAt: "2024-01-15T10:00:00Z",
    };
    const result = safeParseMethodologyExtension(data);
    expect(result).not.toBeNull();
    expect(result?.id).toBe("test");
  });

  it("should return null for invalid data", () => {
    const result = safeParseMethodologyExtension({});
    expect(result).toBeNull();
  });
});

describe("parseMethodologyPhase", () => {
  it("should parse valid phase", () => {
    const data = { id: "phase1", name: "Phase 1", order: 0 };
    const result = parseMethodologyPhase(data);
    expect(result.id).toBe("phase1");
    expect(result.name).toBe("Phase 1");
    expect(result.order).toBe(0);
  });

  it("should throw for invalid data", () => {
    expect(() => parseMethodologyPhase({ id: "test" })).toThrow();
  });
});

describe("safeParseMethodologyPhase", () => {
  it("should return phase for valid data", () => {
    const result = safeParseMethodologyPhase({
      id: "phase1",
      name: "Phase 1",
      order: 0,
    });
    expect(result).not.toBeNull();
    expect(result?.id).toBe("phase1");
  });

  it("should return null for invalid data", () => {
    const result = safeParseMethodologyPhase({ id: "test" });
    expect(result).toBeNull();
  });
});

// ============================================
// Methodology Phase Column Mapping Tests
// ============================================

describe("BMAD workflow column behaviors", () => {
  it("should have agent profiles on columns", () => {
    const brainstorm = BMAD_METHODOLOGY.workflow.columns.find(
      (c) => c.id === "brainstorm"
    );
    expect(brainstorm?.behavior?.agentProfile).toBe("bmad-analyst");

    const sprint = BMAD_METHODOLOGY.workflow.columns.find(
      (c) => c.id === "sprint"
    );
    expect(sprint?.behavior?.agentProfile).toBe("bmad-developer");
  });
});

describe("GSD workflow column behaviors", () => {
  it("should have agent profiles on columns", () => {
    const initialize = GSD_METHODOLOGY.workflow.columns.find(
      (c) => c.id === "initialize"
    );
    expect(initialize?.behavior?.agentProfile).toBe("gsd-project-researcher");

    const executing = GSD_METHODOLOGY.workflow.columns.find(
      (c) => c.id === "executing"
    );
    expect(executing?.behavior?.agentProfile).toBe("gsd-executor");
  });
});

// ============================================
// Phase ordering Tests
// ============================================

describe("Methodology phases are properly ordered", () => {
  it("BMAD phases should be in correct order", () => {
    const phases = [...BMAD_METHODOLOGY.phases].sort((a, b) => a.order - b.order);
    expect(phases[0].id).toBe("analysis");
    expect(phases[1].id).toBe("planning");
    expect(phases[2].id).toBe("solutioning");
    expect(phases[3].id).toBe("implementation");
  });

  it("GSD phases should be in correct order", () => {
    const phases = [...GSD_METHODOLOGY.phases].sort((a, b) => a.order - b.order);
    expect(phases[0].id).toBe("initialize");
    expect(phases[1].id).toBe("plan");
    expect(phases[2].id).toBe("execute");
    expect(phases[3].id).toBe("verify");
  });
});
