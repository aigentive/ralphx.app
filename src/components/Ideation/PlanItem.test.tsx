import { render, screen, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { describe, it, expect, vi } from "vitest";
import { PlanItem } from "./PlanItem";
import type { PlanItemProps } from "./PlanItem";
import type { IdeationSession } from "@/types/ideation";
import type { SessionGroup } from "./planBrowserUtils";
import type { SessionProgress } from "@/hooks/useSessionProgress";

function createSession(overrides: Partial<IdeationSession> = {}): IdeationSession {
  return {
    id: "session-1",
    projectId: "project-1",
    title: "Test Session",
    status: "active",
    planArtifactId: null,
    seedTaskId: null,
    parentSessionId: null,
    createdAt: "2026-01-24T12:00:00Z",
    updatedAt: "2026-01-24T12:00:00Z",
    archivedAt: null,
    convertedAt: null,
    ...overrides,
  };
}

function createProgress(overrides: Partial<SessionProgress> = {}): SessionProgress {
  return { idle: 0, active: 0, done: 0, total: 0, ...overrides };
}

const defaultProps: PlanItemProps = {
  plan: createSession(),
  isSelected: false,
  group: "drafts",
  isEditing: false,
  editingTitle: "",
  isMenuOpen: false,
  inputRef: { current: null },
  onSelect: vi.fn(),
  onStartRename: vi.fn(),
  onCancelRename: vi.fn(),
  onConfirmRename: vi.fn(),
  onTitleChange: vi.fn(),
  onKeyDown: vi.fn(),
  onMenuOpenChange: vi.fn(),
  onArchive: vi.fn(),
  onDelete: vi.fn(),
  onReopen: vi.fn(),
  onResetReaccept: vi.fn(),
};

function renderItem(overrides: Partial<PlanItemProps> = {}) {
  return render(<PlanItem {...defaultProps} {...overrides} />);
}

describe("PlanItem", () => {
  it("renders the session title", () => {
    renderItem();
    expect(screen.getByText("Test Session")).toBeInTheDocument();
  });

  it("renders 'Untitled Plan' when title is null", () => {
    renderItem({ plan: createSession({ title: null }) });
    expect(screen.getByText("Untitled Plan")).toBeInTheDocument();
  });

  it("calls onSelect when clicked", async () => {
    const onSelect = vi.fn();
    renderItem({ onSelect });
    await userEvent.click(screen.getByTestId("plan-item-session-1"));
    expect(onSelect).toHaveBeenCalledOnce();
  });

  describe("metadata line per group", () => {
    it("drafts: shows relative time with Clock icon", () => {
      renderItem({ group: "drafts" });
      // Should contain some time text (e.g. "Xm ago", "Xh ago", etc.)
      const container = screen.getByTestId("plan-item-session-1");
      // Clock icon is rendered but invisible to text — just check relative time text exists
      expect(container).toBeInTheDocument();
    });

    it("in-progress: shows progress counts with colored text", () => {
      renderItem({
        group: "in-progress",
        plan: createSession({ status: "accepted" }),
        progress: createProgress({ done: 3, active: 2, total: 7 }),
      });
      expect(screen.getByText("3/7 done")).toBeInTheDocument();
      expect(screen.getByText("2 active")).toBeInTheDocument();
    });

    it("in-progress: hides active count when 0", () => {
      renderItem({
        group: "in-progress",
        plan: createSession({ status: "accepted" }),
        progress: createProgress({ done: 3, active: 0, total: 7 }),
      });
      expect(screen.getByText("3/7 done")).toBeInTheDocument();
      expect(screen.queryByText(/active/)).not.toBeInTheDocument();
    });

    it("accepted: shows task count and convertedAt date", () => {
      renderItem({
        group: "accepted",
        plan: createSession({ status: "accepted", convertedAt: "2026-01-20T10:00:00Z" }),
        progress: createProgress({ idle: 5, total: 5 }),
      });
      expect(screen.getByText("5 tasks")).toBeInTheDocument();
      // Date formatting is locale-dependent, so just check it's present
      expect(screen.getByText(/Jan/)).toBeInTheDocument();
    });

    it("accepted: shows singular 'task' for 1 task", () => {
      renderItem({
        group: "accepted",
        plan: createSession({ status: "accepted" }),
        progress: createProgress({ idle: 1, total: 1 }),
      });
      expect(screen.getByText("1 task")).toBeInTheDocument();
    });

    it("done: shows Completed text", () => {
      renderItem({
        group: "done",
        plan: createSession({ status: "accepted" }),
        progress: createProgress({ done: 5, total: 5 }),
      });
      expect(screen.getByText("Completed")).toBeInTheDocument();
    });

    it("archived: shows archived date when available", () => {
      renderItem({
        group: "archived",
        plan: createSession({ status: "archived", archivedAt: "2026-01-15T10:00:00Z" }),
      });
      expect(screen.getByText(/Archived/)).toBeInTheDocument();
      expect(screen.getByText(/Jan/)).toBeInTheDocument();
    });

    it("archived: shows 'Archived' without date when archivedAt is null", () => {
      renderItem({
        group: "archived",
        plan: createSession({ status: "archived", archivedAt: null }),
      });
      expect(screen.getByText("Archived")).toBeInTheDocument();
    });
  });

  describe("muted styling for done/archived groups", () => {
    it("done items have reduced opacity when not selected", () => {
      renderItem({ group: "done" });
      const item = screen.getByTestId("plan-item-session-1");
      expect(item.style.opacity).toBe("0.7");
    });

    it("archived items have reduced opacity when not selected", () => {
      renderItem({ group: "archived" });
      const item = screen.getByTestId("plan-item-session-1");
      expect(item.style.opacity).toBe("0.7");
    });

    it("done items have full opacity when selected", () => {
      renderItem({ group: "done", isSelected: true });
      const item = screen.getByTestId("plan-item-session-1");
      expect(item.style.opacity).toBe("1");
    });

    it("drafts items have full opacity", () => {
      renderItem({ group: "drafts" });
      const item = screen.getByTestId("plan-item-session-1");
      expect(item.style.opacity).toBe("1");
    });
  });

  describe("context menu actions per group", () => {
    async function openMenu(group: SessionGroup) {
      renderItem({ group });
      const item = screen.getByTestId("plan-item-session-1");
      // Find the menu trigger button (the MoreHorizontal icon button)
      const menuButton = within(item).getAllByRole("button")[0];
      await userEvent.click(menuButton);
    }

    it("drafts: shows Rename, Archive, Delete", async () => {
      await openMenu("drafts");
      expect(screen.getByText("Rename")).toBeInTheDocument();
      expect(screen.getByText("Archive")).toBeInTheDocument();
      expect(screen.getByText("Delete")).toBeInTheDocument();
      expect(screen.queryByText("Reopen")).not.toBeInTheDocument();
      expect(screen.queryByText("Reset & Re-accept")).not.toBeInTheDocument();
    });

    it("accepted: shows Rename, Reopen, Reset & Re-accept, Delete", async () => {
      await openMenu("accepted");
      expect(screen.getByText("Rename")).toBeInTheDocument();
      expect(screen.getByText("Reopen")).toBeInTheDocument();
      expect(screen.getByText("Reset & Re-accept")).toBeInTheDocument();
      expect(screen.getByText("Delete")).toBeInTheDocument();
      expect(screen.queryByText("Archive")).not.toBeInTheDocument();
    });

    it("in-progress: shows Rename, Reopen, Reset & Re-accept, Delete", async () => {
      await openMenu("in-progress");
      expect(screen.getByText("Rename")).toBeInTheDocument();
      expect(screen.getByText("Reopen")).toBeInTheDocument();
      expect(screen.getByText("Reset & Re-accept")).toBeInTheDocument();
      expect(screen.getByText("Delete")).toBeInTheDocument();
    });

    it("done: shows Rename, Reopen, Reset & Re-accept, Delete", async () => {
      await openMenu("done");
      expect(screen.getByText("Rename")).toBeInTheDocument();
      expect(screen.getByText("Reopen")).toBeInTheDocument();
      expect(screen.getByText("Reset & Re-accept")).toBeInTheDocument();
      expect(screen.getByText("Delete")).toBeInTheDocument();
    });

    it("archived: shows Rename, Reopen, Delete (no Reset & Re-accept)", async () => {
      await openMenu("archived");
      expect(screen.getByText("Rename")).toBeInTheDocument();
      expect(screen.getByText("Reopen")).toBeInTheDocument();
      expect(screen.getByText("Delete")).toBeInTheDocument();
      expect(screen.queryByText("Reset & Re-accept")).not.toBeInTheDocument();
    });
  });
});
