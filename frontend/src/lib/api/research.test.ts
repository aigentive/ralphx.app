import { describe, it, expect, vi, beforeEach } from "vitest";
import { invoke } from "@tauri-apps/api/core";
import {
  startResearch,
  pauseResearch,
  resumeResearch,
  stopResearch,
  getResearchProcesses,
  getResearchProcess,
  getResearchPresets,
  ResearchProcessResponseSchema,
  ResearchPresetResponseSchema,
  StartResearchInputSchema,
  CustomDepthInputSchema,
} from "./research";

// Mock Tauri invoke
vi.mock("@tauri-apps/api/core", () => ({
  invoke: vi.fn(),
}));

const mockInvoke = invoke as ReturnType<typeof vi.fn>;

// Test data helpers
const createMockResearchProcess = (overrides = {}) => ({
  id: "process-1",
  name: "Test Research",
  question: "What architecture should we use?",
  context: null,
  scope: null,
  constraints: [],
  agent_profile_id: "deep-researcher",
  depth_preset: "standard",
  max_iterations: 50,
  timeout_hours: 2,
  checkpoint_interval: 10,
  target_bucket: "research-outputs",
  status: "pending",
  current_iteration: 0,
  progress_percentage: 0,
  error_message: null,
  created_at: "2026-01-24T12:00:00Z",
  started_at: null,
  completed_at: null,
  ...overrides,
});

const createMockPreset = (overrides = {}) => ({
  id: "standard",
  name: "Standard",
  max_iterations: 50,
  timeout_hours: 2,
  checkpoint_interval: 10,
  description: "Thorough investigation - 50 iterations, 2 hrs timeout",
  ...overrides,
});

describe("ResearchProcessResponseSchema", () => {
  it("should parse valid research process response", () => {
    const process = createMockResearchProcess();
    expect(() => ResearchProcessResponseSchema.parse(process)).not.toThrow();
  });

  it("should parse process with all optional fields", () => {
    const process = createMockResearchProcess({
      context: "Building a new app",
      scope: "Backend only",
      constraints: ["No microservices", "Use SQLite"],
      started_at: "2026-01-24T12:01:00Z",
      completed_at: "2026-01-24T14:00:00Z",
    });
    const result = ResearchProcessResponseSchema.parse(process);
    expect(result.context).toBe("Building a new app");
    expect(result.constraints).toHaveLength(2);
  });

  it("should parse running process with progress", () => {
    const process = createMockResearchProcess({
      status: "running",
      current_iteration: 25,
      progress_percentage: 50,
      started_at: "2026-01-24T12:01:00Z",
    });
    const result = ResearchProcessResponseSchema.parse(process);
    expect(result.status).toBe("running");
    expect(result.current_iteration).toBe(25);
    expect(result.progress_percentage).toBe(50);
  });

  it("should parse failed process with error", () => {
    const process = createMockResearchProcess({
      status: "failed",
      error_message: "Timeout exceeded",
    });
    const result = ResearchProcessResponseSchema.parse(process);
    expect(result.status).toBe("failed");
    expect(result.error_message).toBe("Timeout exceeded");
  });

  it("should parse process without depth_preset (custom depth)", () => {
    const process = createMockResearchProcess({
      depth_preset: null,
      max_iterations: 100,
      timeout_hours: 4,
      checkpoint_interval: 20,
    });
    const result = ResearchProcessResponseSchema.parse(process);
    expect(result.depth_preset).toBeNull();
    expect(result.max_iterations).toBe(100);
  });

  it("should reject process without required fields", () => {
    expect(() => ResearchProcessResponseSchema.parse({})).toThrow();
    expect(() => ResearchProcessResponseSchema.parse({ id: "p1" })).toThrow();
  });

  it("should reject process with invalid status", () => {
    const process = createMockResearchProcess({ status: "invalid_status" });
    expect(() => ResearchProcessResponseSchema.parse(process)).toThrow();
  });
});

describe("ResearchPresetResponseSchema", () => {
  it("should parse valid preset response", () => {
    const preset = createMockPreset();
    expect(() => ResearchPresetResponseSchema.parse(preset)).not.toThrow();
  });

  it("should parse all 4 presets", () => {
    const presets = [
      createMockPreset({ id: "quick-scan", name: "Quick Scan", max_iterations: 10 }),
      createMockPreset({ id: "standard", name: "Standard", max_iterations: 50 }),
      createMockPreset({ id: "deep-dive", name: "Deep Dive", max_iterations: 200 }),
      createMockPreset({ id: "exhaustive", name: "Exhaustive", max_iterations: 500 }),
    ];
    presets.forEach((preset) => {
      expect(() => ResearchPresetResponseSchema.parse(preset)).not.toThrow();
    });
  });

  it("should reject preset without required fields", () => {
    expect(() => ResearchPresetResponseSchema.parse({})).toThrow();
  });
});

