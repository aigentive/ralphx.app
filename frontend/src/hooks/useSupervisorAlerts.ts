/**
 * Supervisor alerts hook - Extended version with filtering and acknowledgement
 *
 * Provides hooks for listening to supervisor events and managing alerts
 * with filtering, acknowledgement, and computed state.
 */

import { useMemo, useCallback } from "react";
import type { Severity, AlertType } from "@/types/supervisor";
import { useSupervisorStore } from "./useSupervisorAlerts.store";
import { useSupervisorEventListener } from "./useSupervisorAlerts.listener";

// Re-export for convenience
export { useSupervisorStore } from "./useSupervisorAlerts.store";
export { useSupervisorEventListener } from "./useSupervisorAlerts.listener";

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
