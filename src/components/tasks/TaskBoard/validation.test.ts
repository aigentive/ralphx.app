/**
 * Tests for drag-drop validation logic
 */

import { describe, it, expect } from "vitest";
import { createMockTask } from "@/test/mock-data";
import { validateDrop, type ValidationResult } from "./validation";

describe("validateDrop", () => {
  describe("source column restrictions", () => {
    it("should block drag out of In Progress column", () => {
      const task = createMockTask({ internalStatus: "executing" });
      const result = validateDrop(task, "in_progress", "backlog");
      expect(result.valid).toBe(false);
      expect(result.error).toContain("cannot be moved");
    });

    it("should block drag out of In Review column", () => {
      const task = createMockTask({ internalStatus: "pending_review" });
      const result = validateDrop(task, "in_review", "backlog");
      expect(result.valid).toBe(false);
      expect(result.error).toContain("cannot be moved");
    });

    it("should block drag within Done column", () => {
      const task = createMockTask({ internalStatus: "approved" });
      const result = validateDrop(task, "done", "done");
      expect(result.valid).toBe(false);
      expect(result.error).toContain("Cannot reorder");
    });
  });

  describe("target column restrictions", () => {
    it("should block drag to Done column", () => {
      const task = createMockTask({ internalStatus: "ready" });
      const result = validateDrop(task, "todo", "done");
      expect(result.valid).toBe(false);
      expect(result.error).toContain("manually complete");
    });

    it("should block drag to In Progress column", () => {
      const task = createMockTask({ internalStatus: "ready" });
      const result = validateDrop(task, "todo", "in_progress");
      expect(result.valid).toBe(false);
      expect(result.error).toContain("system-managed");
    });

    it("should block drag to In Review column", () => {
      const task = createMockTask({ internalStatus: "ready" });
      const result = validateDrop(task, "todo", "in_review");
      expect(result.valid).toBe(false);
      expect(result.error).toContain("system-managed");
    });
  });

  describe("Planned column validation", () => {
    it("should require title for Planned column", () => {
      const task = createMockTask({ title: "", description: "desc" });
      const result = validateDrop(task, "backlog", "planned");
      expect(result.valid).toBe(false);
      expect(result.error).toContain("title");
    });

    it("should require description for Planned column", () => {
      const task = createMockTask({ title: "Title", description: null });
      const result = validateDrop(task, "backlog", "planned");
      expect(result.valid).toBe(false);
      expect(result.error).toContain("description");
    });

    it("should allow valid task to Planned column", () => {
      const task = createMockTask({ title: "Title", description: "Description" });
      const result = validateDrop(task, "backlog", "planned");
      expect(result.valid).toBe(true);
    });
  });

  describe("allowed transitions", () => {
    it("should allow drag within same column for reordering", () => {
      const task = createMockTask({ internalStatus: "backlog" });
      const result = validateDrop(task, "backlog", "backlog");
      expect(result.valid).toBe(true);
    });

    it("should allow drag from Backlog to Todo", () => {
      const task = createMockTask({ internalStatus: "backlog" });
      const result = validateDrop(task, "backlog", "todo");
      expect(result.valid).toBe(true);
    });

    it("should allow drag from Todo to Planned", () => {
      const task = createMockTask({ title: "T", description: "D", internalStatus: "ready" });
      const result = validateDrop(task, "todo", "planned");
      expect(result.valid).toBe(true);
    });

    it("should allow drag from Planned back to Backlog", () => {
      const task = createMockTask({ internalStatus: "ready" });
      const result = validateDrop(task, "planned", "backlog");
      expect(result.valid).toBe(true);
    });

    it("should allow drag from Draft to any allowed column", () => {
      const task = createMockTask({ title: "T", description: "D" });
      expect(validateDrop(task, "draft", "backlog").valid).toBe(true);
      expect(validateDrop(task, "draft", "todo").valid).toBe(true);
      expect(validateDrop(task, "draft", "planned").valid).toBe(true);
    });
  });
});
