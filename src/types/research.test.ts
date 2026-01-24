import { describe, it, expect } from "vitest";
import {
  // Schemas
  ResearchDepthPresetSchema,
  CustomDepthSchema,
  ResearchDepthPresetVariantSchema,
  ResearchDepthCustomVariantSchema,
  ResearchDepthSchema,
  ResearchProcessStatusSchema,
  ResearchBriefSchema,
  ResearchOutputSchema,
  ResearchProgressSchema,
  ResearchProcessSchema,
  CreateResearchProcessInputSchema,
  ResearchPresetInfoSchema,
  // Constants
  RESEARCH_DEPTH_PRESET_VALUES,
  RESEARCH_PRESETS,
  RESEARCH_PROCESS_STATUS_VALUES,
  ACTIVE_RESEARCH_STATUSES,
  TERMINAL_RESEARCH_STATUSES,
  DEFAULT_RESEARCH_OUTPUT,
  RESEARCH_PRESET_INFO,
  // Functions
  getPresetConfig,
  createPresetDepth,
  createCustomDepth,
  resolveDepth,
  isPresetDepth,
  isCustomDepth,
  isActiveResearchStatus,
  isTerminalResearchStatus,
  isPausedResearchStatus,
  createResearchBrief,
  createFullResearchBrief,
  createResearchOutput,
  createResearchProgress,
  calculateProgressPercentage,
  shouldCheckpoint,
  getResolvedDepth,
  getProcessProgressPercentage,
  processShouldCheckpoint,
  isMaxIterationsReached,
  isProcessActive,
  isProcessTerminal,
  isProcessPaused,
  getPresetInfo,
  parseResearchProcess,
  safeParseResearchProcess,
  parseResearchBrief,
  safeParseResearchBrief,
  parseResearchDepth,
  safeParseResearchDepth,
  // Types
  type ResearchDepthPreset,
  type CustomDepth,
  type ResearchDepth,
  type ResearchProcessStatus,
  type ResearchBrief,
  type ResearchOutput,
  type ResearchProgress,
  type ResearchProcess,
} from "./research";

// ============================================
// ResearchDepthPreset Tests
// ============================================

describe("ResearchDepthPresetSchema", () => {
  it("should validate all preset values", () => {
    expect(ResearchDepthPresetSchema.parse("quick-scan")).toBe("quick-scan");
    expect(ResearchDepthPresetSchema.parse("standard")).toBe("standard");
    expect(ResearchDepthPresetSchema.parse("deep-dive")).toBe("deep-dive");
    expect(ResearchDepthPresetSchema.parse("exhaustive")).toBe("exhaustive");
  });

  it("should reject invalid values", () => {
    expect(() => ResearchDepthPresetSchema.parse("invalid")).toThrow();
    expect(() => ResearchDepthPresetSchema.parse("")).toThrow();
    expect(() => ResearchDepthPresetSchema.parse(123)).toThrow();
  });
});

describe("RESEARCH_DEPTH_PRESET_VALUES", () => {
  it("should have 4 presets", () => {
    expect(RESEARCH_DEPTH_PRESET_VALUES).toHaveLength(4);
  });

  it("should include all presets", () => {
    expect(RESEARCH_DEPTH_PRESET_VALUES).toContain("quick-scan");
    expect(RESEARCH_DEPTH_PRESET_VALUES).toContain("standard");
    expect(RESEARCH_DEPTH_PRESET_VALUES).toContain("deep-dive");
    expect(RESEARCH_DEPTH_PRESET_VALUES).toContain("exhaustive");
  });
});

// ============================================
// CustomDepth Tests
// ============================================

describe("CustomDepthSchema", () => {
  it("should validate valid custom depth", () => {
    const depth: CustomDepth = {
      maxIterations: 100,
      timeoutHours: 4,
      checkpointInterval: 20,
    };
    expect(CustomDepthSchema.parse(depth)).toEqual(depth);
  });

  it("should require positive integers for maxIterations", () => {
    expect(() =>
      CustomDepthSchema.parse({
        maxIterations: 0,
        timeoutHours: 1,
        checkpointInterval: 5,
      })
    ).toThrow();
    expect(() =>
      CustomDepthSchema.parse({
        maxIterations: -1,
        timeoutHours: 1,
        checkpointInterval: 5,
      })
    ).toThrow();
  });

  it("should require positive timeoutHours", () => {
    expect(() =>
      CustomDepthSchema.parse({
        maxIterations: 10,
        timeoutHours: 0,
        checkpointInterval: 5,
      })
    ).toThrow();
    expect(() =>
      CustomDepthSchema.parse({
        maxIterations: 10,
        timeoutHours: -1,
        checkpointInterval: 5,
      })
    ).toThrow();
  });

  it("should require positive integers for checkpointInterval", () => {
    expect(() =>
      CustomDepthSchema.parse({
        maxIterations: 10,
        timeoutHours: 1,
        checkpointInterval: 0,
      })
    ).toThrow();
  });
});

