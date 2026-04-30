import { describe, it, expect, vi, beforeEach } from "vitest";
import { render, screen, fireEvent } from "@testing-library/react";
import { WaitingForCapacityState } from "./EmptyStates";
import { useExecutionStatus, usePauseExecution } from "@/hooks/useExecutionControl";
import type { ExecutionStatusResponse } from "@/api/execution";

const mocks = vi.hoisted(() => ({
  openModal: vi.fn(),
  resume: vi.fn(),
  executionStatus: {
    isPaused: false,
    haltMode: "running",
    runningCount: 0,
    maxConcurrent: 3,
    globalMaxConcurrent: 20,
    queuedCount: 0,
    canStartTask: true,
    ideationActive: 0,
    ideationIdle: 0,
    ideationWaiting: 0,
    ideationMaxProject: 2,
    ideationMaxGlobal: 10,
  },
}));

vi.mock("@/hooks/useExecutionControl", () => ({
  useExecutionStatus: vi.fn(),
  usePauseExecution: vi.fn(),
}));

vi.mock("@/stores/uiStore", () => ({
  useUiStore: vi.fn((selector: (state: { openModal: typeof mocks.openModal; executionStatus: ExecutionStatusResponse }) => unknown) =>
    selector({ openModal: mocks.openModal, executionStatus: mocks.executionStatus as ExecutionStatusResponse })
  ),
}));

const mockedUseExecutionStatus = vi.mocked(useExecutionStatus);
const mockedUsePauseExecution = vi.mocked(usePauseExecution);

const baseStatus = mocks.executionStatus as ExecutionStatusResponse;

function mockStatus(status: ExecutionStatusResponse) {
  mockedUseExecutionStatus.mockReturnValue({
    data: status,
    isLoading: false,
    isError: false,
  } as ReturnType<typeof useExecutionStatus>);
}

beforeEach(() => {
  vi.clearAllMocks();
  mockedUsePauseExecution.mockReturnValue({
    resume: mocks.resume,
    isPending: false,
  } as ReturnType<typeof usePauseExecution>);
});

describe("WaitingForCapacityState", () => {
  it("shows a paused saved-request state with a resume action", () => {
    mockStatus({
      ...baseStatus,
      isPaused: true,
      haltMode: "paused",
      canStartTask: false,
      ideationWaiting: 1,
    });

    render(<WaitingForCapacityState pendingInitialPrompt="Fix font scaling" projectId="project-1" />);

    expect(screen.getByText("Execution paused")).toBeInTheDocument();
    expect(screen.getByText("Your request is saved. Resume execution to start this ideation run.")).toBeInTheDocument();
    expect(screen.getByText("Saved request")).toBeInTheDocument();
    expect(screen.queryByText("Waiting for capacity")).not.toBeInTheDocument();
    expect(screen.queryByText("Adjust limits in Settings")).not.toBeInTheDocument();

    fireEvent.click(screen.getByRole("button", { name: "Resume execution" }));
    expect(mocks.resume).toHaveBeenCalledTimes(1);
  });

  it("keeps capacity copy and settings link when execution is running", () => {
    mockStatus({
      ...baseStatus,
      ideationActive: 2,
      ideationWaiting: 1,
    });

    render(<WaitingForCapacityState pendingInitialPrompt="Fix font scaling" projectId="project-1" />);

    expect(screen.getByText("Waiting for capacity")).toBeInTheDocument();
    expect(screen.getByText(/2\/2 slots active in this project/)).toBeInTheDocument();
    expect(screen.getByText("Queued message")).toBeInTheDocument();

    fireEvent.click(screen.getByRole("button", { name: "Adjust limits in Settings" }));
    expect(mocks.openModal).toHaveBeenCalledWith("settings", { section: "ideation-workflow" });
  });
});
