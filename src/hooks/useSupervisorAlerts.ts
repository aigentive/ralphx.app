/**
 * Supervisor alerts hook - Extended version with filtering and acknowledgement
 *
 * Provides hooks for listening to supervisor events and managing alerts
 * with filtering, acknowledgement, and computed state.
 */

import { useEffect, useMemo, useCallback, useState } from "react";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import { create } from "zustand";
import { immer } from "zustand/middleware/immer";
import {
  SupervisorEventSchema,
  SupervisorAlertSchema,
  type SupervisorEvent,
  type SupervisorAlert,
  type Severity,
  type AlertType,
  type SupervisorConfig,
} from "@/types/supervisor";

// ============================================================================
// Constants
// ============================================================================

/** Maximum number of alerts to keep */
const MAX_ALERTS = 50;

/** Default supervisor configuration */
const DEFAULT_CONFIG: SupervisorConfig = {
  loopDetectionThreshold: 3,
  stuckTimeoutMinutes: 5,
  tokenWarningThreshold: 50000,
  timeWarningMinutes: 10,
  errorRepeatThreshold: 3,
};

// ============================================================================
// Supervisor Store
// ============================================================================

interface SupervisorState {
  /** All supervisor alerts */
  alerts: SupervisorAlert[];
  /** Supervisor configuration */
  config: SupervisorConfig;
  /** Connected to event stream */
  isConnected: boolean;
}

interface SupervisorActions {
  /** Add a new alert */
  addAlert: (alert: Omit<SupervisorAlert, "id" | "acknowledged" | "createdAt">) => void;
  /** Acknowledge an alert by ID */
  acknowledgeAlert: (id: string) => void;
  /** Acknowledge all alerts */
  acknowledgeAll: () => void;
  /** Dismiss (remove) an alert by ID */
  dismissAlert: (id: string) => void;
  /** Dismiss all acknowledged alerts */
  dismissAcknowledged: () => void;
  /** Clear all alerts */
  clearAll: () => void;
  /** Clear alerts for a specific task */
  clearAlertsForTask: (taskId: string) => void;
  /** Update configuration */
  updateConfig: (config: Partial<SupervisorConfig>) => void;
  /** Set connection status */
  setConnected: (connected: boolean) => void;
}

export const useSupervisorStore = create<SupervisorState & SupervisorActions>()(
  immer((set) => ({
    // Initial state
    alerts: [],
    config: DEFAULT_CONFIG,
    isConnected: false,

    // Actions
    addAlert: (alertData) =>
      set((state) => {
        const alert: SupervisorAlert = {
          id: crypto.randomUUID(),
          ...alertData,
          acknowledged: false,
          createdAt: new Date().toISOString(),
        };
        state.alerts.unshift(alert); // Newest first
        // Limit alerts
        if (state.alerts.length > MAX_ALERTS) {
          state.alerts = state.alerts.slice(0, MAX_ALERTS);
        }
      }),

    acknowledgeAlert: (id) =>
      set((state) => {
        const alert = state.alerts.find((a) => a.id === id);
        if (alert) {
          alert.acknowledged = true;
          alert.acknowledgedAt = new Date().toISOString();
        }
      }),

    acknowledgeAll: () =>
      set((state) => {
        const now = new Date().toISOString();
        state.alerts.forEach((alert) => {
          if (!alert.acknowledged) {
            alert.acknowledged = true;
            alert.acknowledgedAt = now;
          }
        });
      }),

    dismissAlert: (id) =>
      set((state) => {
        state.alerts = state.alerts.filter((a) => a.id !== id);
      }),

    dismissAcknowledged: () =>
      set((state) => {
        state.alerts = state.alerts.filter((a) => !a.acknowledged);
      }),

    clearAll: () =>
      set((state) => {
        state.alerts = [];
      }),

    clearAlertsForTask: (taskId) =>
      set((state) => {
        state.alerts = state.alerts.filter((a) => a.taskId !== taskId);
      }),

    updateConfig: (config) =>
      set((state) => {
        state.config = { ...state.config, ...config };
      }),

    setConnected: (connected) =>
      set((state) => {
        state.isConnected = connected;
      }),
  }))
);

// ============================================================================
// Filtering Hook
// ============================================================================

interface SupervisorAlertFilters {
  /** Filter by severity levels */
  severities?: Severity[];
  /** Filter by alert types */
  types?: AlertType[];
  /** Filter by task ID */
  taskId?: string;
  /** Include acknowledged alerts */
  includeAcknowledged?: boolean;
}

/**
 * Hook to get filtered supervisor alerts
 *
 * @param filters - Optional filters to apply
 * @returns Filtered alerts and helper functions
 */
export function useFilteredAlerts(filters: SupervisorAlertFilters = {}) {
  const alerts = useSupervisorStore((s) => s.alerts);

  const filteredAlerts = useMemo(() => {
    let result = alerts;

    // Filter by severity
    if (filters.severities && filters.severities.length > 0) {
      result = result.filter((a) => filters.severities!.includes(a.severity));
    }

    // Filter by type
    if (filters.types && filters.types.length > 0) {
      result = result.filter((a) => filters.types!.includes(a.type));
    }

    // Filter by task ID
    if (filters.taskId) {
      result = result.filter((a) => a.taskId === filters.taskId);
    }

    // Filter out acknowledged unless explicitly included
    if (!filters.includeAcknowledged) {
      result = result.filter((a) => !a.acknowledged);
    }

    return result;
  }, [alerts, filters.severities, filters.types, filters.taskId, filters.includeAcknowledged]);

  return filteredAlerts;
}

// ============================================================================
// Alert Stats Hook
// ============================================================================