describe("RESEARCH_PRESETS", () => {
  it("should have quick-scan preset", () => {
    expect(RESEARCH_PRESETS["quick-scan"]).toEqual({
      maxIterations: 10,
      timeoutHours: 0.5,
      checkpointInterval: 5,
    });
  });

  it("should have standard preset", () => {
    expect(RESEARCH_PRESETS["standard"]).toEqual({
      maxIterations: 50,
      timeoutHours: 2,
      checkpointInterval: 10,
    });
  });

  it("should have deep-dive preset", () => {
    expect(RESEARCH_PRESETS["deep-dive"]).toEqual({
      maxIterations: 200,
      timeoutHours: 8,
      checkpointInterval: 25,
    });
  });

  it("should have exhaustive preset", () => {
    expect(RESEARCH_PRESETS["exhaustive"]).toEqual({
      maxIterations: 500,
      timeoutHours: 24,
      checkpointInterval: 50,
    });
  });
});

describe("getPresetConfig", () => {
  it("should return config for quick-scan", () => {
    expect(getPresetConfig("quick-scan")).toEqual(RESEARCH_PRESETS["quick-scan"]);
  });

  it("should return config for standard", () => {
    expect(getPresetConfig("standard")).toEqual(RESEARCH_PRESETS["standard"]);
  });

  it("should return config for deep-dive", () => {
    expect(getPresetConfig("deep-dive")).toEqual(RESEARCH_PRESETS["deep-dive"]);
  });

  it("should return config for exhaustive", () => {
    expect(getPresetConfig("exhaustive")).toEqual(RESEARCH_PRESETS["exhaustive"]);
  });
});

// ============================================
// ResearchDepth Tests
// ============================================

describe("ResearchDepthPresetVariantSchema", () => {
  it("should validate preset variant", () => {
    const depth = { type: "preset" as const, preset: "standard" as const };
    expect(ResearchDepthPresetVariantSchema.parse(depth)).toEqual(depth);
  });

  it("should reject invalid preset", () => {
    expect(() =>
      ResearchDepthPresetVariantSchema.parse({
        type: "preset",
        preset: "invalid",
      })
    ).toThrow();
  });
});

describe("ResearchDepthCustomVariantSchema", () => {
  it("should validate custom variant", () => {
    const depth = {
      type: "custom" as const,
      config: { maxIterations: 100, timeoutHours: 4, checkpointInterval: 20 },
    };
    expect(ResearchDepthCustomVariantSchema.parse(depth)).toEqual(depth);
  });

  it("should reject invalid config", () => {
    expect(() =>
      ResearchDepthCustomVariantSchema.parse({
        type: "custom",
        config: { maxIterations: -1, timeoutHours: 1, checkpointInterval: 5 },
      })
    ).toThrow();
  });
});

describe("ResearchDepthSchema", () => {
  it("should validate preset depth", () => {
    const depth: ResearchDepth = { type: "preset", preset: "deep-dive" };
    expect(ResearchDepthSchema.parse(depth)).toEqual(depth);
  });

  it("should validate custom depth", () => {
    const depth: ResearchDepth = {
      type: "custom",
      config: { maxIterations: 150, timeoutHours: 5, checkpointInterval: 30 },
    };
    expect(ResearchDepthSchema.parse(depth)).toEqual(depth);
  });

  it("should reject invalid type", () => {
    expect(() =>
      ResearchDepthSchema.parse({ type: "invalid", preset: "standard" })
    ).toThrow();
  });
});

describe("createPresetDepth", () => {
  it("should create preset depth", () => {
    const depth = createPresetDepth("quick-scan");
    expect(depth).toEqual({ type: "preset", preset: "quick-scan" });
  });
});

describe("createCustomDepth", () => {
  it("should create custom depth", () => {
    const config: CustomDepth = {
      maxIterations: 100,
      timeoutHours: 4,
      checkpointInterval: 20,
    };
    const depth = createCustomDepth(config);
    expect(depth).toEqual({ type: "custom", config });
  });
});

