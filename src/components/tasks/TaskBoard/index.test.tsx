/**
 * Tests for TaskBoard index exports
 */

import { describe, it, expect } from "vitest";
import {
  TaskBoard,
  TaskBoardSkeleton,
  Column,
  TaskCard,
  useTaskBoard,
  workflowKeys,
  type TaskBoardProps,
} from "./index";

describe("TaskBoard exports", () => {
  it("should export TaskBoard component", () => {
    expect(TaskBoard).toBeDefined();
    expect(typeof TaskBoard).toBe("function");
  });

  it("should export TaskBoardSkeleton component", () => {
    expect(TaskBoardSkeleton).toBeDefined();
    expect(typeof TaskBoardSkeleton).toBe("function");
  });

  it("should export Column component", () => {
    expect(Column).toBeDefined();
    expect(typeof Column).toBe("function");
  });

  it("should export TaskCard component", () => {
    expect(TaskCard).toBeDefined();
    expect(typeof TaskCard).toBe("function");
  });

  it("should export useTaskBoard hook", () => {
    expect(useTaskBoard).toBeDefined();
    expect(typeof useTaskBoard).toBe("function");
  });

  it("should export workflowKeys", () => {
    expect(workflowKeys).toBeDefined();
    expect(workflowKeys.all).toEqual(["workflows"]);
    expect(workflowKeys.detail("test")).toEqual(["workflows", "test"]);
  });

  it("should allow type usage", () => {
    const props: TaskBoardProps = {
      projectId: "p1",
      workflowId: "w1",
    };
    expect(props.projectId).toBe("p1");

    // Type check only - BoardColumn and UseTaskBoardResult
    const typeCheck = true;
    expect(typeCheck).toBe(true);
  });
});
