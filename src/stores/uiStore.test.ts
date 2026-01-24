import { describe, it, expect, beforeEach } from "vitest";
import { useUiStore } from "./uiStore";

describe("uiStore", () => {
  beforeEach(() => {
    // Reset store to initial state before each test
    useUiStore.setState({
      sidebarOpen: true,
      activeModal: null,
      notifications: [],
    });
  });

  describe("sidebar", () => {
    it("toggles sidebar visibility", () => {
      expect(useUiStore.getState().sidebarOpen).toBe(true);

      useUiStore.getState().toggleSidebar();
      expect(useUiStore.getState().sidebarOpen).toBe(false);

      useUiStore.getState().toggleSidebar();
      expect(useUiStore.getState().sidebarOpen).toBe(true);
    });

    it("sets sidebar visibility directly", () => {
      useUiStore.getState().setSidebarOpen(false);
      expect(useUiStore.getState().sidebarOpen).toBe(false);

      useUiStore.getState().setSidebarOpen(true);
      expect(useUiStore.getState().sidebarOpen).toBe(true);
    });
  });

  describe("modal", () => {
    it("opens a modal with type", () => {
      useUiStore.getState().openModal("task-detail");

      const state = useUiStore.getState();
      expect(state.activeModal).toBe("task-detail");
    });

    it("opens a modal with context", () => {
      useUiStore.getState().openModal("task-detail", { taskId: "task-1" });

      const state = useUiStore.getState();
      expect(state.activeModal).toBe("task-detail");
      expect(state.modalContext).toEqual({ taskId: "task-1" });
    });

    it("closes the modal", () => {
      useUiStore.setState({
        activeModal: "task-detail",
        modalContext: { taskId: "task-1" },
      });

      useUiStore.getState().closeModal();

      const state = useUiStore.getState();
      expect(state.activeModal).toBeNull();
      expect(state.modalContext).toBeUndefined();
    });

    it("replaces modal when opening new one", () => {
      useUiStore.getState().openModal("task-detail");
      useUiStore.getState().openModal("settings");

      const state = useUiStore.getState();
      expect(state.activeModal).toBe("settings");
    });
  });

  describe("notifications", () => {
    it("adds a notification", () => {
      useUiStore.getState().addNotification({
        id: "notif-1",
        type: "success",
        message: "Task completed",
      });

      const state = useUiStore.getState();
      expect(state.notifications).toHaveLength(1);
      expect(state.notifications[0]?.message).toBe("Task completed");
    });

    it("adds multiple notifications", () => {
      useUiStore.getState().addNotification({
        id: "notif-1",
        type: "success",
        message: "First",
      });
      useUiStore.getState().addNotification({
        id: "notif-2",
        type: "error",
        message: "Second",
      });

      const state = useUiStore.getState();
      expect(state.notifications).toHaveLength(2);
    });

    it("removes a notification by id", () => {
      useUiStore.setState({
        notifications: [
          { id: "notif-1", type: "success", message: "First" },
          { id: "notif-2", type: "error", message: "Second" },
        ],
      });

      useUiStore.getState().removeNotification("notif-1");

      const state = useUiStore.getState();
      expect(state.notifications).toHaveLength(1);
      expect(state.notifications[0]?.id).toBe("notif-2");
    });

    it("clears all notifications", () => {
      useUiStore.setState({
        notifications: [
          { id: "notif-1", type: "success", message: "First" },
          { id: "notif-2", type: "error", message: "Second" },
        ],
      });

      useUiStore.getState().clearNotifications();

      const state = useUiStore.getState();
      expect(state.notifications).toHaveLength(0);
    });

    it("does nothing when removing nonexistent notification", () => {
      useUiStore.setState({
        notifications: [{ id: "notif-1", type: "success", message: "First" }],
      });

      useUiStore.getState().removeNotification("nonexistent");

      const state = useUiStore.getState();
      expect(state.notifications).toHaveLength(1);
    });
  });

  describe("loading state", () => {
    it("sets loading state", () => {
      useUiStore.getState().setLoading("tasks", true);

      const state = useUiStore.getState();
      expect(state.loading.tasks).toBe(true);
    });

    it("clears loading state", () => {
      useUiStore.setState({ loading: { tasks: true } });

      useUiStore.getState().setLoading("tasks", false);

      const state = useUiStore.getState();
      expect(state.loading.tasks).toBe(false);
    });

    it("tracks multiple loading states", () => {
      useUiStore.getState().setLoading("tasks", true);
      useUiStore.getState().setLoading("projects", true);

      const state = useUiStore.getState();
      expect(state.loading.tasks).toBe(true);
      expect(state.loading.projects).toBe(true);
    });
  });

  describe("confirmation dialog", () => {
    it("shows confirmation dialog", () => {
      useUiStore.getState().showConfirmation({
        title: "Delete Task",
        message: "Are you sure?",
        onConfirm: () => {},
      });

      const state = useUiStore.getState();
      expect(state.confirmation).toBeDefined();
      expect(state.confirmation?.title).toBe("Delete Task");
    });

    it("hides confirmation dialog", () => {
      useUiStore.setState({
        confirmation: {
          title: "Test",
          message: "Test",
          onConfirm: () => {},
        },
      });

      useUiStore.getState().hideConfirmation();

      const state = useUiStore.getState();
      expect(state.confirmation).toBeNull();
    });
  });
});