describe("resolveDepth", () => {
  it("should resolve preset to config", () => {
    const depth = createPresetDepth("quick-scan");
    expect(resolveDepth(depth)).toEqual(RESEARCH_PRESETS["quick-scan"]);
  });

  it("should return custom config as-is", () => {
    const config: CustomDepth = {
      maxIterations: 100,
      timeoutHours: 4,
      checkpointInterval: 20,
    };
    const depth = createCustomDepth(config);
    expect(resolveDepth(depth)).toEqual(config);
  });
});

describe("isPresetDepth", () => {
  it("should return true for preset", () => {
    const depth = createPresetDepth("standard");
    expect(isPresetDepth(depth)).toBe(true);
  });

  it("should return false for custom", () => {
    const depth = createCustomDepth({
      maxIterations: 100,
      timeoutHours: 4,
      checkpointInterval: 20,
    });
    expect(isPresetDepth(depth)).toBe(false);
  });
});

describe("isCustomDepth", () => {
  it("should return true for custom", () => {
    const depth = createCustomDepth({
      maxIterations: 100,
      timeoutHours: 4,
      checkpointInterval: 20,
    });
    expect(isCustomDepth(depth)).toBe(true);
  });

  it("should return false for preset", () => {
    const depth = createPresetDepth("standard");
    expect(isCustomDepth(depth)).toBe(false);
  });
});

// ============================================
// ResearchProcessStatus Tests
// ============================================

describe("ResearchProcessStatusSchema", () => {
  it("should validate all status values", () => {
    expect(ResearchProcessStatusSchema.parse("pending")).toBe("pending");
    expect(ResearchProcessStatusSchema.parse("running")).toBe("running");
    expect(ResearchProcessStatusSchema.parse("paused")).toBe("paused");
    expect(ResearchProcessStatusSchema.parse("completed")).toBe("completed");
    expect(ResearchProcessStatusSchema.parse("failed")).toBe("failed");
  });

  it("should reject invalid values", () => {
    expect(() => ResearchProcessStatusSchema.parse("invalid")).toThrow();
  });
});

describe("RESEARCH_PROCESS_STATUS_VALUES", () => {
  it("should have 5 statuses", () => {
    expect(RESEARCH_PROCESS_STATUS_VALUES).toHaveLength(5);
  });

  it("should include all statuses", () => {
    expect(RESEARCH_PROCESS_STATUS_VALUES).toContain("pending");
    expect(RESEARCH_PROCESS_STATUS_VALUES).toContain("running");
    expect(RESEARCH_PROCESS_STATUS_VALUES).toContain("paused");
    expect(RESEARCH_PROCESS_STATUS_VALUES).toContain("completed");
    expect(RESEARCH_PROCESS_STATUS_VALUES).toContain("failed");
  });
});

describe("ACTIVE_RESEARCH_STATUSES", () => {
  it("should include pending and running", () => {
    expect(ACTIVE_RESEARCH_STATUSES).toContain("pending");
    expect(ACTIVE_RESEARCH_STATUSES).toContain("running");
  });

  it("should have 2 statuses", () => {
    expect(ACTIVE_RESEARCH_STATUSES).toHaveLength(2);
  });
});

describe("TERMINAL_RESEARCH_STATUSES", () => {
  it("should include completed and failed", () => {
    expect(TERMINAL_RESEARCH_STATUSES).toContain("completed");
    expect(TERMINAL_RESEARCH_STATUSES).toContain("failed");
  });

  it("should have 2 statuses", () => {
    expect(TERMINAL_RESEARCH_STATUSES).toHaveLength(2);
  });
});

describe("isActiveResearchStatus", () => {
  it("should return true for pending", () => {
    expect(isActiveResearchStatus("pending")).toBe(true);
  });

  it("should return true for running", () => {
    expect(isActiveResearchStatus("running")).toBe(true);
  });

  it("should return false for paused", () => {
    expect(isActiveResearchStatus("paused")).toBe(false);
  });

  it("should return false for completed", () => {
    expect(isActiveResearchStatus("completed")).toBe(false);
  });

  it("should return false for failed", () => {
    expect(isActiveResearchStatus("failed")).toBe(false);
  });
});

describe("isTerminalResearchStatus", () => {
  it("should return true for completed", () => {
    expect(isTerminalResearchStatus("completed")).toBe(true);
  });

  it("should return true for failed", () => {
    expect(isTerminalResearchStatus("failed")).toBe(true);
  });

  it("should return false for pending", () => {
    expect(isTerminalResearchStatus("pending")).toBe(false);
  });

  it("should return false for running", () => {
    expect(isTerminalResearchStatus("running")).toBe(false);
  });

  it("should return false for paused", () => {
    expect(isTerminalResearchStatus("paused")).toBe(false);
  });
});

