import { describe, it, expect } from "vitest";
import { render, screen, fireEvent } from "@testing-library/react";
import { TeamFindingsSection } from "./TeamFindingsSection";
import type { TeamFinding } from "./TeamFindingsSection";

const mockFindings: TeamFinding[] = [
  { specialist: "Frontend", keyFinding: "Existing ChatPanel uses unified hooks", color: "#4ade80" },
  { specialist: "Backend", keyFinding: "AgenticClient trait supports team spawning", color: "#60a5fa" },
  { specialist: "Infra", keyFinding: "Database schema needs new team_sessions table" },
];

describe("TeamFindingsSection", () => {
  it("renders findings table with specialist names and key findings", () => {
    render(
      <TeamFindingsSection
        findings={mockFindings}
        teamMode="research"
        teammateCount={3}
        defaultExpanded={true}
      />
    );

    expect(screen.getByText("Frontend")).toBeInTheDocument();
    expect(screen.getByText("Backend")).toBeInTheDocument();
    expect(screen.getByText("Infra")).toBeInTheDocument();
    expect(screen.getByText("Existing ChatPanel uses unified hooks")).toBeInTheDocument();
    expect(screen.getByText("AgenticClient trait supports team spawning")).toBeInTheDocument();
    expect(screen.getByText("Database schema needs new team_sessions table")).toBeInTheDocument();
  });

  it("collapses and expands findings", () => {
    render(
      <TeamFindingsSection
        findings={mockFindings}
        teamMode="research"
        teammateCount={3}
      />
    );

    // Default collapsed — findings not visible
    expect(screen.queryByText("Frontend")).not.toBeInTheDocument();

    // Click to expand
    fireEvent.click(screen.getByRole("button", { name: /team research summary/i }));
    expect(screen.getByText("Frontend")).toBeInTheDocument();
  });

  it("renders nothing when findings array is empty", () => {
    const { container } = render(
      <TeamFindingsSection
        findings={[]}
        teamMode="research"
        teammateCount={0}
      />
    );

    expect(container.firstChild).toBeNull();
  });

  it("shows teammate count badge", () => {
    render(
      <TeamFindingsSection
        findings={mockFindings}
        teamMode="research"
        teammateCount={3}
      />
    );

    expect(screen.getByText("3")).toBeInTheDocument();
  });
});
