import { describe, it, expect, beforeEach } from "vitest";
import { useActivityStore } from "./activityStore";
import type { AgentMessageEvent, SupervisorAlertEvent } from "@/types/events";

// Helper to create test messages
const createTestMessage = (
  overrides: Partial<AgentMessageEvent> = {}
): AgentMessageEvent => ({
  taskId: "task-1",
  type: "text",
  content: "Test message",
  timestamp: Date.now(),
  ...overrides,
});

// Helper to create test alerts
const createTestAlert = (
  overrides: Partial<SupervisorAlertEvent> = {}
): SupervisorAlertEvent => ({
  taskId: "task-1",
  severity: "medium",
  type: "error",
  message: "Test alert",
  ...overrides,
});

describe("activityStore", () => {
  beforeEach(() => {
    // Reset store to initial state before each test
    useActivityStore.setState({
      messages: [],
      alerts: [],
    });
  });

  describe("messages", () => {
    it("adds a message to the store", () => {
      const message = createTestMessage();

      useActivityStore.getState().addMessage(message);

      const state = useActivityStore.getState();
      expect(state.messages).toHaveLength(1);
      expect(state.messages[0]?.content).toBe("Test message");
    });

    it("adds multiple messages in order", () => {
      useActivityStore.getState().addMessage(createTestMessage({ content: "First" }));
      useActivityStore.getState().addMessage(createTestMessage({ content: "Second" }));
      useActivityStore.getState().addMessage(createTestMessage({ content: "Third" }));

      const state = useActivityStore.getState();
      expect(state.messages).toHaveLength(3);
      expect(state.messages[0]?.content).toBe("First");
      expect(state.messages[2]?.content).toBe("Third");
    });

    it("clears all messages", () => {
      useActivityStore.getState().addMessage(createTestMessage({ content: "First" }));
      useActivityStore.getState().addMessage(createTestMessage({ content: "Second" }));

      useActivityStore.getState().clearMessages();

      const state = useActivityStore.getState();
      expect(state.messages).toHaveLength(0);
    });

    it("clears messages for a specific task", () => {
      useActivityStore.getState().addMessage(createTestMessage({ taskId: "task-1", content: "Task 1" }));
      useActivityStore.getState().addMessage(createTestMessage({ taskId: "task-2", content: "Task 2" }));
      useActivityStore.getState().addMessage(createTestMessage({ taskId: "task-1", content: "Task 1 again" }));

      useActivityStore.getState().clearMessagesForTask("task-1");

      const state = useActivityStore.getState();
      expect(state.messages).toHaveLength(1);
      expect(state.messages[0]?.taskId).toBe("task-2");
    });

    it("respects maximum message limit (ring buffer)", () => {
      // Add more than the limit
      for (let i = 0; i < 150; i++) {
        useActivityStore.getState().addMessage(createTestMessage({ content: `Message ${i}` }));
      }

      const state = useActivityStore.getState();
      // Should have max 100 messages (or whatever the limit is)
      expect(state.messages.length).toBeLessThanOrEqual(100);
      // Should have the most recent messages
      expect(state.messages[state.messages.length - 1]?.content).toBe("Message 149");
    });
  });

  describe("alerts", () => {
    it("adds an alert to the store", () => {
      const alert = createTestAlert();

      useActivityStore.getState().addAlert(alert);

      const state = useActivityStore.getState();
      expect(state.alerts).toHaveLength(1);
      expect(state.alerts[0]?.message).toBe("Test alert");
    });

    it("adds multiple alerts in order", () => {
      useActivityStore.getState().addAlert(createTestAlert({ message: "First" }));
      useActivityStore.getState().addAlert(createTestAlert({ message: "Second" }));

      const state = useActivityStore.getState();
      expect(state.alerts).toHaveLength(2);
      expect(state.alerts[0]?.message).toBe("First");
    });

    it("clears all alerts", () => {
      useActivityStore.getState().addAlert(createTestAlert({ message: "First" }));
      useActivityStore.getState().addAlert(createTestAlert({ message: "Second" }));

      useActivityStore.getState().clearAlerts();

      const state = useActivityStore.getState();
      expect(state.alerts).toHaveLength(0);
    });

    it("clears alerts for a specific task", () => {
      useActivityStore.getState().addAlert(createTestAlert({ taskId: "task-1", message: "Alert 1" }));
      useActivityStore.getState().addAlert(createTestAlert({ taskId: "task-2", message: "Alert 2" }));
      useActivityStore.getState().addAlert(createTestAlert({ taskId: "task-1", message: "Alert 3" }));

      useActivityStore.getState().clearAlertsForTask("task-1");

      const state = useActivityStore.getState();
      expect(state.alerts).toHaveLength(1);
      expect(state.alerts[0]?.taskId).toBe("task-2");
    });

    it("dismisses a specific alert", () => {
      useActivityStore.getState().addAlert(createTestAlert({ taskId: "task-1", message: "Alert 1" }));
      useActivityStore.getState().addAlert(createTestAlert({ taskId: "task-2", message: "Alert 2" }));

      // Get the first alert's index position
      const state = useActivityStore.getState();
      const firstAlert = state.alerts[0];

      useActivityStore.getState().dismissAlert(0);

      const newState = useActivityStore.getState();
      expect(newState.alerts).toHaveLength(1);
      expect(newState.alerts[0]?.message).toBe("Alert 2");
    });
  });

  describe("clear all", () => {
    it("clears both messages and alerts", () => {
      useActivityStore.getState().addMessage(createTestMessage());
      useActivityStore.getState().addAlert(createTestAlert());

      useActivityStore.getState().clearAll();

      const state = useActivityStore.getState();
      expect(state.messages).toHaveLength(0);
      expect(state.alerts).toHaveLength(0);
    });
  });

  describe("selectors", () => {
    it("getMessagesForTask returns filtered messages", () => {
      useActivityStore.getState().addMessage(createTestMessage({ taskId: "task-1" }));
      useActivityStore.getState().addMessage(createTestMessage({ taskId: "task-2" }));
      useActivityStore.getState().addMessage(createTestMessage({ taskId: "task-1" }));

      const messages = useActivityStore.getState().getMessagesForTask("task-1");
      expect(messages).toHaveLength(2);
    });

    it("getAlertsForTask returns filtered alerts", () => {
      useActivityStore.getState().addAlert(createTestAlert({ taskId: "task-1" }));
      useActivityStore.getState().addAlert(createTestAlert({ taskId: "task-2" }));

      const alerts = useActivityStore.getState().getAlertsForTask("task-1");
      expect(alerts).toHaveLength(1);
    });

    it("getAlertsBySeverity returns filtered alerts", () => {
      useActivityStore.getState().addAlert(createTestAlert({ severity: "low" }));
      useActivityStore.getState().addAlert(createTestAlert({ severity: "high" }));
      useActivityStore.getState().addAlert(createTestAlert({ severity: "high" }));

      const highAlerts = useActivityStore.getState().getAlertsBySeverity("high");
      expect(highAlerts).toHaveLength(2);
    });

    it("hasUnreadAlerts returns true when critical/high alerts exist", () => {
      expect(useActivityStore.getState().hasUnreadAlerts()).toBe(false);

      useActivityStore.getState().addAlert(createTestAlert({ severity: "low" }));
      expect(useActivityStore.getState().hasUnreadAlerts()).toBe(false);

      useActivityStore.getState().addAlert(createTestAlert({ severity: "high" }));
      expect(useActivityStore.getState().hasUnreadAlerts()).toBe(true);
    });
  });
});
