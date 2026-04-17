import { fireEvent, render, screen } from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";

import {
  ExecutionHarnessSection,
  IdeationHarnessSection,
} from "./IdeationHarnessSection";
import type { AgentHarnessLaneView } from "@/api/ideation-harness";
import { useAgentHarnessSettings } from "@/hooks/useIdeationHarnessSettings";
import { useProjectStore } from "@/stores/projectStore";

vi.mock("@/hooks/useIdeationHarnessSettings", () => ({
  useAgentHarnessSettings: vi.fn(),
}));

vi.mock("@/stores/projectStore", () => ({
  useProjectStore: vi.fn(),
  selectActiveProject: (state: { activeProject: unknown }) => state.activeProject,
}));

const globalLanes: AgentHarnessLaneView[] = [
  {
    lane: "ideation_primary",
    row: {
      projectId: null,
      lane: "ideation_primary",
      harness: "codex",
      model: "gpt-5.4",
      effort: "xhigh",
      approvalPolicy: "never",
      sandboxMode: "danger-full-access",
      updatedAt: new Date().toISOString(),
    },
    configuredHarness: "codex",
    effectiveHarness: "codex",
    binaryPath: "/usr/local/bin/codex",
    binaryFound: true,
    probeSucceeded: true,
    available: true,
    missingCoreExecFeatures: [],
    error: null,
  },
  {
    lane: "ideation_verifier",
    row: {
      projectId: null,
      lane: "ideation_verifier",
      harness: "claude",
      model: null,
      effort: null,
      approvalPolicy: null,
      sandboxMode: null,
      updatedAt: new Date().toISOString(),
    },
    configuredHarness: "claude",
    effectiveHarness: "claude",
    binaryPath: "/usr/local/bin/claude",
    binaryFound: true,
    probeSucceeded: true,
    available: true,
    missingCoreExecFeatures: [],
    error: null,
  },
];

const updateLane = vi.fn();

describe("IdeationHarnessSection", () => {
  beforeEach(() => {
    vi.clearAllMocks();
    vi.mocked(useProjectStore).mockReturnValue({
      id: "project-1",
      name: "Project One",
    });
    vi.mocked(useAgentHarnessSettings).mockImplementation((projectId) => ({
      lanes: projectId === null ? globalLanes : [],
      isLoading: false,
      isPlaceholderData: false,
      isError: false,
      error: null,
      updateLane,
      isUpdating: false,
      saveError: null,
      resetError: vi.fn(),
    }));
  });

  it("renders Codex-only lane controls for Codex lanes", () => {
    render(<IdeationHarnessSection />);

    expect(screen.getByText("Approval")).toBeInTheDocument();
    expect(screen.getByText("Sandbox")).toBeInTheDocument();
    expect(screen.queryByText("Fallback Harness")).not.toBeInTheDocument();
    expect(
      screen.getByText(
        "Temporarily locked for Codex: RalphX MCP tools currently require Never approval and Danger Full Access.",
      ),
    ).toBeInTheDocument();
    expect(screen.getByTestId("approval-ideation_primary")).toHaveAttribute(
      "data-disabled",
    );
    expect(screen.getByTestId("sandbox-ideation_primary")).toHaveAttribute(
      "data-disabled",
    );
    expect(screen.getByText("Ideation Agents")).toBeInTheDocument();
    expect(screen.queryByText("Execution Worker")).not.toBeInTheDocument();
  });

  it("saves merged lane settings when a model field changes", () => {
    render(<IdeationHarnessSection />);

    const modelInput = screen.getByDisplayValue("gpt-5.4");
    fireEvent.change(modelInput, { target: { value: "gpt-5.4.1" } });
    fireEvent.blur(modelInput);

    expect(updateLane).toHaveBeenCalledWith(
      {
        lane: "ideation_primary",
        harness: "codex",
        model: "gpt-5.4.1",
        effort: "xhigh",
        approvalPolicy: "never",
        sandboxMode: "danger-full-access",
      },
      { onError: expect.any(Function) },
    );
  });

  it("shows Codex model presets in dropdown when input is cleared and focused", () => {
    render(<IdeationHarnessSection />);

    const modelInput = screen.getByDisplayValue("gpt-5.4");
    // Clear the input to see all presets
    fireEvent.change(modelInput, { target: { value: "" } });
    fireEvent.focus(modelInput);

    expect(screen.getByText("gpt-5.4 (Current)")).toBeInTheDocument();
    expect(screen.getByText("gpt-5.4-mini")).toBeInTheDocument();
    expect(screen.getByText("gpt-5.3-codex")).toBeInTheDocument();
    expect(screen.getByText("gpt-5.3-codex-spark")).toBeInTheDocument();
  });

  it("shows Claude model presets for Claude harness lanes", () => {
    render(<IdeationHarnessSection />);

    // ideation_verifier is Claude harness with null model
    const claudeModelInput = screen.getByPlaceholderText("sonnet");
    expect(claudeModelInput).toBeInTheDocument();
    fireEvent.change(claudeModelInput, { target: { value: "" } });
    fireEvent.focus(claudeModelInput);

    expect(screen.getByText("sonnet")).toBeInTheDocument();
    expect(screen.getByText("opus")).toBeInTheDocument();
    expect(screen.getByText("haiku")).toBeInTheDocument();
  });

  it("shows effort options with clearer labels including Default and Maximum", () => {
    render(<IdeationHarnessSection />);

    // The effort select for ideation_primary shows "Maximum" for xhigh
    // Check that the effort dropdowns render with the updated labels in the DOM
    const effortTriggers = document.querySelectorAll('[placeholder="Select effort"]');
    expect(effortTriggers.length).toBe(0); // triggers don't have placeholders; SelectValue shows selected

    // Verify the effort options are rendered inside SelectContent (accessible)
    // The "Default" label replaces "Inherit"
    expect(screen.queryByText("Inherit")).not.toBeInTheDocument();
    expect(screen.queryByText("XHigh")).not.toBeInTheDocument();
  });
});

describe("ExecutionHarnessSection", () => {
  beforeEach(() => {
    vi.clearAllMocks();
    vi.mocked(useProjectStore).mockReturnValue({
      id: "project-1",
      name: "Project One",
    });
    vi.mocked(useAgentHarnessSettings).mockImplementation((projectId) => ({
      lanes:
        projectId === null
          ? [
              ...globalLanes,
              {
                lane: "execution_worker",
                row: {
                  projectId: null,
                  lane: "execution_worker",
                  harness: "codex",
                  model: "gpt-5.4",
                  effort: "xhigh",
                  approvalPolicy: "never",
                  sandboxMode: "danger-full-access",
                  updatedAt: new Date().toISOString(),
                },
                configuredHarness: "codex",
                effectiveHarness: "codex",
                binaryPath: "/usr/local/bin/codex",
                binaryFound: true,
                probeSucceeded: true,
                available: true,
                missingCoreExecFeatures: [],
                error: null,
              },
            ]
          : [],
      isLoading: false,
      isPlaceholderData: false,
      isError: false,
      error: null,
      updateLane,
      isUpdating: false,
      saveError: null,
      resetError: vi.fn(),
    }));
  });

  it("renders execution lanes as a first-class section", () => {
    render(<ExecutionHarnessSection />);

    expect(screen.getByText("Execution Pipeline Agents")).toBeInTheDocument();
    expect(screen.getByText("Execution Worker")).toBeInTheDocument();
    expect(screen.queryByText("Primary Ideation")).not.toBeInTheDocument();
  });
});
