/**
 * running-processes transform tests — snake_case → camelCase conversion
 */

import { describe, it, expect } from "vitest";
import {
  transformTeammateSummary,
  transformRunningProcess,
} from "./running-processes.transforms";

describe("transformTeammateSummary", () => {
  it("transforms required fields only (name + status)", () => {
    const raw = { name: "coder-1", status: "running" };
    const result = transformTeammateSummary(raw);

    expect(result.name).toBe("coder-1");
    expect(result.status).toBe("running");
    expect(result).not.toHaveProperty("step");
    expect(result).not.toHaveProperty("model");
    expect(result).not.toHaveProperty("color");
    expect(result).not.toHaveProperty("stepsCompleted");
    expect(result).not.toHaveProperty("stepsTotal");
    expect(result).not.toHaveProperty("wave");
  });

  it("transforms all optional fields when present", () => {
    const raw = {
      name: "coder-2",
      status: "idle",
      step: "Implement auth",
      model: "sonnet",
      color: "#3b82f6",
      steps_completed: 3,
      steps_total: 8,
      wave: 2,
    };
    const result = transformTeammateSummary(raw);

    expect(result.name).toBe("coder-2");
    expect(result.step).toBe("Implement auth");
    expect(result.model).toBe("sonnet");
    expect(result.color).toBe("#3b82f6");
    expect(result.stepsCompleted).toBe(3);
    expect(result.stepsTotal).toBe(8);
    expect(result.wave).toBe(2);
  });

  it("renames stepsCompleted/stepsTotal/wave from snake_case", () => {
    const raw = {
      name: "coder-3",
      status: "running",
      steps_completed: 0,
      steps_total: 5,
      wave: 1,
    };
    const result = transformTeammateSummary(raw);

    expect(result.stepsCompleted).toBe(0);
    expect(result.stepsTotal).toBe(5);
    expect(result.wave).toBe(1);
    // Verify snake_case keys are NOT present in output
    expect(result).not.toHaveProperty("steps_completed");
    expect(result).not.toHaveProperty("steps_total");
  });
});

describe("transformRunningProcess", () => {
  const baseRaw = {
    task_id: "task-1",
    title: "Auth feature",
    internal_status: "executing",
    step_progress: null,
    elapsed_seconds: 120,
    trigger_origin: "user",
    task_branch: "task/auth-feature",
  };

  it("transforms team fields when present", () => {
    const raw = {
      ...baseRaw,
      team_name: "auth-team",
      teammates: [
        { name: "coder-1", status: "running" },
        { name: "coder-2", status: "idle" },
      ],
      current_wave: 1,
      total_waves: 3,
    };
    const result = transformRunningProcess(raw);

    expect(result.teamName).toBe("auth-team");
    expect(result.teammates).toHaveLength(2);
    expect(result.teammates![0]!.name).toBe("coder-1");
    expect(result.currentWave).toBe(1);
    expect(result.totalWaves).toBe(3);
  });

  it("omits team fields when not in raw data", () => {
    const result = transformRunningProcess(baseRaw);

    expect(result.taskId).toBe("task-1");
    expect(result.internalStatus).toBe("executing");
    expect(result).not.toHaveProperty("teamName");
    expect(result).not.toHaveProperty("teammates");
    expect(result).not.toHaveProperty("currentWave");
    expect(result).not.toHaveProperty("totalWaves");
  });

  it("handles empty teammates array", () => {
    const raw = {
      ...baseRaw,
      team_name: "empty-team",
      teammates: [],
      current_wave: 0,
      total_waves: 0,
    };
    const result = transformRunningProcess(raw);

    expect(result.teamName).toBe("empty-team");
    expect(result.teammates).toEqual([]);
    expect(result.currentWave).toBe(0);
    expect(result.totalWaves).toBe(0);
  });
});
