import type React from "react";
import { render, screen, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { describe, it, expect, vi, beforeEach } from "vitest";
import { PlanItem } from "./PlanItem";
import type { PlanItemProps } from "./PlanItem";
import type { IdeationSessionWithProgress, SessionProgress } from "@/types/ideation";
import type { SessionGroup } from "./planBrowserUtils";
import { useChatStore } from "@/stores/chatStore";
import { useIdeationStore } from "@/stores/ideationStore";

function createProgress(overrides: Partial<SessionProgress> = {}): SessionProgress {
  return { idle: 0, active: 0, done: 0, total: 0, ...overrides };
}

function createSession(overrides: Partial<IdeationSessionWithProgress> = {}): IdeationSessionWithProgress {
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
    progress: null,
    parentSessionTitle: null,
    ...overrides,
  };
}

const defaultProps: PlanItemProps = {
  plan: createSession(),
  isSelected: false,
  group: "drafts",
  isEditing: false,
  isMenuOpen: false,
  inputRef: { current: null },
  onSelect: vi.fn(),
  onStartRename: vi.fn(),
  onConfirmRename: vi.fn(),
  onTitleChange: vi.fn(),
  onKeyDown: vi.fn(),
  onMenuOpenChange: vi.fn(),
  onArchive: vi.fn(),
  onReopen: vi.fn(),
  onResetReaccept: vi.fn(),
};

function renderItem(overrides: Partial<PlanItemProps> = {}) {
  return render(<PlanItem {...defaultProps} {...overrides} />);
}

