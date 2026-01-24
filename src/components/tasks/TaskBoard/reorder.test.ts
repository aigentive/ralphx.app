/**
 * Tests for priority reordering logic
 */

import { describe, it, expect } from "vitest";
import { createMockTask } from "@/test/mock-data";
import { calculateNewPriority, reorderTasks } from "./reorder";

describe("calculateNewPriority", () => {
  it("should return 0 for first position", () => {
    const tasks = [
      createMockTask({ id: "t1", priority: 0 }),
      createMockTask({ id: "t2", priority: 1 }),
    ];
    const result = calculateNewPriority(tasks, 0);
    expect(result).toBe(0);
  });

  it("should return priority between neighbors", () => {
    const tasks = [
      createMockTask({ id: "t1", priority: 0 }),
      createMockTask({ id: "t2", priority: 2 }),
    ];
    const result = calculateNewPriority(tasks, 1);
    expect(result).toBe(1);
  });

  it("should return max priority + 1 for last position", () => {
    const tasks = [
      createMockTask({ id: "t1", priority: 0 }),
      createMockTask({ id: "t2", priority: 1 }),
    ];
    const result = calculateNewPriority(tasks, 2);
    expect(result).toBe(2);
  });

  it("should handle empty array", () => {
    const result = calculateNewPriority([], 0);
    expect(result).toBe(0);
  });
});

describe("reorderTasks", () => {
  it("should move task from higher to lower index", () => {
    const tasks = [
      createMockTask({ id: "t1" }),
      createMockTask({ id: "t2" }),
      createMockTask({ id: "t3" }),
    ];
    const result = reorderTasks(tasks, 2, 0);
    expect(result.map(t => t.id)).toEqual(["t3", "t1", "t2"]);
  });

  it("should move task from lower to higher index", () => {
    const tasks = [
      createMockTask({ id: "t1" }),
      createMockTask({ id: "t2" }),
      createMockTask({ id: "t3" }),
    ];
    const result = reorderTasks(tasks, 0, 2);
    expect(result.map(t => t.id)).toEqual(["t2", "t3", "t1"]);
  });

  it("should return same order if indices are equal", () => {
    const tasks = [
      createMockTask({ id: "t1" }),
      createMockTask({ id: "t2" }),
    ];
    const result = reorderTasks(tasks, 0, 0);
    expect(result.map(t => t.id)).toEqual(["t1", "t2"]);
  });

  it("should update priorities based on new positions", () => {
    const tasks = [
      createMockTask({ id: "t1", priority: 0 }),
      createMockTask({ id: "t2", priority: 1 }),
      createMockTask({ id: "t3", priority: 2 }),
    ];
    const result = reorderTasks(tasks, 2, 0);
    expect(result[0]?.priority).toBe(0);
    expect(result[1]?.priority).toBe(1);
    expect(result[2]?.priority).toBe(2);
  });
});