describe("isPausedResearchStatus", () => {
  it("should return true for paused", () => {
    expect(isPausedResearchStatus("paused")).toBe(true);
  });

  it("should return false for other statuses", () => {
    expect(isPausedResearchStatus("pending")).toBe(false);
    expect(isPausedResearchStatus("running")).toBe(false);
    expect(isPausedResearchStatus("completed")).toBe(false);
    expect(isPausedResearchStatus("failed")).toBe(false);
  });
});

// ============================================
// ResearchBrief Tests
// ============================================

describe("ResearchBriefSchema", () => {
  it("should validate brief with only question", () => {
    const brief: ResearchBrief = {
      question: "What is the best architecture?",
      constraints: [],
    };
    expect(ResearchBriefSchema.parse(brief)).toEqual(brief);
  });

  it("should validate full brief", () => {
    const brief: ResearchBrief = {
      question: "What is the best architecture?",
      context: "Building a new feature",
      scope: "Backend only",
      constraints: ["Must be fast", "Must be secure"],
    };
    expect(ResearchBriefSchema.parse(brief)).toEqual(brief);
  });

  it("should default constraints to empty array", () => {
    const result = ResearchBriefSchema.parse({ question: "Test?" });
    expect(result.constraints).toEqual([]);
  });

  it("should reject empty question", () => {
    expect(() => ResearchBriefSchema.parse({ question: "" })).toThrow();
  });
});

describe("createResearchBrief", () => {
  it("should create brief with just question", () => {
    const brief = createResearchBrief("What framework to use?");
    expect(brief.question).toBe("What framework to use?");
    expect(brief.constraints).toEqual([]);
    expect(brief.context).toBeUndefined();
    expect(brief.scope).toBeUndefined();
  });
});

describe("createFullResearchBrief", () => {
  it("should create full brief", () => {
    const brief = createFullResearchBrief(
      "What framework?",
      "Building API",
      "Backend",
      ["Fast", "Secure"]
    );
    expect(brief.question).toBe("What framework?");
    expect(brief.context).toBe("Building API");
    expect(brief.scope).toBe("Backend");
    expect(brief.constraints).toEqual(["Fast", "Secure"]);
  });

  it("should handle missing optional fields", () => {
    const brief = createFullResearchBrief("What framework?");
    expect(brief.question).toBe("What framework?");
    expect(brief.context).toBeUndefined();
    expect(brief.scope).toBeUndefined();
    expect(brief.constraints).toEqual([]);
  });
});

// ============================================
// ResearchOutput Tests
// ============================================

describe("ResearchOutputSchema", () => {
  it("should validate research output", () => {
    const output: ResearchOutput = {
      targetBucket: "my-bucket",
      artifactTypes: ["research_document", "findings"],
    };
    expect(ResearchOutputSchema.parse(output)).toEqual(output);
  });

  it("should default artifactTypes to empty array", () => {
    const result = ResearchOutputSchema.parse({ targetBucket: "bucket" });
    expect(result.artifactTypes).toEqual([]);
  });
});

describe("DEFAULT_RESEARCH_OUTPUT", () => {
  it("should have research-outputs as target bucket", () => {
    expect(DEFAULT_RESEARCH_OUTPUT.targetBucket).toBe("research-outputs");
  });

  it("should include standard artifact types", () => {
    expect(DEFAULT_RESEARCH_OUTPUT.artifactTypes).toContain("research_document");
    expect(DEFAULT_RESEARCH_OUTPUT.artifactTypes).toContain("findings");
    expect(DEFAULT_RESEARCH_OUTPUT.artifactTypes).toContain("recommendations");
  });
});

describe("createResearchOutput", () => {
  it("should create output with bucket only", () => {
    const output = createResearchOutput("my-bucket");
    expect(output.targetBucket).toBe("my-bucket");
    expect(output.artifactTypes).toEqual([]);
  });

  it("should create output with artifact types", () => {
    const output = createResearchOutput("my-bucket", ["findings"]);
    expect(output.targetBucket).toBe("my-bucket");
    expect(output.artifactTypes).toEqual(["findings"]);
  });
});

// ============================================
// ResearchProgress Tests
// ============================================

