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
      approvalPolicy: "on-request",
      sandboxMode: "workspace-write",
      fallbackHarness: "claude",
      updatedAt: new Date().toISOString(),
    },
    configuredHarness: "codex",
    effectiveHarness: "codex",
    fallbackHarness: "claude",
    fallbackActivated: false,
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
      fallbackHarness: null,
      updatedAt: new Date().toISOString(),
    },
    configuredHarness: "claude",
    effectiveHarness: "claude",
    fallbackHarness: null,
    fallbackActivated: false,
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
    expect(screen.getByText("Fallback Harness")).toBeInTheDocument();
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
        approvalPolicy: "on-request",
        sandboxMode: "workspace-write",
        fallbackHarness: "claude",
      },
      { onError: expect.any(Function) },
    );
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
                  approvalPolicy: "on-request",
                  sandboxMode: "workspace-write",
                  fallbackHarness: "claude",
                  updatedAt: new Date().toISOString(),
                },
                configuredHarness: "codex",
                effectiveHarness: "codex",
                fallbackHarness: "claude",
                fallbackActivated: false,
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
