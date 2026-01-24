import { describe, it, expect, beforeEach } from "vitest";
import { renderHook, act } from "@testing-library/react";
import {
  useSupervisorStore,
  useFilteredAlerts,
  useAlertStats,
  useSupervisorAlerts,
} from "./useSupervisorAlerts";
import type { SupervisorAlert as _SupervisorAlert } from "@/types/supervisor";

// Reset store before each test
beforeEach(() => {
  const store = useSupervisorStore.getState();
  store.clearAll();
});

describe("useSupervisorStore", () => {
  describe("addAlert", () => {
    it("adds a new alert with generated id and createdAt", () => {
      const store = useSupervisorStore.getState();

      store.addAlert({
        taskId: "task-123",
        type: "loop_detected",
        severity: "high",
        message: "Loop detected",
      });

      const alerts = useSupervisorStore.getState().alerts;
      expect(alerts).toHaveLength(1);
      expect(alerts[0].id).toBeDefined();
      expect(alerts[0].createdAt).toBeDefined();
      expect(alerts[0].acknowledged).toBe(false);
    });

    it("limits alerts to MAX_ALERTS", () => {
      const store = useSupervisorStore.getState();

      // Add more than MAX_ALERTS (50)
      for (let i = 0; i < 55; i++) {
        store.addAlert({
          taskId: `task-${i}`,
          type: "error",
          severity: "low",
          message: `Error ${i}`,
        });
      }

      const alerts = useSupervisorStore.getState().alerts;
      expect(alerts.length).toBeLessThanOrEqual(50);
    });

    it("adds newest alerts first", () => {
      const store = useSupervisorStore.getState();

      store.addAlert({
        taskId: "task-1",
        type: "error",
        severity: "low",
        message: "First",
      });

      store.addAlert({
        taskId: "task-2",
        type: "error",
        severity: "low",
        message: "Second",
      });

      const alerts = useSupervisorStore.getState().alerts;
      expect(alerts[0].message).toBe("Second");
      expect(alerts[1].message).toBe("First");
    });
  });

  describe("acknowledgeAlert", () => {
    it("acknowledges an alert by id", () => {
      const store = useSupervisorStore.getState();

      store.addAlert({
        taskId: "task-123",
        type: "loop_detected",
        severity: "high",
        message: "Loop detected",
      });

      const alertId = useSupervisorStore.getState().alerts[0].id;
      store.acknowledgeAlert(alertId);

      const alert = useSupervisorStore.getState().alerts[0];
      expect(alert.acknowledged).toBe(true);
      expect(alert.acknowledgedAt).toBeDefined();
    });

    it("does nothing for non-existent id", () => {
      const store = useSupervisorStore.getState();

      store.addAlert({
        taskId: "task-123",
        type: "error",
        severity: "low",
        message: "Test",
      });

      store.acknowledgeAlert("non-existent-id");

      const alert = useSupervisorStore.getState().alerts[0];
      expect(alert.acknowledged).toBe(false);
    });
  });

  describe("acknowledgeAll", () => {
    it("acknowledges all alerts", () => {
      const store = useSupervisorStore.getState();

      store.addAlert({
        taskId: "task-1",
        type: "error",
        severity: "low",
        message: "First",
      });

      store.addAlert({
        taskId: "task-2",
        type: "error",
        severity: "high",
        message: "Second",
      });

      store.acknowledgeAll();

      const alerts = useSupervisorStore.getState().alerts;
      expect(alerts.every((a) => a.acknowledged)).toBe(true);
    });
  });

  describe("dismissAlert", () => {
    it("removes an alert by id", () => {
      const store = useSupervisorStore.getState();

      store.addAlert({
        taskId: "task-123",
        type: "error",
        severity: "low",
        message: "Test",
      });

      const alertId = useSupervisorStore.getState().alerts[0].id;
      store.dismissAlert(alertId);

      expect(useSupervisorStore.getState().alerts).toHaveLength(0);
    });
  });

  describe("dismissAcknowledged", () => {
    it("removes only acknowledged alerts", () => {
      const store = useSupervisorStore.getState();

      store.addAlert({
        taskId: "task-1",
        type: "error",
        severity: "low",
        message: "First",
      });

      store.addAlert({
        taskId: "task-2",
        type: "error",
        severity: "high",
        message: "Second",
      });

      const firstAlertId = useSupervisorStore.getState().alerts[1].id;
      store.acknowledgeAlert(firstAlertId);

      store.dismissAcknowledged();

      const alerts = useSupervisorStore.getState().alerts;
      expect(alerts).toHaveLength(1);
      expect(alerts[0].message).toBe("Second");
    });
  });

  describe("clearAlertsForTask", () => {
    it("removes alerts for a specific task", () => {
      const store = useSupervisorStore.getState();

      store.addAlert({
        taskId: "task-1",
        type: "error",
        severity: "low",
        message: "Task 1 error",
      });

      store.addAlert({
        taskId: "task-2",
        type: "error",
        severity: "low",
        message: "Task 2 error",
      });

      store.clearAlertsForTask("task-1");

      const alerts = useSupervisorStore.getState().alerts;
      expect(alerts).toHaveLength(1);
      expect(alerts[0].taskId).toBe("task-2");
    });
  });

  describe("updateConfig", () => {
    it("updates configuration partially", () => {
      const store = useSupervisorStore.getState();

      store.updateConfig({ loopDetectionThreshold: 5 });

      const config = useSupervisorStore.getState().config;
      expect(config.loopDetectionThreshold).toBe(5);
      expect(config.stuckTimeoutMinutes).toBe(5); // unchanged
    });
  });
});

