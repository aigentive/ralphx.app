import { describe, expect, it } from "vitest";
import type { TaskGraphNode } from "@/api/task-graph.types";
import { applyTaskSyncEvent, createEngineState, stepState } from "./engine";

function makeTask(taskId: string, internalStatus: string): TaskGraphNode {
  return {
    taskId,
    title: `Task ${taskId}`,
    description: null,
    category: "dev",
    internalStatus,
    priority: 1,
    inDegree: 0,
    outDegree: 0,
    tier: 1,
    planArtifactId: null,
    sourceProposalId: null,
  };
}

describe("battle-v2 engine", () => {
  it("removes completed task on sync event and adds score", () => {
    const state = createEngineState([makeTask("a1", "executing")], 900);

    applyTaskSyncEvent(state, {
      taskId: "a1",
      fromStatus: "executing",
      toStatus: "merged",
      timestamp: Date.now(),
      source: "task:event",
    }, 900);

    expect(state.tasks.has("a1")).toBe(false);
    expect(state.score).toBeGreaterThan(0);
    expect(state.combo).toBeGreaterThan(0);
  });

  it("spawns a mini boss when pressure is high enough over time", () => {
    const tasks = [
      makeTask("t1", "executing"),
      makeTask("t2", "reviewing"),
      makeTask("t3", "merging"),
    ];
    const state = createEngineState(tasks, 1000);

    // Force director interval and cooldown to pass
    state.lastDirectorTick = Date.now() - 5_000;
    state.bossCooldownUntil = Date.now() - 1;

    stepState(
      state,
      0.016,
      Date.now(),
      1000,
      700,
      { left: false, right: false, firing: false, ability: false },
      5,
      3
    );

    const hasBoss = Array.from(state.entities.values()).some((e) => e.kind === "miniBoss");
    expect(hasBoss).toBe(true);
  });

  it("uses ability when charge is full", () => {
    const state = createEngineState([makeTask("a1", "reviewing")], 900);
    state.abilityCharge = 100;

    stepState(
      state,
      0.016,
      Date.now(),
      900,
      600,
      { left: false, right: false, firing: false, ability: true },
      1,
      0
    );

    expect(state.abilityCharge).toBe(0);
    expect(state.focusActiveUntil).toBeGreaterThan(Date.now());
  });

  it("suppresses breached task threats instead of deleting task state", () => {
    const state = createEngineState([makeTask("a1", "executing")], 900);
    const entity = Array.from(state.entities.values())[0];
    expect(entity).toBeDefined();
    if (!entity) return;
    entity.y = 540;

    const now = Date.now();
    stepState(
      state,
      0.016,
      now,
      900,
      600,
      { left: false, right: false, firing: false, ability: false },
      1,
      0
    );

    const task = state.tasks.get("a1");
    expect(task).toBeDefined();
    expect(task?.suppressedUntil ?? 0).toBeGreaterThan(now);
  });
});