interface AlertStats {
  total: number;
  unacknowledged: number;
  critical: number;
  high: number;
  medium: number;
  low: number;
  byType: Record<AlertType, number>;
}

/**
 * Hook to get alert statistics
 */
export function useAlertStats(): AlertStats {
  const alerts = useSupervisorStore((s) => s.alerts);

  return useMemo(() => {
    const stats: AlertStats = {
      total: alerts.length,
      unacknowledged: alerts.filter((a) => !a.acknowledged).length,
      critical: alerts.filter((a) => a.severity === "critical").length,
      high: alerts.filter((a) => a.severity === "high").length,
      medium: alerts.filter((a) => a.severity === "medium").length,
      low: alerts.filter((a) => a.severity === "low").length,
      byType: {
        loop_detected: 0,
        stuck: 0,
        error: 0,
        escalation: 0,
        token_warning: 0,
        time_warning: 0,
      },
    };

    alerts.forEach((alert) => {
      stats.byType[alert.type]++;
    });

    return stats;
  }, [alerts]);
}

// ============================================================================
// Event Listener Hook
// ============================================================================

/**
 * Hook to listen for supervisor events from the backend
 *
 * Listens to both 'supervisor:event' (raw events) and 'supervisor:alert' (alerts)
 * channels and updates the supervisor store accordingly.
 *
 * @param options - Optional configuration
 */
export function useSupervisorEventListener(options: { enabled?: boolean } = {}) {
  const { enabled = true } = options;
  const addAlert = useSupervisorStore((s) => s.addAlert);
  const setConnected = useSupervisorStore((s) => s.setConnected);

  useEffect(() => {
    if (!enabled) return;

    const unlisteners: Promise<UnlistenFn>[] = [];

    // Listen for supervisor alerts (pre-processed by backend)
    unlisteners.push(
      listen<unknown>("supervisor:alert", (event) => {
        const parsed = SupervisorAlertSchema.omit({
          id: true,
          acknowledged: true,
          createdAt: true,
          acknowledgedAt: true,
        }).safeParse(event.payload);

        if (parsed.success) {
          addAlert(parsed.data);
        } else {
          console.error("Invalid supervisor alert:", parsed.error.message);
        }
      })
    );

    // Listen for raw supervisor events (for custom processing)
    unlisteners.push(
      listen<unknown>("supervisor:event", (event) => {
        const parsed = SupervisorEventSchema.safeParse(event.payload);

        if (!parsed.success) {
          console.error("Invalid supervisor event:", parsed.error.message);
          return;
        }

        // Convert certain events to alerts
        const supervisorEvent = parsed.data;

        switch (supervisorEvent.type) {
          case "error":
            addAlert({
              taskId: supervisorEvent.taskId,
              type: "error",
              severity: supervisorEvent.info.recoverable ? "medium" : "high",
              message: supervisorEvent.info.message,
              details: `Source: ${supervisorEvent.info.source}`,
            });
            break;

          case "token_threshold":
            addAlert({
              taskId: supervisorEvent.taskId,
              type: "token_warning",
              severity: "medium",
              message: `Token usage ${supervisorEvent.tokensUsed} exceeds threshold ${supervisorEvent.threshold}`,
              suggestedAction: "pause",
            });
            break;

          case "time_threshold":
            addAlert({
              taskId: supervisorEvent.taskId,
              type: "time_warning",
              severity: "medium",
              message: `Execution time ${supervisorEvent.elapsedMinutes}min exceeds threshold ${supervisorEvent.thresholdMinutes}min`,
              suggestedAction: "pause",
            });
            break;
        }
      })
    );

    setConnected(true);

    return () => {
      setConnected(false);
      unlisteners.forEach((unlisten) => unlisten.then((fn) => fn()));
    };
  }, [enabled, addAlert, setConnected]);
}

// ============================================================================
// Combined Hook
// ============================================================================

/**
 * Main hook for supervisor alerts functionality
 *
 * Combines event listening, filtering, and actions into a single hook.
 *
 * @param options - Hook options
 * @returns Alerts state and actions
 */
export function useSupervisorAlerts(
  options: {
    /** Enable event listening */
    enableListener?: boolean;
    /** Filters to apply */
    filters?: SupervisorAlertFilters;
  } = {}
) {
  const { enableListener = true, filters = {} } = options;

  // Set up event listener
  useSupervisorEventListener({ enabled: enableListener });

  // Get store actions
  const acknowledgeAlert = useSupervisorStore((s) => s.acknowledgeAlert);
  const acknowledgeAll = useSupervisorStore((s) => s.acknowledgeAll);
  const dismissAlert = useSupervisorStore((s) => s.dismissAlert);
  const dismissAcknowledged = useSupervisorStore((s) => s.dismissAcknowledged);
  const clearAll = useSupervisorStore((s) => s.clearAll);
  const isConnected = useSupervisorStore((s) => s.isConnected);

  // Get filtered alerts
  const alerts = useFilteredAlerts(filters);

  // Get stats
  const stats = useAlertStats();

  // Memoized callbacks
  const acknowledge = useCallback(
    (id: string) => acknowledgeAlert(id),
    [acknowledgeAlert]
  );

  const dismiss = useCallback(
    (id: string) => dismissAlert(id),
    [dismissAlert]
  );

  return {
    /** Filtered alerts */
    alerts,
    /** Alert statistics */
    stats,
    /** Whether connected to event stream */
    isConnected,
    /** Acknowledge a single alert */
    acknowledge,
    /** Acknowledge all alerts */
    acknowledgeAll,
    /** Dismiss a single alert */
    dismiss,
    /** Dismiss all acknowledged alerts */
    dismissAcknowledged,
    /** Clear all alerts */
    clearAll,
  };
}