describe("ResearchProgressSchema", () => {
  it("should validate progress", () => {
    const progress: ResearchProgress = {
      currentIteration: 10,
      status: "running",
      lastCheckpoint: "checkpoint-1",
      errorMessage: undefined,
    };
    expect(ResearchProgressSchema.parse(progress)).toEqual(progress);
  });

  it("should default currentIteration to 0", () => {
    const result = ResearchProgressSchema.parse({});
    expect(result.currentIteration).toBe(0);
  });

  it("should default status to pending", () => {
    const result = ResearchProgressSchema.parse({});
    expect(result.status).toBe("pending");
  });

  it("should reject negative currentIteration", () => {
    expect(() =>
      ResearchProgressSchema.parse({ currentIteration: -1 })
    ).toThrow();
  });
});

describe("createResearchProgress", () => {
  it("should create initial progress", () => {
    const progress = createResearchProgress();
    expect(progress.currentIteration).toBe(0);
    expect(progress.status).toBe("pending");
    expect(progress.lastCheckpoint).toBeUndefined();
    expect(progress.errorMessage).toBeUndefined();
  });
});

describe("calculateProgressPercentage", () => {
  it("should return 0 for 0 iterations", () => {
    expect(calculateProgressPercentage(0, 100)).toBe(0);
  });

  it("should return 50 for half iterations", () => {
    expect(calculateProgressPercentage(50, 100)).toBe(50);
  });

  it("should return 100 for max iterations", () => {
    expect(calculateProgressPercentage(100, 100)).toBe(100);
  });

  it("should cap at 100 for over max iterations", () => {
    expect(calculateProgressPercentage(150, 100)).toBe(100);
  });

  it("should return 0 for 0 max iterations", () => {
    expect(calculateProgressPercentage(50, 0)).toBe(0);
  });
});

describe("shouldCheckpoint", () => {
  it("should return false for iteration 0", () => {
    expect(shouldCheckpoint(0, 5)).toBe(false);
  });

  it("should return true at checkpoint interval", () => {
    expect(shouldCheckpoint(5, 5)).toBe(true);
    expect(shouldCheckpoint(10, 5)).toBe(true);
  });

  it("should return false between intervals", () => {
    expect(shouldCheckpoint(3, 5)).toBe(false);
    expect(shouldCheckpoint(7, 5)).toBe(false);
  });

  it("should return false for interval 0", () => {
    expect(shouldCheckpoint(5, 0)).toBe(false);
  });
});

// ============================================
// ResearchProcess Tests
// ============================================

describe("ResearchProcessSchema", () => {
  const validProcess: ResearchProcess = {
    id: "process-1",
    name: "Architecture Research",
    brief: { question: "What framework?", constraints: [] },
    depth: { type: "preset", preset: "standard" },
    agentProfileId: "deep-researcher",
    output: { targetBucket: "research-outputs", artifactTypes: ["findings"] },
    progress: { currentIteration: 5, status: "running" },
    createdAt: "2024-01-15T10:00:00Z",
    startedAt: "2024-01-15T10:01:00Z",
  };

  it("should validate valid process", () => {
    expect(ResearchProcessSchema.parse(validProcess)).toEqual(validProcess);
  });

  it("should allow optional startedAt and completedAt", () => {
    const process = { ...validProcess, startedAt: undefined, completedAt: undefined };
    expect(ResearchProcessSchema.parse(process)).toBeTruthy();
  });

  it("should require all mandatory fields", () => {
    expect(() => ResearchProcessSchema.parse({})).toThrow();
    expect(() => ResearchProcessSchema.parse({ id: "1" })).toThrow();
  });
});

describe("CreateResearchProcessInputSchema", () => {
  it("should validate input with required fields", () => {
    const input = {
      name: "Test Research",
      brief: { question: "What to do?" },
      agentProfileId: "deep-researcher",
    };
    expect(CreateResearchProcessInputSchema.parse(input)).toBeTruthy();
  });

  it("should allow optional depth and output", () => {
    const input = {
      name: "Test",
      brief: { question: "Test?" },
      agentProfileId: "agent",
      depth: { type: "preset" as const, preset: "quick-scan" as const },
      output: { targetBucket: "bucket", artifactTypes: [] },
    };
    expect(CreateResearchProcessInputSchema.parse(input)).toBeTruthy();
  });
});

