/**
 * ExportPlanDialog.test.tsx
 * Tests for the rewritten export dialog with JSON + Markdown download buttons
 */

import { describe, it, expect, vi, beforeEach } from "vitest";
import { render, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { ExportPlanDialog } from "./ExportPlanDialog";
import type { Artifact } from "@/types/artifact";

// ============================================================================
// Mocks
// ============================================================================

const mockExportSession = vi.fn();
let mockIsExporting = false;

vi.mock("@/hooks/useSessionExportImport", () => ({
  useSessionExportImport: () => ({
    exportSession: mockExportSession,
    get isExporting() { return mockIsExporting; },
    importSession: vi.fn(),
    isImporting: false,
  }),
}));

import { save } from "@tauri-apps/plugin-dialog";
import { writeTextFile } from "@tauri-apps/plugin-fs";
import { toast } from "sonner";

vi.mock("@tauri-apps/plugin-dialog", () => ({
  save: vi.fn(),
  open: vi.fn(),
}));

vi.mock("@tauri-apps/plugin-fs", () => ({
  writeTextFile: vi.fn(),
  readTextFile: vi.fn(),
  stat: vi.fn(),
}));

vi.mock("sonner", () => ({
  toast: {
    success: vi.fn(),
    error: vi.fn(),
  },
}));

// ============================================================================
// Fixtures
// ============================================================================

const inlinePlanArtifact: Artifact = {
  id: "artifact-1",
  type: "specification",
  name: "My Plan",
  content: { type: "inline", text: "# Plan Content\n\nSome plan text here." },
  metadata: {
    createdAt: "2026-01-01T00:00:00+00:00",
    createdBy: "orchestrator",
    version: 1,
  },
  derivedFrom: [],
};

const fileTypeArtifact: Artifact = {
  id: "artifact-2",
  type: "specification",
  name: "File Plan",
  content: { type: "file", path: "/some/path/plan.md" },
  metadata: {
    createdAt: "2026-01-01T00:00:00+00:00",
    createdBy: "orchestrator",
    version: 1,
  },
  derivedFrom: [],
};

const defaultProps = {
  open: true,
  onOpenChange: vi.fn(),
  sessionId: "session-abc",
  sessionTitle: "My Verified Plan",
  verificationStatus: "verified",
  planArtifact: inlinePlanArtifact,
  projectId: "proj-1",
};

// ============================================================================
// Tests
// ============================================================================

describe("ExportPlanDialog", () => {
  beforeEach(() => {
    vi.clearAllMocks();
    mockIsExporting = false;
  });

  // --- Rendering ---

  it("renders title 'Export Plan' when open", () => {
    render(<ExportPlanDialog {...defaultProps} />);
    expect(screen.getByText("Export Plan")).toBeInTheDocument();
  });

  it("renders session title and verification badge", () => {
    render(<ExportPlanDialog {...defaultProps} />);
    expect(screen.getByText("My Verified Plan")).toBeInTheDocument();
    expect(screen.getByText("Verified")).toBeInTheDocument();
  });

  it("renders 'Verified (imported)' for imported_verified status", () => {
    render(
      <ExportPlanDialog {...defaultProps} verificationStatus="imported_verified" />
    );
    expect(screen.getByText("Verified (imported)")).toBeInTheDocument();
  });

  it("renders Download JSON and Download Markdown buttons", () => {
    render(<ExportPlanDialog {...defaultProps} />);
    const downloadButtons = screen.getAllByRole("button", { name: /Download/i });
    expect(downloadButtons).toHaveLength(2);
  });

  it("renders 'Download JSON' card", () => {
    render(<ExportPlanDialog {...defaultProps} />);
    expect(screen.getByText("Download JSON")).toBeInTheDocument();
  });

  it("renders 'Download Markdown' card", () => {
    render(<ExportPlanDialog {...defaultProps} />);
    expect(screen.getByText("Download Markdown")).toBeInTheDocument();
  });

  // --- No plan state ---

  it("shows 'No plan content available' when planArtifact is null", () => {
    render(<ExportPlanDialog {...defaultProps} planArtifact={null} />);
    expect(screen.getByText(/No plan content available/i)).toBeInTheDocument();
  });

  it("both buttons are disabled when planArtifact is null", () => {
    render(<ExportPlanDialog {...defaultProps} planArtifact={null} />);
    const buttons = screen.getAllByRole("button", { name: /Download/i });
    buttons.forEach((btn) => expect(btn).toBeDisabled());
  });

  // --- File-type artifact ---

  it("JSON button is enabled but Markdown button is disabled for file-type artifact", () => {
    render(<ExportPlanDialog {...defaultProps} planArtifact={fileTypeArtifact} />);
    const [jsonBtn, mdBtn] = screen.getAllByRole("button", { name: /Download/i });
    expect(jsonBtn).not.toBeDisabled();
    expect(mdBtn).toBeDisabled();
  });

  it("does not show 'No plan content available' for file-type artifact", () => {
    render(<ExportPlanDialog {...defaultProps} planArtifact={fileTypeArtifact} />);
    expect(
      screen.queryByText(/No plan content available/i)
    ).not.toBeInTheDocument();
  });

  // --- JSON download ---

  it("calls exportSession with correct args when JSON Download is clicked", async () => {
    const user = userEvent.setup();
    render(<ExportPlanDialog {...defaultProps} />);

    const [jsonBtn] = screen.getAllByRole("button", { name: /Download/i });
    await user.click(jsonBtn);

    expect(mockExportSession).toHaveBeenCalledTimes(1);
    expect(mockExportSession).toHaveBeenCalledWith("session-abc", "proj-1", true);
  });

  it("JSON button is disabled and shows 'Exporting...' while isExporting is true", () => {
    mockIsExporting = true;
    render(<ExportPlanDialog {...defaultProps} />);
    const exportingBtn = screen.getAllByRole("button", { name: /Exporting\.\.\./i });
    expect(exportingBtn[0]).toBeDisabled();
  });

  // --- Markdown download ---

  it("calls save dialog and writeTextFile when Markdown Download is clicked", async () => {
    const user = userEvent.setup();
    vi.mocked(save).mockResolvedValueOnce("/some/path/plan.md");

    render(<ExportPlanDialog {...defaultProps} />);

    const [, mdBtn] = screen.getAllByRole("button", { name: /Download/i });
    await user.click(mdBtn);

    expect(save).toHaveBeenCalledWith(
      expect.objectContaining({
        filters: [{ name: "Markdown", extensions: ["md"] }],
        defaultPath: "My Verified Plan.md",
      })
    );
    expect(writeTextFile).toHaveBeenCalledWith(
      "/some/path/plan.md",
      "# Plan Content\n\nSome plan text here."
    );
  });

  it("uses 'plan.md' as fallback filename when sessionTitle is null", async () => {
    const user = userEvent.setup();
    vi.mocked(save).mockResolvedValueOnce("/some/path/plan.md");

    render(<ExportPlanDialog {...defaultProps} sessionTitle={null} />);

    const [, mdBtn] = screen.getAllByRole("button", { name: /Download/i });
    await user.click(mdBtn);

    expect(save).toHaveBeenCalledWith(
      expect.objectContaining({ defaultPath: "plan.md" })
    );
  });

  it("shows success toast after successful markdown download", async () => {
    const user = userEvent.setup();
    vi.mocked(save).mockResolvedValueOnce("/some/path/plan.md");
    vi.mocked(writeTextFile).mockResolvedValueOnce(undefined);

    render(<ExportPlanDialog {...defaultProps} />);

    const [, mdBtn] = screen.getAllByRole("button", { name: /Download/i });
    await user.click(mdBtn);

    await waitFor(() => {
      expect(toast.success).toHaveBeenCalledWith("Plan exported as Markdown");
    });
  });

  it("shows error toast when writeTextFile fails", async () => {
    const user = userEvent.setup();
    vi.mocked(save).mockResolvedValueOnce("/some/path/plan.md");
    vi.mocked(writeTextFile).mockRejectedValueOnce(new Error("disk error"));

    render(<ExportPlanDialog {...defaultProps} />);

    const [, mdBtn] = screen.getAllByRole("button", { name: /Download/i });
    await user.click(mdBtn);

    await waitFor(() => {
      expect(toast.error).toHaveBeenCalledWith("Failed to export plan as Markdown");
    });
  });

  // --- Cancellation ---

  it("does not call writeTextFile when save dialog is cancelled (returns null)", async () => {
    const user = userEvent.setup();
    vi.mocked(save).mockResolvedValueOnce(null);

    render(<ExportPlanDialog {...defaultProps} />);

    const [, mdBtn] = screen.getAllByRole("button", { name: /Download/i });
    await user.click(mdBtn);

    await waitFor(() => {
      expect(writeTextFile).not.toHaveBeenCalled();
      expect(toast.error).not.toHaveBeenCalled();
      expect(toast.success).not.toHaveBeenCalled();
    });
  });

  // --- Loading state ---

  it("markdown button is disabled and shows 'Exporting...' while download is in progress", async () => {
    const user = userEvent.setup();
    // Save never resolves — button should stay in loading state
    vi.mocked(save).mockReturnValueOnce(new Promise(() => {}));

    render(<ExportPlanDialog {...defaultProps} />);

    const [, mdBtn] = screen.getAllByRole("button", { name: /Download/i });
    await user.click(mdBtn);

    await waitFor(() => {
      expect(screen.getByRole("button", { name: /Exporting\.\.\./i })).toBeDisabled();
    });
  });
});
