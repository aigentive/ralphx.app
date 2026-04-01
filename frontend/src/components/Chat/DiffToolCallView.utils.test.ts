/**
 * DiffToolCallView.utils tests
 *
 * Tests for isTaskToolCall() and isDiffToolCall() predicates
 * which drive the ToolCallIndicator routing logic.
 */

import { describe, it, expect } from "vitest";
import {
  isTaskToolCall,
  isDiffToolCall,
} from "./DiffToolCallView.utils";

describe("isTaskToolCall", () => {
  describe("Task tool names", () => {
    it("returns true for 'Task' (capitalized)", () => {
      expect(isTaskToolCall("Task")).toBe(true);
    });

    it("returns true for 'task' (lowercase)", () => {
      expect(isTaskToolCall("task")).toBe(true);
    });

    it("returns true for 'TASK' (uppercase)", () => {
      expect(isTaskToolCall("TASK")).toBe(true);
    });

    it("returns true for 'TaSk' (mixed case)", () => {
      expect(isTaskToolCall("TaSk")).toBe(true);
    });
  });

  describe("Agent tool names (extended support)", () => {
    it("returns true for 'Agent' (capitalized)", () => {
      expect(isTaskToolCall("Agent")).toBe(true);
    });

    it("returns true for 'agent' (lowercase)", () => {
      expect(isTaskToolCall("agent")).toBe(true);
    });

    it("returns true for 'AGENT' (uppercase)", () => {
      expect(isTaskToolCall("AGENT")).toBe(true);
    });

    it("returns true for 'aGeNt' (mixed case)", () => {
      expect(isTaskToolCall("aGeNt")).toBe(true);
    });
  });

  describe("Non-subagent tool names", () => {
    it("returns false for 'Edit'", () => {
      expect(isTaskToolCall("Edit")).toBe(false);
    });

    it("returns false for 'Write'", () => {
      expect(isTaskToolCall("Write")).toBe(false);
    });

    it("returns false for 'Read'", () => {
      expect(isTaskToolCall("Read")).toBe(false);
    });

    it("returns false for 'Bash'", () => {
      expect(isTaskToolCall("Bash")).toBe(false);
    });

    it("returns false for 'Glob'", () => {
      expect(isTaskToolCall("Glob")).toBe(false);
    });

    it("returns false for 'Grep'", () => {
      expect(isTaskToolCall("Grep")).toBe(false);
    });

    it("returns false for empty string", () => {
      expect(isTaskToolCall("")).toBe(false);
    });

    it("returns false for 'update_task'", () => {
      expect(isTaskToolCall("update_task")).toBe(false);
    });
  });
});

describe("isDiffToolCall", () => {
  describe("Diff tool names", () => {
    it("returns true for 'Edit'", () => {
      expect(isDiffToolCall("Edit")).toBe(true);
    });

    it("returns true for 'edit' (lowercase)", () => {
      expect(isDiffToolCall("edit")).toBe(true);
    });

    it("returns true for 'EDIT' (uppercase)", () => {
      expect(isDiffToolCall("EDIT")).toBe(true);
    });

    it("returns true for 'Write'", () => {
      expect(isDiffToolCall("Write")).toBe(true);
    });

    it("returns true for 'write' (lowercase)", () => {
      expect(isDiffToolCall("write")).toBe(true);
    });

    it("returns true for 'WRITE' (uppercase)", () => {
      expect(isDiffToolCall("WRITE")).toBe(true);
    });
  });

  describe("Non-diff tool names", () => {
    it("returns false for 'Task'", () => {
      expect(isDiffToolCall("Task")).toBe(false);
    });

    it("returns false for 'Agent'", () => {
      expect(isDiffToolCall("Agent")).toBe(false);
    });

    it("returns false for 'Read'", () => {
      expect(isDiffToolCall("Read")).toBe(false);
    });

    it("returns false for 'Bash'", () => {
      expect(isDiffToolCall("Bash")).toBe(false);
    });

    it("returns false for empty string", () => {
      expect(isDiffToolCall("")).toBe(false);
    });
  });
});
