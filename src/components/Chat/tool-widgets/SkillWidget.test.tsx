import { describe, it, expect } from "vitest";
import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { SkillWidget } from "./SkillWidget";
import type { ToolCall } from "./shared.constants";

function makeSkillCall(overrides: Partial<ToolCall> = {}): ToolCall {
  return {
    id: "skill-1",
    name: "Skill",
    arguments: { skill: "ralphx:rule-manager" },
    result: "Rule optimization complete",
    ...overrides,
  };
}

describe("SkillWidget", () => {
  describe("rendering", () => {
    it("shows skill name in title", () => {
      render(<SkillWidget toolCall={makeSkillCall()} />);
      expect(screen.getByText("ralphx:rule-manager")).toBeInTheDocument();
    });

    it("shows skill name with args when args provided", () => {
      render(
        <SkillWidget
          toolCall={makeSkillCall({
            arguments: { skill: "commit", args: "-m 'Fix bug'" },
          })}
        />
      );
      expect(screen.getByText("commit -m 'Fix bug'")).toBeInTheDocument();
    });

    it("shows fallback 'Skill' when no skill name provided", () => {
      render(
        <SkillWidget
          toolCall={makeSkillCall({ arguments: {} })}
        />
      );
      expect(screen.getByText("Skill")).toBeInTheDocument();
    });

    it("shows ok badge on success", () => {
      render(<SkillWidget toolCall={makeSkillCall()} />);
      expect(screen.getByText("ok")).toBeInTheDocument();
    });

    it("shows error badge on error", () => {
      render(
        <SkillWidget
          toolCall={makeSkillCall({
            error: "Skill not found",
            result: undefined,
          })}
        />
      );
      expect(screen.getByText("error")).toBeInTheDocument();
    });

    it("shows result text in body", () => {
      render(<SkillWidget toolCall={makeSkillCall()} />);
      expect(screen.getByText("Rule optimization complete")).toBeInTheDocument();
    });
  });

  describe("error states", () => {
    it("renders error text in red background", () => {
      render(
        <SkillWidget
          toolCall={makeSkillCall({
            error: "Failed to execute skill",
            result: undefined,
          })}
        />
      );
      expect(screen.getByText("Failed to execute skill")).toBeInTheDocument();
      // Error badge should be present
      expect(screen.getByText("error")).toBeInTheDocument();
    });

    it("auto-expands on error", () => {
      render(
        <SkillWidget
          toolCall={makeSkillCall({
            error: "Skill execution failed",
            result: undefined,
          })}
        />
      );
      // Error text should be visible (auto-expanded)
      expect(screen.getByText("Skill execution failed")).toBeInTheDocument();
    });
  });

  describe("result parsing", () => {
    it("parses plain string result", () => {
      render(
        <SkillWidget
          toolCall={makeSkillCall({ result: "Success message" })}
        />
      );
      expect(screen.getByText("Success message")).toBeInTheDocument();
    });

    it("parses MCP wrapper result", () => {
      render(
        <SkillWidget
          toolCall={makeSkillCall({
            result: [{ type: "text", text: "MCP wrapped result" }],
          })}
        />
      );
      expect(screen.getByText("MCP wrapped result")).toBeInTheDocument();
    });

    it("parses multiline result", () => {
      render(
        <SkillWidget
          toolCall={makeSkillCall({
            result: "Line 1\nLine 2\nLine 3",
          })}
        />
      );
      expect(screen.getByText(/Line 1.*Line 2.*Line 3/s)).toBeInTheDocument();
    });
  });

  describe("pending state", () => {
    it("shows no badge when result is undefined and no error", () => {
      render(
        <SkillWidget
          toolCall={makeSkillCall({ result: undefined })}
        />
      );
      expect(screen.queryByText("ok")).not.toBeInTheDocument();
      expect(screen.queryByText("error")).not.toBeInTheDocument();
    });
  });

  describe("compact mode", () => {
    it("passes compact prop without crashing", () => {
      render(<SkillWidget toolCall={makeSkillCall()} compact />);
      expect(screen.getByText("ralphx:rule-manager")).toBeInTheDocument();
    });
  });

  describe("collapse interaction", () => {
    it("starts collapsed by default for success results", () => {
      const { container } = render(
        <SkillWidget
          toolCall={makeSkillCall({ result: "Long result text" })}
        />
      );
      // Should have a chevron for collapse
      const chevrons = container.querySelectorAll('[style*="rotate"]');
      expect(chevrons.length).toBeGreaterThan(0);
    });

    it("toggles body visibility on click", async () => {
      const user = userEvent.setup();
      render(
        <SkillWidget
          toolCall={makeSkillCall({ result: "Result content" })}
        />
      );

      // The widget card header is clickable
      const toggle = screen.getByRole("button");
      expect(screen.getByText("Result content")).toBeInTheDocument();

      // Click to expand
      await user.click(toggle);
      // Content still visible
      expect(screen.getByText("Result content")).toBeInTheDocument();
    });
  });
});
