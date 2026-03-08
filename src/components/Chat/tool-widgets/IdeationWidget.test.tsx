import { describe, it, expect } from "vitest";
import { render, screen } from "@testing-library/react";
import { IdeationWidget } from "./IdeationWidget";
import type { ToolCall } from "./shared.constants";

function makeToolCall(name: string, overrides: Partial<ToolCall> = {}): ToolCall {
  return {
    id: "ideation-1",
    name,
    arguments: {},
    ...overrides,
  };
}

function mcpWrap(obj: unknown): unknown {
  return [{ type: "text", text: JSON.stringify(obj) }];
}

describe("IdeationWidget", () => {
  describe("PlanCreated (create_plan_artifact)", () => {
    it("extracts name from MCP-wrapped result", () => {
      const toolCall = makeToolCall("create_plan_artifact", {
        result: mcpWrap({ name: "My New Plan", version: 1 }),
      });
      render(<IdeationWidget toolCall={toolCall} />);
      expect(screen.getByText("My New Plan")).toBeInTheDocument();
      expect(screen.getByText("Plan created")).toBeInTheDocument();
    });

    it("falls back to arguments.title when result has no name", () => {
      const toolCall = makeToolCall("create_plan_artifact", {
        arguments: { title: "From Args" },
        result: mcpWrap({ version: 1 }),
      });
      render(<IdeationWidget toolCall={toolCall} />);
      expect(screen.getByText("From Args")).toBeInTheDocument();
    });

    it("falls back to 'Plan' when result and args both missing name", () => {
      const toolCall = makeToolCall("create_plan_artifact", {
        result: mcpWrap({}),
      });
      render(<IdeationWidget toolCall={toolCall} />);
      expect(screen.getByText("Plan")).toBeInTheDocument();
    });

    it("extracts name from plain object result (passthrough)", () => {
      const toolCall = makeToolCall("create_plan_artifact", {
        result: { name: "Plain Object Plan" },
      });
      render(<IdeationWidget toolCall={toolCall} />);
      expect(screen.getByText("Plain Object Plan")).toBeInTheDocument();
    });
  });

  describe("GetProposal (get_proposal)", () => {
    it("extracts title and category from MCP-wrapped result", () => {
      const toolCall = makeToolCall("get_proposal", {
        result: mcpWrap({ title: "Add Auth", category: "feature" }),
      });
      render(<IdeationWidget toolCall={toolCall} />);
      expect(screen.getByText("Add Auth")).toBeInTheDocument();
      expect(screen.getByText("feature")).toBeInTheDocument();
    });

    it("shows 'Loading proposal...' when result has no title", () => {
      const toolCall = makeToolCall("get_proposal", {
        result: mcpWrap({}),
      });
      render(<IdeationWidget toolCall={toolCall} />);
      expect(screen.getByText("Loading proposal...")).toBeInTheDocument();
    });

    it("extracts title from plain object result", () => {
      const toolCall = makeToolCall("get_proposal", {
        result: { title: "Fix Bug", category: "bug" },
      });
      render(<IdeationWidget toolCall={toolCall} />);
      expect(screen.getByText("Fix Bug")).toBeInTheDocument();
    });
  });

  describe("GetSessionPlan (get_session_plan)", () => {
    it("extracts name and version from MCP-wrapped result", () => {
      const toolCall = makeToolCall("get_session_plan", {
        result: mcpWrap({ name: "Session Plan Alpha", version: 3 }),
      });
      render(<IdeationWidget toolCall={toolCall} />);
      expect(screen.getByText("Session Plan Alpha")).toBeInTheDocument();
      expect(screen.getByText("v3")).toBeInTheDocument();
    });

    it("shows 'No plan artifact' when result has no name", () => {
      const toolCall = makeToolCall("get_session_plan", {
        result: mcpWrap({}),
      });
      render(<IdeationWidget toolCall={toolCall} />);
      expect(screen.getByText("No plan artifact")).toBeInTheDocument();
    });

    it("extracts name from plain object result", () => {
      const toolCall = makeToolCall("get_session_plan", {
        result: { name: "Plain Plan", version: 2 },
      });
      render(<IdeationWidget toolCall={toolCall} />);
      expect(screen.getByText("Plain Plan")).toBeInTheDocument();
      expect(screen.getByText("v2")).toBeInTheDocument();
    });
  });
});