describe("getResolvedDepth", () => {
  it("should resolve preset depth", () => {
    const process: ResearchProcess = {
      id: "1",
      name: "Test",
      brief: { question: "Test?", constraints: [] },
      depth: { type: "preset", preset: "quick-scan" },
      agentProfileId: "agent",
      output: { targetBucket: "bucket", artifactTypes: [] },
      progress: { currentIteration: 0, status: "pending" },
      createdAt: "2024-01-15T10:00:00Z",
    };
    expect(getResolvedDepth(process)).toEqual(RESEARCH_PRESETS["quick-scan"]);
  });

  it("should resolve custom depth", () => {
    const customConfig: CustomDepth = {
      maxIterations: 100,
      timeoutHours: 4,
      checkpointInterval: 20,
    };
    const process: ResearchProcess = {
      id: "1",
      name: "Test",
      brief: { question: "Test?", constraints: [] },
      depth: { type: "custom", config: customConfig },
      agentProfileId: "agent",
      output: { targetBucket: "bucket", artifactTypes: [] },
      progress: { currentIteration: 0, status: "pending" },
      createdAt: "2024-01-15T10:00:00Z",
    };
    expect(getResolvedDepth(process)).toEqual(customConfig);
  });
});

describe("getProcessProgressPercentage", () => {
  it("should calculate percentage based on resolved depth", () => {
    const process: ResearchProcess = {
      id: "1",
      name: "Test",
      brief: { question: "Test?", constraints: [] },
      depth: { type: "preset", preset: "quick-scan" }, // 10 max iterations
      agentProfileId: "agent",
      output: { targetBucket: "bucket", artifactTypes: [] },
      progress: { currentIteration: 5, status: "running" },
      createdAt: "2024-01-15T10:00:00Z",
    };
    expect(getProcessProgressPercentage(process)).toBe(50);
  });
});

describe("processShouldCheckpoint", () => {
  it("should check based on resolved depth interval", () => {
    const process: ResearchProcess = {
      id: "1",
      name: "Test",
      brief: { question: "Test?", constraints: [] },
      depth: { type: "preset", preset: "quick-scan" }, // checkpoint_interval = 5
      agentProfileId: "agent",
      output: { targetBucket: "bucket", artifactTypes: [] },
      progress: { currentIteration: 5, status: "running" },
      createdAt: "2024-01-15T10:00:00Z",
    };
    expect(processShouldCheckpoint(process)).toBe(true);
  });

  it("should return false when not at interval", () => {
    const process: ResearchProcess = {
      id: "1",
      name: "Test",
      brief: { question: "Test?", constraints: [] },
      depth: { type: "preset", preset: "quick-scan" },
      agentProfileId: "agent",
      output: { targetBucket: "bucket", artifactTypes: [] },
      progress: { currentIteration: 3, status: "running" },
      createdAt: "2024-01-15T10:00:00Z",
    };
    expect(processShouldCheckpoint(process)).toBe(false);
  });
});

describe("isMaxIterationsReached", () => {
  it("should return true when at max iterations", () => {
    const process: ResearchProcess = {
      id: "1",
      name: "Test",
      brief: { question: "Test?", constraints: [] },
      depth: { type: "preset", preset: "quick-scan" }, // 10 max iterations
      agentProfileId: "agent",
      output: { targetBucket: "bucket", artifactTypes: [] },
      progress: { currentIteration: 10, status: "running" },
      createdAt: "2024-01-15T10:00:00Z",
    };
    expect(isMaxIterationsReached(process)).toBe(true);
  });

  it("should return true when over max iterations", () => {
    const process: ResearchProcess = {
      id: "1",
      name: "Test",
      brief: { question: "Test?", constraints: [] },
      depth: { type: "preset", preset: "quick-scan" },
      agentProfileId: "agent",
      output: { targetBucket: "bucket", artifactTypes: [] },
      progress: { currentIteration: 15, status: "running" },
      createdAt: "2024-01-15T10:00:00Z",
    };
    expect(isMaxIterationsReached(process)).toBe(true);
  });

  it("should return false when under max iterations", () => {
    const process: ResearchProcess = {
      id: "1",
      name: "Test",
      brief: { question: "Test?", constraints: [] },
      depth: { type: "preset", preset: "quick-scan" },
      agentProfileId: "agent",
      output: { targetBucket: "bucket", artifactTypes: [] },
      progress: { currentIteration: 5, status: "running" },
      createdAt: "2024-01-15T10:00:00Z",
    };
    expect(isMaxIterationsReached(process)).toBe(false);
  });
});

