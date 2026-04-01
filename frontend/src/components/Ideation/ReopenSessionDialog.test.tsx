import { describe, it, expect, vi } from "vitest";
import { render, screen } from "@testing-library/react";
import { ReopenSessionDialog } from "./ReopenSessionDialog";

const defaultProps = {
  open: true,
  onOpenChange: vi.fn(),
  mode: "reopen" as const,
  sessionTitle: "My Session",
  taskCount: 3,
  onConfirm: vi.fn(),
  isLoading: false,
};

describe("ReopenSessionDialog", () => {
  describe("branch info display", () => {
    it("renders branch info when featureBranch is set", () => {
      render(
        <ReopenSessionDialog
          {...defaultProps}
          featureBranch="feature/my-branch"
          targetBranch="develop"
        />,
      );
      expect(
        screen.getByText(/Feature branch: feature\/my-branch → develop/),
      ).toBeInTheDocument();
    });

    it("renders nothing extra when featureBranch is undefined", () => {
      render(<ReopenSessionDialog {...defaultProps} />);
      expect(screen.queryByText(/Feature branch:/)).not.toBeInTheDocument();
    });

    it("shows 'main' fallback when targetBranch is not provided", () => {
      render(
        <ReopenSessionDialog
          {...defaultProps}
          featureBranch="feature/my-branch"
        />,
      );
      expect(
        screen.getByText(/Feature branch: feature\/my-branch → main/),
      ).toBeInTheDocument();
    });
  });

  describe("modes", () => {
    it("shows branch info in reopen mode", () => {
      render(
        <ReopenSessionDialog
          {...defaultProps}
          mode="reopen"
          featureBranch="feature/x"
        />,
      );
      expect(screen.getByText(/Feature branch: feature\/x → main/)).toBeInTheDocument();
    });

    it("shows branch info in reset mode", () => {
      render(
        <ReopenSessionDialog
          {...defaultProps}
          mode="reset"
          featureBranch="feature/x"
          targetBranch="staging"
        />,
      );
      expect(
        screen.getByText(/Feature branch: feature\/x → staging/),
      ).toBeInTheDocument();
    });
  });
});