describe("StartResearchInputSchema", () => {
  it("should parse valid start input", () => {
    const input = {
      name: "New Research",
      question: "What framework should we use?",
      agent_profile_id: "deep-researcher",
    };
    expect(() => StartResearchInputSchema.parse(input)).not.toThrow();
  });

  it("should parse input with all optional fields", () => {
    const input = {
      name: "Full Research",
      question: "What is the best architecture?",
      context: "Building a SAAS app",
      scope: "Backend services only",
      constraints: ["Use TypeScript", "No microservices"],
      agent_profile_id: "researcher",
      depth_preset: "deep-dive",
      target_bucket: "custom-research",
    };
    const result = StartResearchInputSchema.parse(input);
    expect(result.depth_preset).toBe("deep-dive");
    expect(result.constraints).toHaveLength(2);
  });

  it("should parse input with custom depth", () => {
    const input = {
      name: "Custom Research",
      question: "Question?",
      agent_profile_id: "researcher",
      custom_depth: {
        max_iterations: 100,
        timeout_hours: 4,
        checkpoint_interval: 20,
      },
    };
    const result = StartResearchInputSchema.parse(input);
    expect(result.custom_depth?.max_iterations).toBe(100);
  });

  it("should reject input without required fields", () => {
    expect(() => StartResearchInputSchema.parse({})).toThrow();
    expect(() => StartResearchInputSchema.parse({ name: "Test" })).toThrow();
  });

  it("should reject invalid depth_preset", () => {
    const input = {
      name: "Test",
      question: "Question?",
      agent_profile_id: "researcher",
      depth_preset: "invalid-preset",
    };
    expect(() => StartResearchInputSchema.parse(input)).toThrow();
  });
});

describe("CustomDepthInputSchema", () => {
  it("should parse valid custom depth input", () => {
    const input = {
      max_iterations: 100,
      timeout_hours: 4,
      checkpoint_interval: 20,
    };
    expect(() => CustomDepthInputSchema.parse(input)).not.toThrow();
  });

  it("should reject negative values", () => {
    expect(() =>
      CustomDepthInputSchema.parse({
        max_iterations: -1,
        timeout_hours: 4,
        checkpoint_interval: 20,
      })
    ).toThrow();
  });

  it("should reject zero max_iterations", () => {
    expect(() =>
      CustomDepthInputSchema.parse({
        max_iterations: 0,
        timeout_hours: 4,
        checkpoint_interval: 20,
      })
    ).toThrow();
  });
});

describe("startResearch", () => {
  beforeEach(() => {
    mockInvoke.mockReset();
  });

  it("should call start_research command with input", async () => {
    mockInvoke.mockResolvedValue(createMockResearchProcess({ status: "running" }));
    const input = {
      name: "New Research",
      question: "What framework?",
      agent_profile_id: "researcher",
    };

    await startResearch(input);

    expect(mockInvoke).toHaveBeenCalledWith("start_research", { input });
  });

  it("should return started research process", async () => {
    const started = createMockResearchProcess({
      name: "Started Research",
      status: "running",
      started_at: "2026-01-24T12:00:00Z",
    });
    mockInvoke.mockResolvedValue(started);

    const result = await startResearch({
      name: "Started Research",
      question: "Question?",
      agent_profile_id: "researcher",
    });

    expect(result.status).toBe("running");
    expect(result.started_at).not.toBeNull();
  });

  it("should validate input before sending", async () => {
    const invalidInput = { name: "Test" } as never;

    await expect(startResearch(invalidInput)).rejects.toThrow();
    expect(mockInvoke).not.toHaveBeenCalled();
  });
});

describe("pauseResearch", () => {
  beforeEach(() => {
    mockInvoke.mockReset();
  });

  it("should call pause_research command with id", async () => {
    mockInvoke.mockResolvedValue(createMockResearchProcess({ status: "paused" }));

    await pauseResearch("p-123");

    expect(mockInvoke).toHaveBeenCalledWith("pause_research", { id: "p-123" });
  });

  it("should return paused research process", async () => {
    const paused = createMockResearchProcess({ status: "paused", current_iteration: 25 });
    mockInvoke.mockResolvedValue(paused);

    const result = await pauseResearch("p-123");

    expect(result.status).toBe("paused");
    expect(result.current_iteration).toBe(25);
  });

  it("should propagate backend errors", async () => {
    mockInvoke.mockRejectedValue(new Error("Cannot pause research in status: pending"));

    await expect(pauseResearch("p-123")).rejects.toThrow(
      "Cannot pause research in status: pending"
    );
  });
});

