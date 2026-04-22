import { fireEvent, render, screen, waitFor } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";
import type { ReactNode } from "react";
import { MergePipelinePopover } from "./MergePipelinePopover";
import type { MergePipelineTask } from "@/api/merge-pipeline";

const retryMergeMock = vi.hoisted(() => vi.fn());
const moveTaskMock = vi.hoisted(() => vi.fn());
const navigateToTaskMock = vi.hoisted(() => vi.fn());

vi.mock("@/lib/tauri", () => ({
  api: {
    tasks: {
      retryMerge: retryMergeMock,
      move: moveTaskMock,
    },
  },
}));

vi.mock("@/stores/uiStore", () => ({
  useUiStore: (selector: (state: { navigateToTask: typeof navigateToTaskMock }) => unknown) =>
    selector({ navigateToTask: navigateToTaskMock }),
}));

vi.mock("@/components/ui/popover", () => ({
  Popover: ({ children }: { children: ReactNode }) => <div>{children}</div>,
  PopoverTrigger: ({ children }: { children: ReactNode }) => <>{children}</>,
  PopoverContent: ({ children }: { children: ReactNode }) => <div>{children}</div>,
}));

const makeTask = (overrides: Partial<MergePipelineTask> = {}): MergePipelineTask => ({
  taskId: "task-1",
  title: "Escalated merge",
  internalStatus: "merge_incomplete",
  sourceBranch: "ralphx/app/task-1",
  targetBranch: "main",
  isDeferred: false,
  isMainMergeDeferred: false,
  blockingBranch: null,
  conflictFiles: null,
  errorContext: "Repository hook environment failed",
  ...overrides,
});

describe("MergePipelinePopover", () => {
  it("labels attention merge rows as escalated", () => {
    render(
      <MergePipelinePopover active={[]} waiting={[]} needsAttention={[makeTask()]}>
        <button>Open merges</button>
      </MergePipelinePopover>
    );

    expect(screen.getByText("Escalated")).toBeInTheDocument();
  });

  it("uses the merge retry command instead of a generic status move", async () => {
    retryMergeMock.mockResolvedValueOnce(null);
    render(
      <MergePipelinePopover active={[]} waiting={[]} needsAttention={[makeTask()]}>
        <button>Open merges</button>
      </MergePipelinePopover>
    );

    fireEvent.click(screen.getByTitle("Retry merge"));

    await waitFor(() => {
      expect(retryMergeMock).toHaveBeenCalledWith("task-1");
    });
    expect(moveTaskMock).not.toHaveBeenCalled();
  });
});
