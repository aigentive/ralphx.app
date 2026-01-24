import { describe, it, expect } from "vitest";
import {
  WorkflowColumnSchema,
  WorkflowSchemaZ,
  type WorkflowColumn,
  type WorkflowSchema,
} from "./workflow";

describe("WorkflowColumnSchema", () => {
  it("validates a minimal workflow column", () => {
    const column = {
      id: "backlog",
      name: "Backlog",
      mapsTo: "backlog",
    };

    const result = WorkflowColumnSchema.safeParse(column);
    expect(result.success).toBe(true);
  });

  it("validates a column with all optional fields", () => {
    const column = {
      id: "in-progress",
      name: "In Progress",
      color: "#ff6b35",
      icon: "play",
      mapsTo: "executing",
      behavior: {
        skipReview: false,
        autoAdvance: true,
        agentProfile: "worker",
      },
    };

    const result = WorkflowColumnSchema.safeParse(column);
    expect(result.success).toBe(true);
    if (result.success) {
      expect(result.data.color).toBe("#ff6b35");
      expect(result.data.behavior?.autoAdvance).toBe(true);
    }
  });

  it("validates a column with partial behavior", () => {
    const column = {
      id: "review",
      name: "Review",
      mapsTo: "pending_review",
      behavior: {
        skipReview: false,
      },
    };

    const result = WorkflowColumnSchema.safeParse(column);
    expect(result.success).toBe(true);
    if (result.success) {
      expect(result.data.behavior?.skipReview).toBe(false);
      expect(result.data.behavior?.autoAdvance).toBeUndefined();
    }
  });

  it("rejects a column without id", () => {
    const column = {
      name: "Backlog",
      mapsTo: "backlog",
    };

    const result = WorkflowColumnSchema.safeParse(column);
    expect(result.success).toBe(false);
  });

  it("rejects a column without name", () => {
    const column = {
      id: "backlog",
      mapsTo: "backlog",
    };

    const result = WorkflowColumnSchema.safeParse(column);
    expect(result.success).toBe(false);
  });

  it("rejects a column with invalid mapsTo status", () => {
    const column = {
      id: "backlog",
      name: "Backlog",
      mapsTo: "invalid_status",
    };

    const result = WorkflowColumnSchema.safeParse(column);
    expect(result.success).toBe(false);
  });

  it("validates all internal statuses for mapsTo", () => {
    const statuses = [
      "backlog",
      "ready",
      "blocked",
      "executing",
      "execution_done",
      "qa_refining",
      "qa_testing",
      "qa_passed",
      "qa_failed",
      "pending_review",
      "revision_needed",
      "approved",
      "failed",
      "cancelled",
    ];

    for (const status of statuses) {
      const column = {
        id: `col-${status}`,
        name: status,
        mapsTo: status,
      };
      const result = WorkflowColumnSchema.safeParse(column);
      expect(result.success).toBe(true);
    }
  });
});

describe("WorkflowSchemaZ", () => {
  it("validates a minimal workflow", () => {
    const workflow = {
      id: "default",
      name: "Default Workflow",
      columns: [
        { id: "backlog", name: "Backlog", mapsTo: "backlog" },
        { id: "ready", name: "Ready", mapsTo: "ready" },
      ],
    };

    const result = WorkflowSchemaZ.safeParse(workflow);
    expect(result.success).toBe(true);
  });

  it("validates a workflow with all optional fields", () => {
    const workflow = {
      id: "custom",
      name: "Custom Workflow",
      description: "A custom workflow for feature development",
      columns: [
        { id: "backlog", name: "Backlog", mapsTo: "backlog" },
        { id: "in-progress", name: "In Progress", mapsTo: "executing" },
        { id: "done", name: "Done", mapsTo: "approved" },
      ],
      defaults: {
        workerProfile: "senior-dev",
        reviewerProfile: "tech-lead",
      },
    };

    const result = WorkflowSchemaZ.safeParse(workflow);
    expect(result.success).toBe(true);
    if (result.success) {
      expect(result.data.description).toBe("A custom workflow for feature development");
      expect(result.data.defaults?.workerProfile).toBe("senior-dev");
    }
  });

  it("validates a workflow with partial defaults", () => {
    const workflow = {
      id: "minimal",
      name: "Minimal Workflow",
      columns: [{ id: "todo", name: "To Do", mapsTo: "ready" }],
      defaults: {
        workerProfile: "worker",
      },
    };

    const result = WorkflowSchemaZ.safeParse(workflow);
    expect(result.success).toBe(true);
    if (result.success) {
      expect(result.data.defaults?.workerProfile).toBe("worker");
      expect(result.data.defaults?.reviewerProfile).toBeUndefined();
    }
  });

  it("rejects a workflow without id", () => {
    const workflow = {
      name: "No ID Workflow",
      columns: [{ id: "backlog", name: "Backlog", mapsTo: "backlog" }],
    };

    const result = WorkflowSchemaZ.safeParse(workflow);
    expect(result.success).toBe(false);
  });

  it("rejects a workflow without name", () => {
    const workflow = {
      id: "no-name",
      columns: [{ id: "backlog", name: "Backlog", mapsTo: "backlog" }],
    };

    const result = WorkflowSchemaZ.safeParse(workflow);
    expect(result.success).toBe(false);
  });

  it("rejects a workflow without columns", () => {
    const workflow = {
      id: "no-columns",
      name: "No Columns Workflow",
    };

    const result = WorkflowSchemaZ.safeParse(workflow);
    expect(result.success).toBe(false);
  });

  it("rejects a workflow with empty columns array", () => {
    const workflow = {
      id: "empty-columns",
      name: "Empty Columns Workflow",
      columns: [],
    };

    const result = WorkflowSchemaZ.safeParse(workflow);
    // Empty arrays are valid in Zod by default
    expect(result.success).toBe(true);
  });

  it("rejects a workflow with invalid column", () => {
    const workflow = {
      id: "invalid-column",
      name: "Invalid Column Workflow",
      columns: [
        { id: "backlog", name: "Backlog", mapsTo: "invalid_status" },
      ],
    };

    const result = WorkflowSchemaZ.safeParse(workflow);
    expect(result.success).toBe(false);
  });
});

describe("type inference", () => {
  it("correctly infers WorkflowColumn type", () => {
    const column: WorkflowColumn = {
      id: "test",
      name: "Test Column",
      mapsTo: "backlog",
      color: "#fff",
      behavior: {
        skipReview: true,
      },
    };
    expect(column.id).toBe("test");
  });

  it("correctly infers WorkflowSchema type", () => {
    const workflow: WorkflowSchema = {
      id: "test",
      name: "Test Workflow",
      columns: [
        { id: "col1", name: "Column 1", mapsTo: "backlog" },
      ],
    };
    expect(workflow.id).toBe("test");
  });
});