describe("resumeResearch", () => {
  beforeEach(() => {
    mockInvoke.mockReset();
  });

  it("should call resume_research command with id", async () => {
    mockInvoke.mockResolvedValue(createMockResearchProcess({ status: "running" }));

    await resumeResearch("p-123");

    expect(mockInvoke).toHaveBeenCalledWith("resume_research", { id: "p-123" });
  });

  it("should return resumed research process", async () => {
    const resumed = createMockResearchProcess({ status: "running" });
    mockInvoke.mockResolvedValue(resumed);

    const result = await resumeResearch("p-123");

    expect(result.status).toBe("running");
  });

  it("should propagate backend errors", async () => {
    mockInvoke.mockRejectedValue(
      new Error("Cannot resume research in status: running")
    );

    await expect(resumeResearch("p-123")).rejects.toThrow(
      "Cannot resume research in status: running"
    );
  });
});

describe("stopResearch", () => {
  beforeEach(() => {
    mockInvoke.mockReset();
  });

  it("should call stop_research command with id", async () => {
    mockInvoke.mockResolvedValue(
      createMockResearchProcess({ status: "failed", error_message: "Stopped by user" })
    );

    await stopResearch("p-123");

    expect(mockInvoke).toHaveBeenCalledWith("stop_research", { id: "p-123" });
  });

  it("should return stopped research process with error message", async () => {
    const stopped = createMockResearchProcess({
      status: "failed",
      error_message: "Stopped by user",
    });
    mockInvoke.mockResolvedValue(stopped);

    const result = await stopResearch("p-123");

    expect(result.status).toBe("failed");
    expect(result.error_message).toBe("Stopped by user");
  });

  it("should propagate backend errors", async () => {
    mockInvoke.mockRejectedValue(
      new Error("Research process already completed with status: completed")
    );

    await expect(stopResearch("p-123")).rejects.toThrow("already completed");
  });
});

describe("getResearchProcesses", () => {
  beforeEach(() => {
    mockInvoke.mockReset();
  });

  it("should call get_research_processes command without filter", async () => {
    mockInvoke.mockResolvedValue([createMockResearchProcess()]);

    await getResearchProcesses();

    expect(mockInvoke).toHaveBeenCalledWith("get_research_processes", { status: null });
  });

  it("should call get_research_processes with status filter", async () => {
    mockInvoke.mockResolvedValue([createMockResearchProcess({ status: "running" })]);

    await getResearchProcesses("running");

    expect(mockInvoke).toHaveBeenCalledWith("get_research_processes", {
      status: "running",
    });
  });

  it("should return validated array of processes", async () => {
    const processes = [
      createMockResearchProcess({ id: "p1", name: "Research 1" }),
      createMockResearchProcess({ id: "p2", name: "Research 2" }),
    ];
    mockInvoke.mockResolvedValue(processes);

    const result = await getResearchProcesses();

    expect(result).toHaveLength(2);
    expect(result[0]?.name).toBe("Research 1");
  });

  it("should return empty array when no processes", async () => {
    mockInvoke.mockResolvedValue([]);

    const result = await getResearchProcesses();

    expect(result).toEqual([]);
  });

  it("should throw on invalid response", async () => {
    mockInvoke.mockResolvedValue([{ invalid: "process" }]);

    await expect(getResearchProcesses()).rejects.toThrow();
  });
});

describe("getResearchProcess", () => {
  beforeEach(() => {
    mockInvoke.mockReset();
  });

  it("should call get_research_process command with id", async () => {
    mockInvoke.mockResolvedValue(createMockResearchProcess());

    await getResearchProcess("p-123");

    expect(mockInvoke).toHaveBeenCalledWith("get_research_process", { id: "p-123" });
  });

  it("should return null when process not found", async () => {
    mockInvoke.mockResolvedValue(null);

    const result = await getResearchProcess("nonexistent");

    expect(result).toBeNull();
  });

  it("should return validated research process", async () => {
    const process = createMockResearchProcess({ name: "Found Research" });
    mockInvoke.mockResolvedValue(process);

    const result = await getResearchProcess("p-123");

    expect(result?.name).toBe("Found Research");
  });
});

describe("getResearchPresets", () => {
  beforeEach(() => {
    mockInvoke.mockReset();
  });

  it("should call get_research_presets command", async () => {
    mockInvoke.mockResolvedValue([createMockPreset()]);

    await getResearchPresets();

    expect(mockInvoke).toHaveBeenCalledWith("get_research_presets", {});
  });

  it("should return validated array of presets", async () => {
    const presets = [
      createMockPreset({ id: "quick-scan", name: "Quick Scan" }),
      createMockPreset({ id: "standard", name: "Standard" }),
      createMockPreset({ id: "deep-dive", name: "Deep Dive" }),
      createMockPreset({ id: "exhaustive", name: "Exhaustive" }),
    ];
    mockInvoke.mockResolvedValue(presets);

    const result = await getResearchPresets();

    expect(result).toHaveLength(4);
    expect(result.map((p) => p.id)).toContain("standard");
    expect(result.map((p) => p.id)).toContain("deep-dive");
  });

  it("should throw on invalid response", async () => {
    mockInvoke.mockResolvedValue([{ invalid: "preset" }]);

    await expect(getResearchPresets()).rejects.toThrow();
  });
});