describe("useFilteredAlerts", () => {
  beforeEach(() => {
    const store = useSupervisorStore.getState();

    store.addAlert({
      taskId: "task-1",
      type: "loop_detected",
      severity: "high",
      message: "Loop 1",
    });

    store.addAlert({
      taskId: "task-2",
      type: "error",
      severity: "low",
      message: "Error",
    });

    store.addAlert({
      taskId: "task-1",
      type: "stuck",
      severity: "critical",
      message: "Stuck",
    });
  });

  it("returns all unacknowledged alerts by default", () => {
    const { result } = renderHook(() => useFilteredAlerts());
    expect(result.current).toHaveLength(3);
  });

  it("filters by severity", () => {
    const { result } = renderHook(() =>
      useFilteredAlerts({ severities: ["high", "critical"] })
    );
    expect(result.current).toHaveLength(2);
  });

  it("filters by type", () => {
    const { result } = renderHook(() =>
      useFilteredAlerts({ types: ["loop_detected"] })
    );
    expect(result.current).toHaveLength(1);
    expect(result.current[0].type).toBe("loop_detected");
  });

  it("filters by taskId", () => {
    const { result } = renderHook(() => useFilteredAlerts({ taskId: "task-1" }));
    expect(result.current).toHaveLength(2);
  });

  it("includes acknowledged when specified", () => {
    const store = useSupervisorStore.getState();
    const alertId = store.alerts[0].id;
    store.acknowledgeAlert(alertId);

    const { result: withoutAck } = renderHook(() => useFilteredAlerts());
    const { result: withAck } = renderHook(() =>
      useFilteredAlerts({ includeAcknowledged: true })
    );

    expect(withoutAck.current.length).toBe(withAck.current.length - 1);
  });
});

describe("useAlertStats", () => {
  it("returns correct statistics", () => {
    const store = useSupervisorStore.getState();

    store.addAlert({
      taskId: "task-1",
      type: "loop_detected",
      severity: "high",
      message: "Loop",
    });

    store.addAlert({
      taskId: "task-2",
      type: "error",
      severity: "critical",
      message: "Error",
    });

    store.addAlert({
      taskId: "task-3",
      type: "stuck",
      severity: "medium",
      message: "Stuck",
    });

    const alertId = useSupervisorStore.getState().alerts[0].id;
    useSupervisorStore.getState().acknowledgeAlert(alertId);

    const { result } = renderHook(() => useAlertStats());

    expect(result.current.total).toBe(3);
    expect(result.current.unacknowledged).toBe(2);
    expect(result.current.high).toBe(1);
    expect(result.current.critical).toBe(1);
    expect(result.current.medium).toBe(1);
    expect(result.current.byType.loop_detected).toBe(1);
    expect(result.current.byType.error).toBe(1);
    expect(result.current.byType.stuck).toBe(1);
  });
});

describe("useSupervisorAlerts", () => {
  it("returns alerts and actions", () => {
    const store = useSupervisorStore.getState();

    store.addAlert({
      taskId: "task-1",
      type: "error",
      severity: "low",
      message: "Test",
    });

    const { result } = renderHook(() =>
      useSupervisorAlerts({ enableListener: false })
    );

    expect(result.current.alerts).toHaveLength(1);
    expect(result.current.stats.total).toBe(1);
    expect(typeof result.current.acknowledge).toBe("function");
    expect(typeof result.current.acknowledgeAll).toBe("function");
    expect(typeof result.current.dismiss).toBe("function");
    expect(typeof result.current.clearAll).toBe("function");
  });

  it("applies filters", () => {
    const store = useSupervisorStore.getState();

    store.addAlert({
      taskId: "task-1",
      type: "error",
      severity: "low",
      message: "Low severity",
    });

    store.addAlert({
      taskId: "task-2",
      type: "error",
      severity: "high",
      message: "High severity",
    });

    const { result } = renderHook(() =>
      useSupervisorAlerts({
        enableListener: false,
        filters: { severities: ["high", "critical"] },
      })
    );

    expect(result.current.alerts).toHaveLength(1);
    expect(result.current.alerts[0].severity).toBe("high");
  });

  it("acknowledge action works", () => {
    const store = useSupervisorStore.getState();

    store.addAlert({
      taskId: "task-1",
      type: "error",
      severity: "low",
      message: "Test",
    });

    const alertId = useSupervisorStore.getState().alerts[0].id;

    const { result } = renderHook(() =>
      useSupervisorAlerts({ enableListener: false })
    );

    act(() => {
      result.current.acknowledge(alertId);
    });

    const alert = useSupervisorStore.getState().alerts[0];
    expect(alert.acknowledged).toBe(true);
  });

  it("dismiss action works", () => {
    const store = useSupervisorStore.getState();

    store.addAlert({
      taskId: "task-1",
      type: "error",
      severity: "low",
      message: "Test",
    });

    const alertId = useSupervisorStore.getState().alerts[0].id;

    const { result } = renderHook(() =>
      useSupervisorAlerts({ enableListener: false })
    );

    act(() => {
      result.current.dismiss(alertId);
    });

    expect(useSupervisorStore.getState().alerts).toHaveLength(0);
  });
});