describe("isProcessActive", () => {
  it("should return true for pending process", () => {
    const process: ResearchProcess = {
      id: "1",
      name: "Test",
      brief: { question: "Test?", constraints: [] },
      depth: { type: "preset", preset: "standard" },
      agentProfileId: "agent",
      output: { targetBucket: "bucket", artifactTypes: [] },
      progress: { currentIteration: 0, status: "pending" },
      createdAt: "2024-01-15T10:00:00Z",
    };
    expect(isProcessActive(process)).toBe(true);
  });

  it("should return true for running process", () => {
    const process: ResearchProcess = {
      id: "1",
      name: "Test",
      brief: { question: "Test?", constraints: [] },
      depth: { type: "preset", preset: "standard" },
      agentProfileId: "agent",
      output: { targetBucket: "bucket", artifactTypes: [] },
      progress: { currentIteration: 5, status: "running" },
      createdAt: "2024-01-15T10:00:00Z",
    };
    expect(isProcessActive(process)).toBe(true);
  });

  it("should return false for paused process", () => {
    const process: ResearchProcess = {
      id: "1",
      name: "Test",
      brief: { question: "Test?", constraints: [] },
      depth: { type: "preset", preset: "standard" },
      agentProfileId: "agent",
      output: { targetBucket: "bucket", artifactTypes: [] },
      progress: { currentIteration: 5, status: "paused" },
      createdAt: "2024-01-15T10:00:00Z",
    };
    expect(isProcessActive(process)).toBe(false);
  });
});

describe("isProcessTerminal", () => {
  it("should return true for completed process", () => {
    const process: ResearchProcess = {
      id: "1",
      name: "Test",
      brief: { question: "Test?", constraints: [] },
      depth: { type: "preset", preset: "standard" },
      agentProfileId: "agent",
      output: { targetBucket: "bucket", artifactTypes: [] },
      progress: { currentIteration: 50, status: "completed" },
      createdAt: "2024-01-15T10:00:00Z",
    };
    expect(isProcessTerminal(process)).toBe(true);
  });

  it("should return true for failed process", () => {
    const process: ResearchProcess = {
      id: "1",
      name: "Test",
      brief: { question: "Test?", constraints: [] },
      depth: { type: "preset", preset: "standard" },
      agentProfileId: "agent",
      output: { targetBucket: "bucket", artifactTypes: [] },
      progress: { currentIteration: 10, status: "failed", errorMessage: "Error" },
      createdAt: "2024-01-15T10:00:00Z",
    };
    expect(isProcessTerminal(process)).toBe(true);
  });

  it("should return false for running process", () => {
    const process: ResearchProcess = {
      id: "1",
      name: "Test",
      brief: { question: "Test?", constraints: [] },
      depth: { type: "preset", preset: "standard" },
      agentProfileId: "agent",
      output: { targetBucket: "bucket", artifactTypes: [] },
      progress: { currentIteration: 5, status: "running" },
      createdAt: "2024-01-15T10:00:00Z",
    };
    expect(isProcessTerminal(process)).toBe(false);
  });
});

describe("isProcessPaused", () => {
  it("should return true for paused process", () => {
    const process: ResearchProcess = {
      id: "1",
      name: "Test",
      brief: { question: "Test?", constraints: [] },
      depth: { type: "preset", preset: "standard" },
      agentProfileId: "agent",
      output: { targetBucket: "bucket", artifactTypes: [] },
      progress: { currentIteration: 5, status: "paused" },
      createdAt: "2024-01-15T10:00:00Z",
    };
    expect(isProcessPaused(process)).toBe(true);
  });

  it("should return false for running process", () => {
    const process: ResearchProcess = {
      id: "1",
      name: "Test",
      brief: { question: "Test?", constraints: [] },
      depth: { type: "preset", preset: "standard" },
      agentProfileId: "agent",
      output: { targetBucket: "bucket", artifactTypes: [] },
      progress: { currentIteration: 5, status: "running" },
      createdAt: "2024-01-15T10:00:00Z",
    };
    expect(isProcessPaused(process)).toBe(false);
  });
});

// ============================================
// Research Preset Info Tests
// ============================================

describe("ResearchPresetInfoSchema", () => {
  it("should validate preset info", () => {
    const info = {
      preset: "standard" as const,
      name: "Standard",
      description: "Thorough investigation",
      config: RESEARCH_PRESETS["standard"],
    };
    expect(ResearchPresetInfoSchema.parse(info)).toEqual(info);
  });
});