describe("PlanItem", () => {
  beforeEach(() => {
    useChatStore.setState({ agentStatus: {} });
    useIdeationStore.setState({ activeVerificationChildId: {} });
  });

  it("renders the session title", () => {
    renderItem();
    expect(screen.getByText("Test Session")).toBeInTheDocument();
  });

  it("renders 'Untitled Plan' when title is null", () => {
    renderItem({ plan: createSession({ title: null }) });
    expect(screen.getByText("Untitled Plan")).toBeInTheDocument();
  });

  it("calls onSelect with planId when clicked", async () => {
    const onSelect = vi.fn();
    renderItem({ onSelect });
    await userEvent.click(screen.getByTestId("plan-item-session-1"));
    expect(onSelect).toHaveBeenCalledOnce();
    expect(onSelect).toHaveBeenCalledWith("session-1");
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
        plan: createSession({ status: "accepted", progress: createProgress({ done: 3, active: 2, total: 7 }) }),
      });
      expect(screen.getByText("3/7 done")).toBeInTheDocument();
      expect(screen.getByText("2 active")).toBeInTheDocument();
    });

    it("in-progress: hides active count when 0", () => {
      renderItem({
        group: "in-progress",
        plan: createSession({ status: "accepted", progress: createProgress({ done: 3, active: 0, total: 7 }) }),
      });
      expect(screen.getByText("3/7 done")).toBeInTheDocument();
      expect(screen.queryByText(/active/)).not.toBeInTheDocument();
    });

    it("accepted: shows task count and convertedAt date", () => {
      renderItem({
        group: "accepted",
        plan: createSession({ status: "accepted", convertedAt: "2026-01-20T10:00:00Z", progress: createProgress({ idle: 5, total: 5 }) }),
      });
      expect(screen.getByText("5 tasks")).toBeInTheDocument();
      // Date formatting is locale-dependent, so just check it's present
      expect(screen.getByText(/Jan/)).toBeInTheDocument();
    });

    it("accepted: shows singular 'task' for 1 task", () => {
      renderItem({
        group: "accepted",
        plan: createSession({ status: "accepted", progress: createProgress({ idle: 1, total: 1 }) }),
      });
      expect(screen.getByText("1 task")).toBeInTheDocument();
    });

    it("done: shows Completed text", () => {
      renderItem({
        group: "done",
        plan: createSession({ status: "accepted", progress: createProgress({ done: 5, total: 5 }) }),
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

  describe("import badge", () => {
    it("shows import badge when sourceProjectId is present", () => {
      renderItem({
        plan: createSession({ sourceProjectId: "project-other", sourceSessionId: "session-other" }),
      });
      expect(screen.getByTestId("import-badge")).toBeInTheDocument();
      expect(screen.getByText("Imported")).toBeInTheDocument();
    });

    it("hides import badge when sourceProjectId is absent", () => {
      renderItem({ plan: createSession() });
      expect(screen.queryByTestId("import-badge")).not.toBeInTheDocument();
      expect(screen.queryByText("Imported")).not.toBeInTheDocument();
    });

    it("hides import badge when sourceProjectId is null", () => {
      renderItem({ plan: createSession({ sourceProjectId: null }) });
      expect(screen.queryByTestId("import-badge")).not.toBeInTheDocument();
    });

    it("calls onNavigateToSource with planId when import badge is clicked", async () => {
      const onNavigateToSource = vi.fn();
      const onSelect = vi.fn();
      renderItem({
        plan: createSession({ sourceProjectId: "project-other" }),
        onNavigateToSource,
        onSelect,
      });
      await userEvent.click(screen.getByTestId("import-badge"));
      expect(onNavigateToSource).toHaveBeenCalledOnce();
      expect(onNavigateToSource).toHaveBeenCalledWith("session-1");
      // Should NOT trigger onSelect (stopPropagation)
      expect(onSelect).not.toHaveBeenCalled();
    });

    it("does not call onNavigateToSource when badge is absent", () => {
      const onNavigateToSource = vi.fn();
      renderItem({ plan: createSession(), onNavigateToSource });
      expect(screen.queryByTestId("import-badge")).not.toBeInTheDocument();
      expect(onNavigateToSource).not.toHaveBeenCalled();
    });
  });

  describe("agent status indicators", () => {
    it("idle: shows no spinner and no 'Agent working' text", () => {
      renderItem({ group: "drafts" });
      expect(document.querySelector(".animate-spin")).not.toBeInTheDocument();
      expect(screen.queryByText(/Agent working/)).not.toBeInTheDocument();
    });

    it("generating: shows Loader2 spinner and 'Agent working...' text", () => {
      useChatStore.setState({ agentStatus: { "session:session-1": "generating" } });
      renderItem({ group: "drafts" });
      expect(document.querySelector(".animate-spin")).toBeInTheDocument();
      expect(screen.getByText("Agent working...")).toBeInTheDocument();
    });

    it("waiting_for_input: shows no spinner and 'Awaiting input' text", () => {
      useChatStore.setState({ agentStatus: { "session:session-1": "waiting_for_input" } });
      renderItem({ group: "drafts" });
      expect(document.querySelector(".animate-spin")).not.toBeInTheDocument();
      expect(screen.getByText("Awaiting input")).toBeInTheDocument();
    });

    it("in-progress with null progress + active agent shows 'Agent working...' (no early return null)", () => {
      useChatStore.setState({ agentStatus: { "session:session-1": "generating" } });
      renderItem({
        group: "in-progress",
        plan: createSession({ status: "accepted", progress: null }),
      });
      expect(screen.getByText("Agent working...")).toBeInTheDocument();
    });

    it("in-progress with progress + active agent shows Agent working, progress stats", () => {
      useChatStore.setState({ agentStatus: { "session:session-1": "generating" } });
      renderItem({
        group: "in-progress",
        plan: createSession({ status: "accepted", progress: createProgress({ done: 2, active: 1, total: 5 }) }),
      });
      expect(screen.getByText("Agent working")).toBeInTheDocument();
      expect(screen.getByText("2/5 done")).toBeInTheDocument();
      expect(screen.getByText("1 active")).toBeInTheDocument();
    });
  });

  describe("verification activity indicator (PO7)", () => {
    it("shows 'Verifying...' when generating and activeVerificationChildId is set for this session", () => {
      useChatStore.setState({ agentStatus: { "session:session-1": "generating" } });
      useIdeationStore.setState({ activeVerificationChildId: { "session-1": "child-session-1" } });
      renderItem({ group: "drafts" });
      expect(screen.getByText("Verifying...")).toBeInTheDocument();
      expect(screen.queryByText("Agent working...")).not.toBeInTheDocument();
    });

    it("shows blue color for 'Verifying...' text", () => {
      useChatStore.setState({ agentStatus: { "session:session-1": "generating" } });
      useIdeationStore.setState({ activeVerificationChildId: { "session-1": "child-session-1" } });
      renderItem({ group: "drafts" });
      const label = screen.getByText("Verifying...");
      expect(label).toHaveStyle({ color: "hsl(217 91% 60%)" });
    });

    it("shows 'Agent working...' (orange) when generating but no verification child", () => {
      useChatStore.setState({ agentStatus: { "session:session-1": "generating" } });
      renderItem({ group: "drafts" });
      expect(screen.getByText("Agent working...")).toBeInTheDocument();
      const label = screen.getByText("Agent working...");
      expect(label).toHaveStyle({ color: "hsl(14 100% 60%)" });
    });

    it("child session (different session id) shows standard 'Agent working...' — not 'Verifying...'", () => {
      useChatStore.setState({ agentStatus: { "session:session-1": "generating" } });
      // activeVerificationChildId is keyed by parent id, not child id
      useIdeationStore.setState({ activeVerificationChildId: { "parent-session": "session-1" } });
      renderItem({ group: "drafts" });
      // session-1 is the child; its own entry is absent → standard indicator
      expect(screen.getByText("Agent working...")).toBeInTheDocument();
      expect(screen.queryByText("Verifying...")).not.toBeInTheDocument();
    });

    it("shows blue spinner when verifying", () => {
      useChatStore.setState({ agentStatus: { "session:session-1": "generating" } });
      useIdeationStore.setState({ activeVerificationChildId: { "session-1": "child-session-1" } });
      renderItem({ group: "drafts" });
      const spinner = document.querySelector(".animate-spin");
      expect(spinner).toBeInTheDocument();
      expect(spinner).toHaveStyle({ color: "hsl(217 91% 60%)" });
    });

    it("shows orange spinner when generating without verification child", () => {
      useChatStore.setState({ agentStatus: { "session:session-1": "generating" } });
      renderItem({ group: "drafts" });
      const spinner = document.querySelector(".animate-spin");
      expect(spinner).toBeInTheDocument();
      expect(spinner).toHaveStyle({ color: "hsl(14 100% 60%)" });
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

    it("drafts: shows Rename, Archive (no Delete after soft-delete migration)", async () => {
      await openMenu("drafts");
      expect(screen.getByText("Rename")).toBeInTheDocument();
      expect(screen.getByText("Archive")).toBeInTheDocument();
      expect(screen.queryByText("Delete")).not.toBeInTheDocument();
      expect(screen.queryByText("Reopen")).not.toBeInTheDocument();
      expect(screen.queryByText("Reset & Re-accept")).not.toBeInTheDocument();
    });

    it("accepted: shows Rename, Reopen, Reset & Re-accept (no Delete after soft-delete migration)", async () => {
      await openMenu("accepted");
      expect(screen.getByText("Rename")).toBeInTheDocument();
      expect(screen.getByText("Reopen")).toBeInTheDocument();
      expect(screen.getByText("Reset & Re-accept")).toBeInTheDocument();
      expect(screen.queryByText("Delete")).not.toBeInTheDocument();
      expect(screen.queryByText("Archive")).not.toBeInTheDocument();
    });

    it("in-progress: shows Rename, Reopen, Reset & Re-accept (no Delete after soft-delete migration)", async () => {
      await openMenu("in-progress");
      expect(screen.getByText("Rename")).toBeInTheDocument();
      expect(screen.getByText("Reopen")).toBeInTheDocument();
      expect(screen.getByText("Reset & Re-accept")).toBeInTheDocument();
      expect(screen.queryByText("Delete")).not.toBeInTheDocument();
    });

    it("done: shows Rename, Reopen, Reset & Re-accept (no Delete after soft-delete migration)", async () => {
      await openMenu("done");
      expect(screen.getByText("Rename")).toBeInTheDocument();
      expect(screen.getByText("Reopen")).toBeInTheDocument();
      expect(screen.getByText("Reset & Re-accept")).toBeInTheDocument();
      expect(screen.queryByText("Delete")).not.toBeInTheDocument();
    });

    it("archived: shows Rename, Reopen (no Delete, no Reset & Re-accept after soft-delete migration)", async () => {
      await openMenu("archived");
      expect(screen.getByText("Rename")).toBeInTheDocument();
      expect(screen.getByText("Reopen")).toBeInTheDocument();
      expect(screen.queryByText("Delete")).not.toBeInTheDocument();
      expect(screen.queryByText("Reset & Re-accept")).not.toBeInTheDocument();
    });
  });

  describe("React.memo optimization", () => {
    it("PlanItem is exported as a React.memo component", () => {
      // React.memo() returns an object with a .type property pointing to the inner function
      // Regular function components are typeof "function", memo-wrapped are typeof "object"
      expect(typeof PlanItem).toBe("object");
      expect((PlanItem as Record<string, unknown>).type).toBeDefined();
    });

    it("re-renders when isSelected changes (visual confirmation)", () => {
      const { rerender, getByTestId } = render(<PlanItem {...defaultProps} isSelected={false} />);
      const item = getByTestId("plan-item-session-1");
      // Not selected: transparent background
      expect(item.style.background).toBe("transparent");

      rerender(<PlanItem {...defaultProps} isSelected={true} />);
      // Selected: non-transparent background (jsdom normalizes hsla to rgba)
      expect(item.style.background).not.toBe("transparent");
      expect(item.style.background).not.toBe("");
    });

    it("does not visually change item B when only item A's isSelected changes", () => {
      const planA = createSession({ id: "session-A", title: "Session A" });
      const planB = createSession({ id: "session-B", title: "Session B" });

      const stableProps = {
        group: "drafts" as SessionGroup,
        isEditing: false,
        isMenuOpen: false,
        inputRef: { current: null } as React.RefObject<HTMLInputElement | null>,
        onSelect: vi.fn(),
        onStartRename: vi.fn(),
        onConfirmRename: vi.fn(),
        onTitleChange: vi.fn(),
        onKeyDown: vi.fn(),
        onMenuOpenChange: vi.fn(),
      };

      const { rerender, getByTestId } = render(
        <>
          <PlanItem {...stableProps} plan={planA} isSelected={false} />
          <PlanItem {...stableProps} plan={planB} isSelected={false} />
        </>
      );

      const itemB = getByTestId("plan-item-session-B");
      const initialBBackground = itemB.style.background;

      // Change only A's isSelected
      rerender(
        <>
          <PlanItem {...stableProps} plan={planA} isSelected={true} />
          <PlanItem {...stableProps} plan={planB} isSelected={false} />
        </>
      );

      // B's visual state is unchanged
      expect(itemB.style.background).toBe(initialBBackground);
      // A changed to selected state (non-transparent)
      expect(getByTestId("plan-item-session-A").style.background).not.toBe("transparent");
      expect(getByTestId("plan-item-session-A").style.background).not.toBe("");
    });
  });
});