describe("RESEARCH_PRESET_INFO", () => {
  it("should have 4 preset infos", () => {
    expect(RESEARCH_PRESET_INFO).toHaveLength(4);
  });

  it("should have info for quick-scan", () => {
    const info = RESEARCH_PRESET_INFO.find((i) => i.preset === "quick-scan");
    expect(info).toBeDefined();
    expect(info?.name).toBe("Quick Scan");
    expect(info?.config).toEqual(RESEARCH_PRESETS["quick-scan"]);
  });

  it("should have info for standard", () => {
    const info = RESEARCH_PRESET_INFO.find((i) => i.preset === "standard");
    expect(info).toBeDefined();
    expect(info?.name).toBe("Standard");
  });

  it("should have info for deep-dive", () => {
    const info = RESEARCH_PRESET_INFO.find((i) => i.preset === "deep-dive");
    expect(info).toBeDefined();
    expect(info?.name).toBe("Deep Dive");
  });

  it("should have info for exhaustive", () => {
    const info = RESEARCH_PRESET_INFO.find((i) => i.preset === "exhaustive");
    expect(info).toBeDefined();
    expect(info?.name).toBe("Exhaustive");
  });
});

describe("getPresetInfo", () => {
  it("should return info for valid preset", () => {
    const info = getPresetInfo("standard");
    expect(info).toBeDefined();
    expect(info?.preset).toBe("standard");
    expect(info?.name).toBe("Standard");
  });

  it("should return undefined for invalid preset", () => {
    // @ts-expect-error testing invalid input
    const info = getPresetInfo("invalid");
    expect(info).toBeUndefined();
  });
});

// ============================================
// Parsing Helpers Tests
// ============================================

describe("parseResearchProcess", () => {
  it("should parse valid process", () => {
    const data = {
      id: "1",
      name: "Test",
      brief: { question: "Test?" },
      depth: { type: "preset", preset: "standard" },
      agentProfileId: "agent",
      output: { targetBucket: "bucket" },
      progress: {},
      createdAt: "2024-01-15T10:00:00Z",
    };
    const result = parseResearchProcess(data);
    expect(result.id).toBe("1");
    expect(result.name).toBe("Test");
  });

  it("should throw for invalid data", () => {
    expect(() => parseResearchProcess({})).toThrow();
  });
});

describe("safeParseResearchProcess", () => {
  it("should return process for valid data", () => {
    const data = {
      id: "1",
      name: "Test",
      brief: { question: "Test?" },
      depth: { type: "preset", preset: "standard" },
      agentProfileId: "agent",
      output: { targetBucket: "bucket" },
      progress: {},
      createdAt: "2024-01-15T10:00:00Z",
    };
    const result = safeParseResearchProcess(data);
    expect(result).not.toBeNull();
    expect(result?.id).toBe("1");
  });

  it("should return null for invalid data", () => {
    const result = safeParseResearchProcess({});
    expect(result).toBeNull();
  });
});

describe("parseResearchBrief", () => {
  it("should parse valid brief", () => {
    const data = { question: "What framework?" };
    const result = parseResearchBrief(data);
    expect(result.question).toBe("What framework?");
  });

  it("should throw for invalid data", () => {
    expect(() => parseResearchBrief({ question: "" })).toThrow();
  });
});

describe("safeParseResearchBrief", () => {
  it("should return brief for valid data", () => {
    const result = safeParseResearchBrief({ question: "Test?" });
    expect(result).not.toBeNull();
    expect(result?.question).toBe("Test?");
  });

  it("should return null for invalid data", () => {
    const result = safeParseResearchBrief({ question: "" });
    expect(result).toBeNull();
  });
});

describe("parseResearchDepth", () => {
  it("should parse preset depth", () => {
    const result = parseResearchDepth({ type: "preset", preset: "standard" });
    expect(result.type).toBe("preset");
  });

  it("should parse custom depth", () => {
    const result = parseResearchDepth({
      type: "custom",
      config: { maxIterations: 100, timeoutHours: 4, checkpointInterval: 20 },
    });
    expect(result.type).toBe("custom");
  });

  it("should throw for invalid data", () => {
    expect(() => parseResearchDepth({ type: "invalid" })).toThrow();
  });
});

describe("safeParseResearchDepth", () => {
  it("should return depth for valid preset", () => {
    const result = safeParseResearchDepth({ type: "preset", preset: "deep-dive" });
    expect(result).not.toBeNull();
    expect(result?.type).toBe("preset");
  });

  it("should return null for invalid data", () => {
    const result = safeParseResearchDepth({ type: "invalid" });
    expect(result).toBeNull();
  });
});
